use super::client::{
    LLMClient, LLMRequest, LLMResponse, LlmModelLister, ModelInfo, ReasoningLevel,
};
use super::endpoint_url::complete_standard_endpoint;
use crate::error::AppError;
use crate::infrastructure::http::HttpClient;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

pub struct OpenAILLMClient {
    http_client: Arc<dyn HttpClient>,
    endpoint: String,
    model: String,
    api_key: String,
    api_mode: OpenAIAPIMode,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OpenAIEndpointKind {
    ChatCompletions,
    Responses,
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum OpenAIAPIMode {
    Auto,
    ChatCompletions,
    Responses,
}

pub(crate) fn openai_compatible_chat_completions_url(endpoint: &str) -> String {
    complete_standard_endpoint(
        endpoint,
        "/v1/chat/completions",
        &["/v1/chat/completions", "/v1/chat", "/v1"],
    )
}

pub(crate) fn openai_compatible_responses_url(endpoint: &str) -> String {
    complete_standard_endpoint(endpoint, "/v1/responses", &["/v1/responses", "/v1"])
}

pub(crate) fn openai_compatible_models_url(endpoint: &str) -> String {
    complete_standard_endpoint(
        endpoint,
        "/v1/models",
        &[
            "/v1/chat/completions",
            "/v1/chat",
            "/v1/responses",
            "/v1/models",
            "/v1",
        ],
    )
}

fn openai_compatible_v1_base_url(endpoint: &str) -> String {
    complete_standard_endpoint(
        endpoint,
        "/v1",
        &[
            "/v1/chat/completions",
            "/v1/chat",
            "/v1/responses",
            "/v1/models",
            "/v1",
        ],
    )
}

fn is_official_openai_endpoint(endpoint: &str) -> bool {
    openai_compatible_v1_base_url(endpoint.trim_end_matches('/')) == "https://api.openai.com/v1"
}

impl OpenAILLMClient {
    #[allow(dead_code)]
    pub fn new(
        http_client: Arc<dyn HttpClient>,
        endpoint: String,
        model: String,
        api_key: String,
    ) -> Self {
        Self::with_mode(http_client, endpoint, model, api_key, OpenAIAPIMode::Auto)
    }

    pub fn new_chat_completions(
        http_client: Arc<dyn HttpClient>,
        endpoint: String,
        model: String,
        api_key: String,
    ) -> Self {
        Self::with_mode(
            http_client,
            endpoint,
            model,
            api_key,
            OpenAIAPIMode::ChatCompletions,
        )
    }

    pub fn new_responses(
        http_client: Arc<dyn HttpClient>,
        endpoint: String,
        model: String,
        api_key: String,
    ) -> Self {
        Self::with_mode(
            http_client,
            endpoint,
            model,
            api_key,
            OpenAIAPIMode::Responses,
        )
    }

    fn with_mode(
        http_client: Arc<dyn HttpClient>,
        endpoint: String,
        model: String,
        api_key: String,
        api_mode: OpenAIAPIMode,
    ) -> Self {
        Self {
            http_client,
            endpoint,
            model,
            api_key,
            api_mode,
        }
    }

    /// 判断是否是推理模型（参考 Pi: 按 model name 检测）
    fn is_reasoning_model(&self) -> bool {
        self.model.starts_with("o1") || self.model.starts_with("o3")
    }

    fn uses_responses_api(&self) -> bool {
        match self.api_mode {
            OpenAIAPIMode::ChatCompletions => false,
            OpenAIAPIMode::Responses => true,
            OpenAIAPIMode::Auto => match self.explicit_endpoint_kind() {
                Some(OpenAIEndpointKind::ChatCompletions) => false,
                Some(OpenAIEndpointKind::Responses) => true,
                None => {
                    self.model.starts_with("gpt-5") && is_official_openai_endpoint(&self.endpoint)
                }
            },
        }
    }

    fn endpoint_base(&self) -> &str {
        self.endpoint.trim_end_matches('/')
    }

    fn explicit_endpoint_kind(&self) -> Option<OpenAIEndpointKind> {
        let endpoint = self.endpoint_base();

        if endpoint.ends_with("/v1/chat/completions") || endpoint.ends_with("/v1/chat") {
            Some(OpenAIEndpointKind::ChatCompletions)
        } else if endpoint.ends_with("/v1/responses") {
            Some(OpenAIEndpointKind::Responses)
        } else {
            None
        }
    }

    fn chat_completions_url(&self) -> String {
        openai_compatible_chat_completions_url(&self.endpoint)
    }

    fn responses_url(&self) -> String {
        openai_compatible_responses_url(&self.endpoint)
    }

    fn reasoning_effort(level: ReasoningLevel) -> &'static str {
        match level {
            ReasoningLevel::Minimal => "minimal",
            ReasoningLevel::Low => "low",
            ReasoningLevel::Medium => "medium",
            ReasoningLevel::High => "high",
            ReasoningLevel::XHigh => "xhigh",
        }
    }

    /// 应用 reasoning 参数（provider 内部逻辑）
    fn apply_reasoning(&self, body: &mut serde_json::Value, level: ReasoningLevel) {
        if !self.is_reasoning_model() {
            return; // 静默忽略（参考 Pi: 非推理模型不报错）
        }

        body["reasoning_effort"] = json!(match level {
            ReasoningLevel::Minimal | ReasoningLevel::Low => "low",
            ReasoningLevel::Medium => "medium",
            ReasoningLevel::High | ReasoningLevel::XHigh => "high",
        });
    }
}

#[async_trait]
impl LLMClient for OpenAILLMClient {
    async fn generate(&self, request: &LLMRequest) -> Result<LLMResponse> {
        if self.uses_responses_api() {
            return self.generate_responses(request).await;
        }

        let url = self.chat_completions_url();

        // 构造 messages
        let mut messages = Vec::new();
        if let Some(system) = &request.system_prompt {
            messages.push(json!({"role": "system", "content": system}));
        }
        messages.push(json!({"role": "user", "content": &request.user_prompt}));

        let mut body = json!({
            "model": self.model,
            "messages": messages,
        });

        // 参考 Pi: 按能力条件应用参数
        if let Some(temp) = request.options.temperature {
            if !self.is_reasoning_model() {
                // o1/o3 不支持 temperature
                body["temperature"] = json!(temp);
            }
        }

        if let Some(max_tokens) = request.options.max_tokens {
            body["max_tokens"] = json!(max_tokens);
        }

        if let Some(level) = request.options.reasoning {
            self.apply_reasoning(&mut body, level);
        }

        // 发送请求
        let mut headers = HashMap::new();
        headers.insert(
            "Authorization".to_string(),
            format!("Bearer {}", self.api_key),
        );
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        let response = self
            .http_client
            .post(&url, headers, body.to_string())
            .await
            .map_err(|e| AppError::Network(e.to_string()))?;

        // 错误处理
        if response.status == 401 {
            return Err(AppError::Unauthorized("Invalid API key".into()).into());
        } else if response.status == 429 {
            return Err(AppError::RateLimited("Rate limit exceeded".into()).into());
        } else if response.status != 200 {
            return Err(AppError::UpstreamStatus(response.status, response.body).into());
        }

        // 解析响应
        let json: serde_json::Value = serde_json::from_str(&response.body)
            .map_err(|e| AppError::InvalidResponse(format!("JSON parse: {}", e)))?;

        let text = json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| AppError::InvalidResponse("Missing content".into()))?
            .to_string();

        Ok(LLMResponse { text })
    }
}

#[async_trait]
impl LlmModelLister for OpenAILLMClient {
    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        let url = openai_compatible_models_url(&self.endpoint);
        let mut headers = HashMap::new();
        headers.insert(
            "Authorization".to_string(),
            format!("Bearer {}", self.api_key),
        );
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        let response = self
            .http_client
            .get(&url, headers)
            .await
            .map_err(|e| AppError::Network(e.to_string()))?;

        ensure_success_status(response.status, &response.body)?;
        parse_models_response(&response.body)
    }
}

/// Maps a non-2xx model-list response to a user-facing string error.
/// Shared shape with the Anthropic lister; kept here since OpenAI-compatible
/// listing is the common case.
fn ensure_success_status(status: u16, body: &str) -> Result<()> {
    match status {
        200 => Ok(()),
        401 | 403 => {
            Err(AppError::Unauthorized("Invalid API key or insufficient permission".into()).into())
        }
        404 => Err(AppError::InvalidResponse("API endpoint not found".into()).into()),
        429 => Err(AppError::RateLimited("Rate limit exceeded".into()).into()),
        _ => Err(AppError::UpstreamStatus(status, body.to_string()).into()),
    }
}

fn parse_models_response(body: &str) -> Result<Vec<ModelInfo>> {
    let json: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| AppError::InvalidResponse(format!("Model list JSON parse failed: {}", e)))?;
    let data = json["data"].as_array().ok_or_else(|| {
        AppError::InvalidResponse("Model list response is missing data array".into())
    })?;

    let models: Vec<_> = data
        .iter()
        .filter_map(|item| {
            item["id"]
                .as_str()
                .or_else(|| item.as_str())
                .map(|id| ModelInfo { id: id.to_string() })
        })
        .collect();

    if models.is_empty() {
        Err(
            AppError::InvalidResponse("Model list response did not contain model ids".into())
                .into(),
        )
    } else {
        Ok(models)
    }
}

impl OpenAILLMClient {
    async fn generate_responses(&self, request: &LLMRequest) -> Result<LLMResponse> {
        let url = self.responses_url();

        let mut body = json!({
            "model": self.model,
            "input": request.user_prompt,
        });

        if let Some(system) = &request.system_prompt {
            body["instructions"] = json!(system);
        }

        if let Some(max_tokens) = request.options.max_tokens {
            body["max_output_tokens"] = json!(max_tokens);
        }

        if let Some(level) = request.options.reasoning {
            body["reasoning"] = json!({
                "effort": Self::reasoning_effort(level),
            });
        }

        let mut headers = HashMap::new();
        headers.insert(
            "Authorization".to_string(),
            format!("Bearer {}", self.api_key),
        );
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        let response = self
            .http_client
            .post(&url, headers, body.to_string())
            .await
            .map_err(|e| AppError::Network(e.to_string()))?;

        if response.status == 401 {
            return Err(AppError::Unauthorized("Invalid API key".into()).into());
        } else if response.status == 429 {
            return Err(AppError::RateLimited("Rate limit exceeded".into()).into());
        } else if response.status != 200 {
            return Err(AppError::UpstreamStatus(response.status, response.body).into());
        }

        let json: serde_json::Value = serde_json::from_str(&response.body)
            .map_err(|e| AppError::InvalidResponse(format!("JSON parse: {}", e)))?;

        if let Some(text) = json["output_text"].as_str() {
            return Ok(LLMResponse {
                text: text.to_string(),
            });
        }

        let output = json["output"]
            .as_array()
            .ok_or_else(|| AppError::InvalidResponse("Missing output array".into()))?;

        let mut text = String::new();
        for item in output {
            let Some(content) = item["content"].as_array() else {
                continue;
            };

            for block in content {
                if block["type"] == "output_text" {
                    if let Some(block_text) = block["text"].as_str() {
                        text.push_str(block_text);
                    }
                }
            }
        }

        if text.is_empty() {
            return Err(AppError::InvalidResponse("Missing output text".into()).into());
        }

        Ok(LLMResponse { text })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::http::{HttpClient, HttpResponse};
    use crate::infrastructure::llm::{LLMOptions, LLMRequest};
    use async_trait::async_trait;
    use std::collections::HashMap;

    struct MockHttpClient {
        response: HttpResponse,
    }

    struct RecordingHttpClient {
        response: HttpResponse,
        posts: Arc<std::sync::Mutex<Vec<(String, String)>>>,
    }

    #[async_trait]
    impl HttpClient for MockHttpClient {
        async fn post(
            &self,
            _url: &str,
            _headers: HashMap<String, String>,
            _body: String,
        ) -> Result<HttpResponse> {
            Ok(self.response.clone())
        }

        async fn get(&self, _url: &str, _headers: HashMap<String, String>) -> Result<HttpResponse> {
            unimplemented!()
        }
    }

    #[async_trait]
    impl HttpClient for RecordingHttpClient {
        async fn post(
            &self,
            url: &str,
            _headers: HashMap<String, String>,
            body: String,
        ) -> Result<HttpResponse> {
            self.posts.lock().unwrap().push((url.to_string(), body));
            Ok(self.response.clone())
        }

        async fn get(&self, _url: &str, _headers: HashMap<String, String>) -> Result<HttpResponse> {
            unimplemented!()
        }
    }

    struct ListModelsMockHttpClient {
        status: u16,
        body: String,
    }

    #[async_trait]
    impl HttpClient for ListModelsMockHttpClient {
        async fn post(
            &self,
            _url: &str,
            _headers: HashMap<String, String>,
            _body: String,
        ) -> Result<HttpResponse> {
            unimplemented!()
        }

        async fn get(&self, _url: &str, _headers: HashMap<String, String>) -> Result<HttpResponse> {
            Ok(HttpResponse {
                status: self.status,
                body: self.body.clone(),
                headers: HashMap::new(),
            })
        }
    }

    #[tokio::test]
    async fn list_models_parses_openai_compatible_data_array() {
        let mock = Arc::new(ListModelsMockHttpClient {
            status: 200,
            body: r#"{"data":[{"id":"DeepSeek-V4-Pro"},{"id":"GLM-5.1"}]}"#.to_string(),
        });
        let client = OpenAILLMClient::new(
            mock,
            "https://api.openai.com".to_string(),
            "unused".to_string(),
            "key".to_string(),
        );

        let models = client.list_models().await.unwrap();

        let ids: Vec<_> = models.into_iter().map(|m| m.id).collect();
        assert_eq!(ids, vec!["DeepSeek-V4-Pro", "GLM-5.1"]);
    }

    #[tokio::test]
    async fn list_models_rejects_response_without_data_array() {
        let mock = Arc::new(ListModelsMockHttpClient {
            status: 200,
            body: r#"{"object":"list"}"#.to_string(),
        });
        let client = OpenAILLMClient::new(
            mock,
            "https://api.openai.com".to_string(),
            "unused".to_string(),
            "key".to_string(),
        );

        let error = client.list_models().await.unwrap_err();
        assert_eq!(
            error.to_string(),
            "Invalid response: Model list response is missing data array"
        );
    }

    #[tokio::test]
    async fn test_openai_normal_response() {
        let mock = Arc::new(MockHttpClient {
            response: HttpResponse {
                status: 200,
                body: r#"{"choices":[{"message":{"content":"Hello"}}]}"#.to_string(),
                headers: HashMap::new(),
            },
        });

        let client = OpenAILLMClient::new(
            mock,
            "https://api.openai.com".to_string(),
            "gpt-4".to_string(),
            "test-key".to_string(),
        );

        let request = LLMRequest {
            system_prompt: Some("You are a translator".to_string()),
            user_prompt: "Hello".to_string(),
            options: LLMOptions::default(),
        };

        let response = client.generate(&request).await.unwrap();
        assert_eq!(response.text, "Hello");
    }

    #[tokio::test]
    async fn test_openai_unauthorized() {
        let mock = Arc::new(MockHttpClient {
            response: HttpResponse {
                status: 401,
                body: "Unauthorized".to_string(),
                headers: HashMap::new(),
            },
        });

        let client = OpenAILLMClient::new(
            mock,
            "https://api.openai.com".to_string(),
            "gpt-4".to_string(),
            "bad-key".to_string(),
        );

        let request = LLMRequest {
            system_prompt: None,
            user_prompt: "test".to_string(),
            options: LLMOptions::default(),
        };

        let result = client.generate(&request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_o1_model_no_temperature() {
        // o1 模型不应该传 temperature
        // 这个测试验证静默忽略行为
        let client = OpenAILLMClient::new(
            Arc::new(MockHttpClient {
                response: HttpResponse {
                    status: 200,
                    body: r#"{"choices":[{"message":{"content":"test"}}]}"#.to_string(),
                    headers: HashMap::new(),
                },
            }),
            "https://api.openai.com".to_string(),
            "o1-preview".to_string(),
            "test-key".to_string(),
        );

        assert!(client.is_reasoning_model());
    }

    #[tokio::test]
    async fn test_gpt5_models_use_responses_api() {
        let posts = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mock = Arc::new(RecordingHttpClient {
            response: HttpResponse {
                status: 200,
                body: r#"{"output":[{"type":"message","content":[{"type":"output_text","text":"Bonjour"}]}]}"#.to_string(),
                headers: HashMap::new(),
            },
            posts: posts.clone(),
        });

        let client = OpenAILLMClient::new(
            mock,
            "https://api.openai.com".to_string(),
            "gpt-5-mini".to_string(),
            "test-key".to_string(),
        );

        let request = LLMRequest {
            system_prompt: Some("You are a translation engine".to_string()),
            user_prompt: "Hello".to_string(),
            options: LLMOptions {
                reasoning: Some(ReasoningLevel::Low),
                temperature: Some(0.2),
                max_tokens: Some(1024),
            },
        };

        let response = client.generate(&request).await.unwrap();

        let posts = posts.lock().unwrap();
        assert_eq!(posts[0].0, "https://api.openai.com/v1/responses");

        let body: serde_json::Value = serde_json::from_str(&posts[0].1).unwrap();
        assert_eq!(body["model"], "gpt-5-mini");
        assert_eq!(body["instructions"], "You are a translation engine");
        assert_eq!(body["input"], "Hello");
        assert_eq!(body["reasoning"]["effort"], "low");
        assert_eq!(body["max_output_tokens"], 1024);
        assert!(body.get("temperature").is_none());
        assert_eq!(response.text, "Bonjour");
    }

    #[tokio::test]
    async fn test_explicit_chat_completions_endpoint_is_used_as_is_for_gpt5_models() {
        let posts = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mock = Arc::new(RecordingHttpClient {
            response: HttpResponse {
                status: 200,
                body: r#"{"choices":[{"message":{"content":"Bonjour"}}]}"#.to_string(),
                headers: HashMap::new(),
            },
            posts: posts.clone(),
        });

        let client = OpenAILLMClient::new(
            mock,
            "https://llm.example.test/v1/chat/completions".to_string(),
            "gpt-5-mini".to_string(),
            "test-key".to_string(),
        );

        let request = LLMRequest {
            system_prompt: Some("You are a translation engine".to_string()),
            user_prompt: "Hello".to_string(),
            options: LLMOptions {
                reasoning: None,
                temperature: Some(0.2),
                max_tokens: Some(1024),
            },
        };

        let response = client.generate(&request).await.unwrap();

        let posts = posts.lock().unwrap();
        assert_eq!(posts[0].0, "https://llm.example.test/v1/chat/completions");
        assert_eq!(response.text, "Bonjour");
    }

    #[tokio::test]
    async fn test_custom_base_endpoint_uses_chat_completions_for_gpt5_named_models() {
        let posts = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mock = Arc::new(RecordingHttpClient {
            response: HttpResponse {
                status: 200,
                body: r#"{"choices":[{"message":{"content":"Bonjour"}}]}"#.to_string(),
                headers: HashMap::new(),
            },
            posts: posts.clone(),
        });

        let client = OpenAILLMClient::new(
            mock,
            "https://llm.example.test".to_string(),
            "gpt-5-mini".to_string(),
            "test-key".to_string(),
        );

        let request = LLMRequest {
            system_prompt: Some("You are a translation engine".to_string()),
            user_prompt: "Hello".to_string(),
            options: LLMOptions {
                reasoning: None,
                temperature: Some(0.2),
                max_tokens: Some(1024),
            },
        };

        let response = client.generate(&request).await.unwrap();

        let posts = posts.lock().unwrap();
        assert_eq!(posts[0].0, "https://llm.example.test/v1/chat/completions");
        assert_eq!(response.text, "Bonjour");
    }

    #[test]
    fn test_openai_compatible_url_helpers_normalize_base_and_explicit_endpoints() {
        assert_eq!(
            openai_compatible_chat_completions_url("https://llm.example.test"),
            "https://llm.example.test/v1/chat/completions"
        );
        assert_eq!(
            openai_compatible_chat_completions_url("https://llm.example.test/v1"),
            "https://llm.example.test/v1/chat/completions"
        );
        assert_eq!(
            openai_compatible_chat_completions_url("https://llm.example.test/v1/chat/completions"),
            "https://llm.example.test/v1/chat/completions"
        );
        assert_eq!(
            openai_compatible_chat_completions_url("https://llm.example.test/v1/chat/"),
            "https://llm.example.test/v1/chat/completions"
        );
        assert_eq!(
            openai_compatible_chat_completions_url("https://llm.example.test/chat/completions"),
            "https://llm.example.test/chat/completions"
        );
        assert_eq!(
            openai_compatible_chat_completions_url("https://api.openai.com/v1/chat/"),
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(
            openai_compatible_chat_completions_url("https://api.openai.com/v1/ch"),
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(
            openai_compatible_responses_url("https://llm.example.test/responses"),
            "https://llm.example.test/responses"
        );
        assert_eq!(
            openai_compatible_responses_url("https://proxy.example.test/openai/v1"),
            "https://proxy.example.test/openai/v1/responses"
        );
        assert_eq!(
            openai_compatible_models_url("https://llm.example.test/v1/chat/completions"),
            "https://llm.example.test/v1/models"
        );
        assert_eq!(
            openai_compatible_models_url("https://llm.example.test/v1/chat/"),
            "https://llm.example.test/v1/models"
        );
        assert_eq!(
            openai_compatible_models_url("https://llm.example.test/v1/responses"),
            "https://llm.example.test/v1/models"
        );
        assert_eq!(
            openai_compatible_models_url("https://llm.example.test/v1/models"),
            "https://llm.example.test/v1/models"
        );
    }
}

use super::client::{LLMClient, LlmModelLister, LLMRequest, LLMResponse, ModelInfo, ReasoningLevel};
use super::endpoint_url::complete_standard_endpoint;
use crate::error::AppError;
use crate::infrastructure::http::HttpClient;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

pub struct AnthropicLLMClient {
    http_client: Arc<dyn HttpClient>,
    endpoint: String,
    model: String,
    api_key: String,
}

pub(crate) fn anthropic_messages_url(endpoint: &str) -> String {
    complete_standard_endpoint(endpoint, "/v1/messages", &["/v1/messages", "/v1"])
}

pub(crate) fn anthropic_models_url(endpoint: &str) -> String {
    complete_standard_endpoint(
        endpoint,
        "/v1/models",
        &["/v1/messages", "/v1/models", "/v1"],
    )
}

impl AnthropicLLMClient {
    pub fn new(
        http_client: Arc<dyn HttpClient>,
        endpoint: String,
        model: String,
        api_key: String,
    ) -> Self {
        Self {
            http_client,
            endpoint,
            model,
            api_key,
        }
    }

    /// 判断是否支持 extended thinking（参考 Pi: 按模型检测）
    fn supports_thinking(&self) -> bool {
        // Claude Sonnet 3.7+ / Opus 4.0+ 支持
        (self.model.contains("sonnet") && (self.model.contains("3.7") || self.model.contains("4.")))
            || (self.model.contains("opus") && self.model.contains("4."))
    }

    /// 应用 reasoning（参考 Pi: thinkingEnabled + budgetTokens）
    fn apply_reasoning(&self, body: &mut serde_json::Value, level: ReasoningLevel) {
        if !self.supports_thinking() {
            return; // 静默忽略
        }

        let budget = match level {
            ReasoningLevel::Minimal => 1024,
            ReasoningLevel::Low => 2048,
            ReasoningLevel::Medium => 4096,
            ReasoningLevel::High => 8192,
            ReasoningLevel::XHigh => 16384,
        };

        body["thinking"] = json!({
            "type": "enabled",
            "budget_tokens": budget,
        });
    }
}

#[async_trait]
impl LLMClient for AnthropicLLMClient {
    async fn generate(&self, request: &LLMRequest) -> Result<LLMResponse> {
        let url = anthropic_messages_url(&self.endpoint);

        let mut body = json!({
            "model": self.model,
            "messages": [{
                "role": "user",
                "content": &request.user_prompt,
            }],
            "max_tokens": request.options.max_tokens.unwrap_or(8192),
        });

        if let Some(system) = &request.system_prompt {
            body["system"] = json!(system);
        }

        if let Some(temp) = request.options.temperature {
            body["temperature"] = json!(temp);
        }

        // Reasoning
        if let Some(level) = request.options.reasoning {
            self.apply_reasoning(&mut body, level);
        }

        // 发送请求
        let mut headers = HashMap::new();
        headers.insert("x-api-key".to_string(), self.api_key.clone());
        headers.insert("anthropic-version".to_string(), "2023-06-01".to_string());
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

        // 解析响应（参考 Pi: 提取 text 类型的 content block）
        let json: serde_json::Value = serde_json::from_str(&response.body)
            .map_err(|e| AppError::InvalidResponse(format!("JSON parse: {}", e)))?;

        let content_blocks = json["content"]
            .as_array()
            .ok_or_else(|| AppError::InvalidResponse("Missing content array".into()))?;

        let mut text = String::new();
        for block in content_blocks {
            if block["type"] == "text" {
                if let Some(t) = block["text"].as_str() {
                    text.push_str(t);
                }
            }
            // thinking block 忽略（翻译不需要）
        }

        if text.is_empty() {
            return Err(AppError::InvalidResponse("Empty text content".into()).into());
        }

        Ok(LLMResponse { text })
    }
}

#[async_trait]
impl LlmModelLister for AnthropicLLMClient {
    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        let url = anthropic_models_url(&self.endpoint);
        let mut headers = HashMap::new();
        headers.insert("x-api-key".to_string(), self.api_key.clone());
        headers.insert(
            "anthropic-version".to_string(),
            "2023-06-01".to_string(),
        );
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        let response = self
            .http_client
            .get(&url, headers)
            .await
            .map_err(|e| AppError::Network(e.to_string()))?;

        ensure_anthropic_success_status(response.status, &response.body)?;
        parse_anthropic_models_response(&response.body)
    }
}

fn ensure_anthropic_success_status(status: u16, body: &str) -> Result<()> {
    match status {
        200 => Ok(()),
        401 | 403 => Err(AppError::Unauthorized("Invalid API key or insufficient permission".into()).into()),
        404 => Err(AppError::InvalidResponse("API endpoint not found".into()).into()),
        429 => Err(AppError::RateLimited("Rate limit exceeded".into()).into()),
        _ => Err(AppError::UpstreamStatus(status, body.to_string()).into()),
    }
}

fn parse_anthropic_models_response(body: &str) -> Result<Vec<ModelInfo>> {
    let json: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| AppError::InvalidResponse(format!("Model list JSON parse failed: {}", e)))?;
    let data = json["data"]
        .as_array()
        .ok_or_else(|| AppError::InvalidResponse("Model list response is missing data array".into()))?;

    let models: Vec<_> = models_from_array(data, "id");

    if models.is_empty() {
        Err(AppError::InvalidResponse(
            "Model list response did not contain model ids".into(),
        )
        .into())
    } else {
        Ok(models)
    }
}

fn models_from_array(data: &[serde_json::Value], field: &str) -> Vec<ModelInfo> {
    data.iter()
        .filter_map(|item| {
            item[field]
                .as_str()
                .or_else(|| item.as_str())
                .map(|id| ModelInfo { id: id.to_string() })
        })
        .collect()
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

    #[tokio::test]
    async fn list_models_parses_anthropic_data_array() {
        let mock = Arc::new(ListModelsMockHttpClient {
            status: 200,
            body: r#"{"data":[{"id":"claude-sonnet-4-5"},{"id":"claude-haiku-4-5"}]}"#.to_string(),
        });
        let client = AnthropicLLMClient::new(
            mock,
            "https://api.anthropic.com".to_string(),
            "unused".to_string(),
            "key".to_string(),
        );

        let models = client.list_models().await.unwrap();

        let ids: Vec<_> = models.into_iter().map(|m| m.id).collect();
        assert_eq!(ids, vec!["claude-sonnet-4-5", "claude-haiku-4-5"]);
    }

    #[tokio::test]
    async fn list_models_rejects_response_without_data_array() {
        let mock = Arc::new(ListModelsMockHttpClient {
            status: 200,
            body: r#"{"object":"list"}"#.to_string(),
        });
        let client = AnthropicLLMClient::new(
            mock,
            "https://api.anthropic.com".to_string(),
            "unused".to_string(),
            "key".to_string(),
        );

        let error = client.list_models().await.unwrap_err();
        assert_eq!(
            error.to_string(),
            "Invalid response: Model list response is missing data array"
        );
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

        async fn get(
            &self,
            _url: &str,
            _headers: HashMap<String, String>,
        ) -> Result<HttpResponse> {
            Ok(HttpResponse {
                status: self.status,
                body: self.body.clone(),
                headers: HashMap::new(),
            })
        }
    }

    #[tokio::test]
    async fn test_anthropic_normal_response() {
        let mock = Arc::new(MockHttpClient {
            response: HttpResponse {
                status: 200,
                body: r#"{"content":[{"type":"text","text":"Hello"}]}"#.to_string(),
                headers: HashMap::new(),
            },
        });

        let client = AnthropicLLMClient::new(
            mock,
            "https://api.anthropic.com".to_string(),
            "claude-3-5-sonnet-latest".to_string(),
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
    async fn test_anthropic_thinking_support() {
        let client = AnthropicLLMClient::new(
            Arc::new(MockHttpClient {
                response: HttpResponse {
                    status: 200,
                    body: r#"{"content":[{"type":"text","text":"test"}]}"#.to_string(),
                    headers: HashMap::new(),
                },
            }),
            "https://api.anthropic.com".to_string(),
            "claude-sonnet-3.7".to_string(),
            "test-key".to_string(),
        );

        assert!(client.supports_thinking());
    }

    #[tokio::test]
    async fn test_anthropic_no_thinking_support() {
        let client = AnthropicLLMClient::new(
            Arc::new(MockHttpClient {
                response: HttpResponse {
                    status: 200,
                    body: r#"{"content":[{"type":"text","text":"test"}]}"#.to_string(),
                    headers: HashMap::new(),
                },
            }),
            "https://api.anthropic.com".to_string(),
            "claude-3-5-haiku".to_string(),
            "test-key".to_string(),
        );

        assert!(!client.supports_thinking());
    }

    #[tokio::test]
    async fn test_anthropic_unauthorized() {
        let mock = Arc::new(MockHttpClient {
            response: HttpResponse {
                status: 401,
                body: "Unauthorized".to_string(),
                headers: HashMap::new(),
            },
        });

        let client = AnthropicLLMClient::new(
            mock,
            "https://api.anthropic.com".to_string(),
            "claude-3-5-sonnet-latest".to_string(),
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

    #[test]
    fn test_anthropic_url_helpers_complete_standard_v1_prefixes() {
        assert_eq!(
            anthropic_messages_url("https://api.anthropic.com"),
            "https://api.anthropic.com/v1/messages"
        );
        assert_eq!(
            anthropic_messages_url("https://api.anthropic.com/v1/messages"),
            "https://api.anthropic.com/v1/messages"
        );
        assert_eq!(
            anthropic_messages_url("https://api.anthropic.com/messages"),
            "https://api.anthropic.com/messages"
        );
        assert_eq!(
            anthropic_models_url("https://api.anthropic.com/messages"),
            "https://api.anthropic.com/messages"
        );
    }
}

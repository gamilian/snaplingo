use super::client::{LLMClient, LLMRequest, LLMResponse, ReasoningLevel};
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
}

impl OpenAILLMClient {
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

    /// 判断是否是推理模型（参考 Pi: 按 model name 检测）
    fn is_reasoning_model(&self) -> bool {
        self.model.starts_with("o1") || self.model.starts_with("o3")
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
        let url = format!(
            "{}/v1/chat/completions",
            self.endpoint.trim_end_matches('/')
        );

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
}

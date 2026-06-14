use super::client::{LLMClient, LLMRequest, LLMResponse, ReasoningLevel};
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
        (self.model.contains("sonnet")
            && (self.model.contains("3.7") || self.model.contains("4.")))
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
        let url = format!("{}/v1/messages", self.endpoint.trim_end_matches('/'));

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
        headers.insert(
            "anthropic-version".to_string(),
            "2023-06-01".to_string(),
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

        async fn get(
            &self,
            _url: &str,
            _headers: HashMap<String, String>,
        ) -> Result<HttpResponse> {
            unimplemented!()
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
}

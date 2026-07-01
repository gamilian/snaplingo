use super::client::{LLMClient, LLMRequest, LLMResponse, ReasoningLevel};
use super::endpoint_url::complete_standard_endpoint;
use crate::error::AppError;
use crate::infrastructure::http::HttpClient;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

pub struct GeminiLLMClient {
    http_client: Arc<dyn HttpClient>,
    endpoint: String,
    model: String,
    api_key: String,
}

pub(crate) fn gemini_generate_content_url(endpoint: &str, model: &str, api_key: &str) -> String {
    let endpoint = endpoint.trim();
    let key = urlencoding::encode(api_key);
    let model_id = model.strip_prefix("models/").unwrap_or(model);
    let standard_path = format!("/v1beta/models/{}:generateContent", model_id);
    let model_path = format!("/v1beta/models/{}", model_id);

    if is_standard_gemini_generate_content_endpoint(endpoint) {
        return append_gemini_key(endpoint.trim_end_matches('/'), &key);
    } else {
        let request_url = complete_standard_endpoint(
            endpoint,
            &standard_path,
            &[
                standard_path.as_str(),
                model_path.as_str(),
                "/v1beta/models",
                "/v1beta",
            ],
        );
        append_gemini_key(&request_url, &key)
    }
}

pub(crate) fn gemini_models_url(endpoint: &str, api_key: &str) -> String {
    let endpoint = endpoint.trim();
    let endpoint_without_query = endpoint.split('?').next().unwrap_or(endpoint);
    let models_url = if let Some((base, _)) = endpoint_without_query.split_once("/v1beta/models/") {
        format!("{}/v1beta/models", base.trim_end_matches('/'))
    } else {
        complete_standard_endpoint(endpoint, "/v1beta/models", &["/v1beta/models", "/v1beta"])
    };

    append_gemini_key(&models_url, &urlencoding::encode(api_key))
}

fn append_gemini_key(endpoint: &str, encoded_key: &str) -> String {
    if endpoint.contains('?') {
        endpoint.to_string()
    } else {
        format!("{}?key={}", endpoint, encoded_key)
    }
}

fn is_standard_gemini_generate_content_endpoint(endpoint: &str) -> bool {
    let endpoint_without_query = endpoint.split('?').next().unwrap_or(endpoint);
    endpoint_without_query.contains("/v1beta/models/")
        && endpoint_without_query.ends_with(":generateContent")
}

impl GeminiLLMClient {
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

    /// 判断是否支持 thinking（参考 Pi）
    fn supports_thinking(&self) -> bool {
        // Gemini 2.0+ 支持
        self.model.contains("gemini-2.") || self.model.contains("gemini-pro-2.")
    }

    fn apply_reasoning(&self, body: &mut serde_json::Value, level: ReasoningLevel) {
        if !self.supports_thinking() {
            return;
        }

        let budget = match level {
            ReasoningLevel::Minimal => 1024,
            ReasoningLevel::Low => 2048,
            ReasoningLevel::Medium => 4096,
            ReasoningLevel::High => 8192,
            ReasoningLevel::XHigh => 16384,
        };

        body["thinking"] = json!({
            "enabled": true,
            "budgetTokens": budget,
        });
    }
}

#[async_trait]
impl LLMClient for GeminiLLMClient {
    async fn generate(&self, request: &LLMRequest) -> Result<LLMResponse> {
        let url = gemini_generate_content_url(&self.endpoint, &self.model, &self.api_key);

        let mut body = json!({
            "contents": [{
                "parts": [{"text": &request.user_prompt}]
            }],
        });

        if let Some(system) = &request.system_prompt {
            body["systemInstruction"] = json!({
                "parts": [{"text": system}]
            });
        }

        // Gemini 用 generationConfig
        let mut generation_config = serde_json::Map::new();
        if let Some(temp) = request.options.temperature {
            generation_config.insert("temperature".to_string(), json!(temp));
        }
        if let Some(max_tokens) = request.options.max_tokens {
            generation_config.insert("maxOutputTokens".to_string(), json!(max_tokens));
        }
        if !generation_config.is_empty() {
            body["generationConfig"] = json!(generation_config);
        }

        // Reasoning
        if let Some(level) = request.options.reasoning {
            self.apply_reasoning(&mut body, level);
        }

        // 发送请求
        let headers = HashMap::from([("Content-Type".to_string(), "application/json".to_string())]);

        let response = self
            .http_client
            .post(&url, headers, body.to_string())
            .await
            .map_err(|e| AppError::Network(e.to_string()))?;

        // 错误处理
        if response.status == 401 || response.status == 403 {
            return Err(AppError::Unauthorized("Invalid API key".into()).into());
        } else if response.status == 429 {
            return Err(AppError::RateLimited("Rate limit exceeded".into()).into());
        } else if response.status != 200 {
            return Err(AppError::UpstreamStatus(response.status, response.body).into());
        }

        // 解析响应
        let json: serde_json::Value = serde_json::from_str(&response.body)
            .map_err(|e| AppError::InvalidResponse(format!("JSON parse: {}", e)))?;

        let text = json["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .ok_or_else(|| AppError::InvalidResponse("Missing text field".into()))?
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
    async fn test_gemini_normal_response() {
        let mock = Arc::new(MockHttpClient {
            response: HttpResponse {
                status: 200,
                body: r#"{"candidates":[{"content":{"parts":[{"text":"Hello"}]}}]}"#.to_string(),
                headers: HashMap::new(),
            },
        });

        let client = GeminiLLMClient::new(
            mock,
            "https://generativelanguage.googleapis.com".to_string(),
            "gemini-1.5-flash".to_string(),
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
    async fn test_gemini_thinking_support() {
        let client = GeminiLLMClient::new(
            Arc::new(MockHttpClient {
                response: HttpResponse {
                    status: 200,
                    body: r#"{"candidates":[{"content":{"parts":[{"text":"test"}]}}]}"#.to_string(),
                    headers: HashMap::new(),
                },
            }),
            "https://generativelanguage.googleapis.com".to_string(),
            "gemini-2.0-flash".to_string(),
            "test-key".to_string(),
        );

        assert!(client.supports_thinking());
    }

    #[tokio::test]
    async fn test_gemini_no_thinking_support() {
        let client = GeminiLLMClient::new(
            Arc::new(MockHttpClient {
                response: HttpResponse {
                    status: 200,
                    body: r#"{"candidates":[{"content":{"parts":[{"text":"test"}]}}]}"#.to_string(),
                    headers: HashMap::new(),
                },
            }),
            "https://generativelanguage.googleapis.com".to_string(),
            "gemini-1.5-flash".to_string(),
            "test-key".to_string(),
        );

        assert!(!client.supports_thinking());
    }

    #[tokio::test]
    async fn test_gemini_unauthorized() {
        let mock = Arc::new(MockHttpClient {
            response: HttpResponse {
                status: 403,
                body: "Forbidden".to_string(),
                headers: HashMap::new(),
            },
        });

        let client = GeminiLLMClient::new(
            mock,
            "https://generativelanguage.googleapis.com".to_string(),
            "gemini-1.5-flash".to_string(),
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
    fn test_gemini_url_helpers_complete_standard_v1beta_prefixes() {
        assert_eq!(
            gemini_generate_content_url(
                "https://generativelanguage.googleapis.com",
                "gemini-2.5-pro",
                "test-key"
            ),
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-pro:generateContent?key=test-key"
        );
        assert_eq!(
            gemini_generate_content_url(
                "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-pro:generateContent",
                "gemini-2.5-pro",
                "test-key"
            ),
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-pro:generateContent?key=test-key"
        );
        assert_eq!(
            gemini_generate_content_url(
                "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-pro",
                "gemini-2.5-pro",
                "test-key"
            ),
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-pro:generateContent?key=test-key"
        );
        assert_eq!(
            gemini_generate_content_url(
                "https://generativelanguage.googleapis.com/models/gemini-2.5-pro:generateContent",
                "gemini-2.5-pro",
                "test-key"
            ),
            "https://generativelanguage.googleapis.com/models/gemini-2.5-pro:generateContent?key=test-key"
        );
        assert_eq!(
            gemini_models_url(
                "https://generativelanguage.googleapis.com/models/gemini-2.5-pro:generateContent",
                "test-key"
            ),
            "https://generativelanguage.googleapis.com/models/gemini-2.5-pro:generateContent?key=test-key"
        );
    }
}

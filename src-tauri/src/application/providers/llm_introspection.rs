use crate::infrastructure::http::HttpClient;
use crate::infrastructure::llm::{
    AnthropicLLMClient, GeminiLLMClient, LlmModelLister, LLMClient, LLMOptions, LLMProtocol,
    LLMRequest, ModelInfo, OpenAILLMClient,
};
use anyhow::Result;
use std::sync::Arc;

/// Facade for LLM provider introspection: list models and test connectivity before saving configuration.
pub struct LlmIntrospection {
    http_client: Arc<dyn HttpClient>,
}

impl LlmIntrospection {
    pub fn new(http_client: Arc<dyn HttpClient>) -> Self {
        Self { http_client }
    }

    /// List models available from the provider at the given endpoint.
    pub async fn list_models(
        &self,
        protocol: LLMProtocol,
        endpoint: &str,
        api_key: &str,
    ) -> Result<Vec<ModelInfo>> {
        let lister: Arc<dyn LlmModelLister> = match protocol {
            LLMProtocol::OpenAI | LLMProtocol::OpenAIResponses => Arc::new(
                OpenAILLMClient::new_chat_completions(
                    self.http_client.clone(),
                    endpoint.to_string(),
                    String::new(), // model not needed for listing
                    api_key.to_string(),
                ),
            ),
            LLMProtocol::Anthropic => Arc::new(AnthropicLLMClient::new(
                self.http_client.clone(),
                endpoint.to_string(),
                String::new(),
                api_key.to_string(),
            )),
            LLMProtocol::Gemini => Arc::new(GeminiLLMClient::new(
                self.http_client.clone(),
                endpoint.to_string(),
                String::new(),
                api_key.to_string(),
            )),
        };
        lister.list_models().await
    }

    /// Test connectivity by sending a minimal generate request.
    pub async fn test(
        &self,
        protocol: LLMProtocol,
        endpoint: &str,
        model: &str,
        api_key: &str,
    ) -> Result<()> {
        let client: Arc<dyn LLMClient> = match protocol {
            LLMProtocol::OpenAI => Arc::new(OpenAILLMClient::new_chat_completions(
                self.http_client.clone(),
                endpoint.to_string(),
                model.to_string(),
                api_key.to_string(),
            )),
            LLMProtocol::OpenAIResponses => Arc::new(OpenAILLMClient::new_responses(
                self.http_client.clone(),
                endpoint.to_string(),
                model.to_string(),
                api_key.to_string(),
            )),
            LLMProtocol::Anthropic => Arc::new(AnthropicLLMClient::new(
                self.http_client.clone(),
                endpoint.to_string(),
                model.to_string(),
                api_key.to_string(),
            )),
            LLMProtocol::Gemini => Arc::new(GeminiLLMClient::new(
                self.http_client.clone(),
                endpoint.to_string(),
                model.to_string(),
                api_key.to_string(),
            )),
        };

        let request = LLMRequest {
            system_prompt: Some("You are a translation engine. Return only OK.".to_string()),
            user_prompt: "OK".to_string(),
            options: LLMOptions {
                reasoning: None,
                temperature: Some(0.0),
                max_tokens: Some(8),
            },
        };

        client.generate(&request).await.map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::http::{HttpClient, HttpResponse};
    use anyhow::Result;
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::Arc;

    struct MockHttpClient {
        get_response: HttpResponse,
        post_response: HttpResponse,
    }

    #[async_trait]
    impl HttpClient for MockHttpClient {
        async fn get(&self, _url: &str, _headers: HashMap<String, String>) -> Result<HttpResponse> {
            Ok(self.get_response.clone())
        }

        async fn post(
            &self,
            _url: &str,
            _headers: HashMap<String, String>,
            _body: String,
        ) -> Result<HttpResponse> {
            Ok(self.post_response.clone())
        }
    }

    fn mock_http_client(get_body: &str, post_body: &str) -> Arc<dyn HttpClient> {
        Arc::new(MockHttpClient {
            get_response: HttpResponse {
                status: 200,
                body: get_body.to_string(),
                headers: HashMap::new(),
            },
            post_response: HttpResponse {
                status: 200,
                body: post_body.to_string(),
                headers: HashMap::new(),
            },
        })
    }

    #[tokio::test]
    async fn list_models_dispatches_openai_protocol() {
        let http = mock_http_client(r#"{"data":[{"id":"gpt-4"}]}"#, "");
        let introspection = LlmIntrospection::new(http);

        let models = introspection
            .list_models(LLMProtocol::OpenAI, "https://api.openai.com/v1", "test-key")
            .await
            .unwrap();

        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "gpt-4");
    }

    #[tokio::test]
    async fn list_models_dispatches_openai_responses_to_same_client() {
        let http = mock_http_client(r#"{"data":[{"id":"gpt-4o"}]}"#, "");
        let introspection = LlmIntrospection::new(http);

        let models = introspection
            .list_models(LLMProtocol::OpenAIResponses, "https://api.openai.com/v1", "test-key")
            .await
            .unwrap();

        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "gpt-4o");
    }

    #[tokio::test]
    async fn list_models_dispatches_anthropic_protocol() {
        let http = mock_http_client(r#"{"data":[{"id":"claude-3-5-sonnet-20241022"}]}"#, "");
        let introspection = LlmIntrospection::new(http);

        let models = introspection
            .list_models(LLMProtocol::Anthropic, "https://api.anthropic.com/v1", "test-key")
            .await
            .unwrap();

        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "claude-3-5-sonnet-20241022");
    }

    #[tokio::test]
    async fn list_models_dispatches_gemini_protocol() {
        let http = mock_http_client(r#"{"models":[{"name":"models/gemini-pro"}]}"#, "");
        let introspection = LlmIntrospection::new(http);

        let models = introspection
            .list_models(LLMProtocol::Gemini, "https://generativelanguage.googleapis.com/v1", "test-key")
            .await
            .unwrap();

        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "gemini-pro");  // Gemini strips "models/" prefix
    }

    #[tokio::test]
    async fn test_sends_generate_request_for_openai() {
        let http = mock_http_client(
            "",
            r#"{"choices":[{"message":{"content":"OK"}}]}"#,
        );
        let introspection = LlmIntrospection::new(http);

        let result = introspection
            .test(LLMProtocol::OpenAI, "https://api.openai.com/v1", "gpt-4", "test-key")
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_sends_generate_request_for_anthropic() {
        let http = mock_http_client(
            "",
            r#"{"content":[{"type":"text","text":"OK"}]}"#,
        );
        let introspection = LlmIntrospection::new(http);

        let result = introspection
            .test(LLMProtocol::Anthropic, "https://api.anthropic.com/v1", "claude-3-5-sonnet-20241022", "test-key")
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_sends_generate_request_for_gemini() {
        let http = mock_http_client(
            "",
            r#"{"candidates":[{"content":{"parts":[{"text":"OK"}]}}]}"#,
        );
        let introspection = LlmIntrospection::new(http);

        let result = introspection
            .test(LLMProtocol::Gemini, "https://generativelanguage.googleapis.com/v1", "gemini-pro", "test-key")
            .await;

        assert!(result.is_ok());
    }
}

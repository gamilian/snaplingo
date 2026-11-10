use super::*;
use anyhow::Result;
use snaplingo_core::providers::*;
use std::sync::Arc;
mod cases {

    use super::*;
    use anyhow::Result;
    use async_trait::async_trait;
    use snaplingo_providers::http::HttpClient;
    use snaplingo_providers::http::HttpResponse;
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

    fn mock_llm_runtime(get_body: &str, post_body: &str) -> Arc<dyn LlmRuntime> {
        Arc::new(snaplingo_providers::llm::ProviderLlmRuntime::new(
            mock_http_client(get_body, post_body),
        ))
    }

    #[tokio::test]
    async fn list_models_dispatches_openai_protocol() {
        let introspection =
            LlmIntrospection::new(mock_llm_runtime(r#"{"data":[{"id":"gpt-4"}]}"#, ""));

        let models = introspection
            .list_models(LLMProtocol::OpenAI, "https://api.openai.com/v1", "test-key")
            .await
            .unwrap();

        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "gpt-4");
    }

    #[tokio::test]
    async fn list_models_dispatches_openai_responses_to_same_client() {
        let introspection =
            LlmIntrospection::new(mock_llm_runtime(r#"{"data":[{"id":"gpt-4o"}]}"#, ""));

        let models = introspection
            .list_models(
                LLMProtocol::OpenAIResponses,
                "https://api.openai.com/v1",
                "test-key",
            )
            .await
            .unwrap();

        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "gpt-4o");
    }

    #[tokio::test]
    async fn list_models_dispatches_anthropic_protocol() {
        let introspection = LlmIntrospection::new(mock_llm_runtime(
            r#"{"data":[{"id":"claude-3-5-sonnet-20241022"}]}"#,
            "",
        ));

        let models = introspection
            .list_models(
                LLMProtocol::Anthropic,
                "https://api.anthropic.com/v1",
                "test-key",
            )
            .await
            .unwrap();

        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "claude-3-5-sonnet-20241022");
    }

    #[tokio::test]
    async fn list_models_dispatches_gemini_protocol() {
        let introspection = LlmIntrospection::new(mock_llm_runtime(
            r#"{"models":[{"name":"models/gemini-pro"}]}"#,
            "",
        ));

        let models = introspection
            .list_models(
                LLMProtocol::Gemini,
                "https://generativelanguage.googleapis.com/v1",
                "test-key",
            )
            .await
            .unwrap();

        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "gemini-pro"); // Gemini strips "models/" prefix
    }

    #[tokio::test]
    async fn test_sends_generate_request_for_openai() {
        let introspection = LlmIntrospection::new(mock_llm_runtime(
            "",
            r#"{"choices":[{"message":{"content":"OK"}}]}"#,
        ));

        let result = introspection
            .test(
                LLMProtocol::OpenAI,
                "https://api.openai.com/v1",
                "gpt-4",
                "test-key",
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_sends_generate_request_for_anthropic() {
        let introspection = LlmIntrospection::new(mock_llm_runtime(
            "",
            r#"{"content":[{"type":"text","text":"OK"}]}"#,
        ));

        let result = introspection
            .test(
                LLMProtocol::Anthropic,
                "https://api.anthropic.com/v1",
                "claude-3-5-sonnet-20241022",
                "test-key",
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_sends_generate_request_for_gemini() {
        let introspection = LlmIntrospection::new(mock_llm_runtime(
            "",
            r#"{"candidates":[{"content":{"parts":[{"text":"OK"}]}}]}"#,
        ));

        let result = introspection
            .test(
                LLMProtocol::Gemini,
                "https://generativelanguage.googleapis.com/v1",
                "gemini-pro",
                "test-key",
            )
            .await;

        assert!(result.is_ok());
    }
}

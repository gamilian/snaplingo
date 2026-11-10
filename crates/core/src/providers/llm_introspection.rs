use crate::providers::LLMOptions;
use crate::providers::LLMProtocol;
use crate::providers::LLMRequest;
use crate::providers::LlmClientConfig;
use crate::providers::LlmRuntime;
use crate::providers::ModelInfo;
use anyhow::Result;
use std::sync::Arc;

/// Facade for LLM provider introspection: list models and test connectivity before saving configuration.
pub struct LlmIntrospection {
    llm_runtime: Arc<dyn LlmRuntime>,
}

impl LlmIntrospection {
    pub fn new(llm_runtime: Arc<dyn LlmRuntime>) -> Self {
        Self { llm_runtime }
    }

    /// List models available from the provider at the given endpoint.
    pub async fn list_models(
        &self,
        protocol: LLMProtocol,
        endpoint: &str,
        api_key: &str,
    ) -> Result<Vec<ModelInfo>> {
        let lister = self.llm_runtime.model_lister(LlmClientConfig {
            protocol,
            endpoint: endpoint.to_string(),
            model: String::new(),
            api_key: api_key.to_string(),
        });
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
        let client = self.llm_runtime.translation_client(LlmClientConfig {
            protocol,
            endpoint: endpoint.to_string(),
            model: model.to_string(),
            api_key: api_key.to_string(),
        });

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

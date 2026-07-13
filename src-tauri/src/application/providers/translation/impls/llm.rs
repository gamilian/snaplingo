use crate::application::providers::common::Provider;
use crate::application::providers::translation::TranslationProvider;
use crate::application::providers::{
    merge_prompt_strategy_config, render_translation_system_prompt, ProviderConfigStore,
    ProviderPromptStrategy,
};
use crate::application::providers::{LLMClient, LLMOptions, LLMRequest, ReasoningLevel};
use crate::domain::translation::{TranslationRequest, TranslationResult};
use crate::Result;
use async_trait::async_trait;
use std::sync::Arc;

/// LLM-based translation provider.
///
/// Uses LLM APIs (OpenAI/Anthropic/Gemini) for translation.
pub struct LLMTranslationProvider {
    llm_client: Arc<dyn LLMClient>,
    id: String,
    name: String,
    reasoning_level: Option<ReasoningLevel>,
    prompt_strategy: ProviderPromptStrategy,
    config_store: Arc<dyn ProviderConfigStore>,
}

impl LLMTranslationProvider {
    pub fn new(
        llm_client: Arc<dyn LLMClient>,
        id: String,
        name: String,
        reasoning_level: Option<ReasoningLevel>,
        prompt_strategy: ProviderPromptStrategy,
        config_store: Arc<dyn ProviderConfigStore>,
    ) -> Self {
        Self {
            llm_client,
            id,
            name,
            reasoning_level,
            prompt_strategy,
            config_store,
        }
    }

    fn system_prompt(&self, request: &TranslationRequest) -> String {
        let config = self.config_store.load_translation_prompt_strategies().ok();
        let config = merge_prompt_strategy_config(config);

        render_translation_system_prompt(&config.strategies, &self.prompt_strategy, request)
    }
}

impl Provider for LLMTranslationProvider {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn is_configured(&self) -> bool {
        true
    }

    fn requires_api_key(&self) -> bool {
        true
    }
}

#[async_trait]
impl TranslationProvider for LLMTranslationProvider {
    async fn translate(&self, request: &TranslationRequest) -> Result<TranslationResult> {
        let llm_request = LLMRequest {
            system_prompt: Some(self.system_prompt(request)),
            user_prompt: request.text.clone(),
            options: LLMOptions {
                reasoning: self.reasoning_level,
                temperature: Some(0.2), // 翻译用低温度
                max_tokens: Some(8192),
            },
        };

        let response = self.llm_client.generate(&llm_request).await?;

        Ok(TranslationResult {
            provider_id: self.id().to_string(),
            translated_text: response.text,
            detected_language: None,
            confidence: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::providers::TranslationPromptStrategyConfig;
    use crate::application::providers::{LLMClient, LLMRequest, LLMResponse};
    use crate::infrastructure::storage::SqliteConfigStore;
    use async_trait::async_trait;

    struct MockLLMClient {
        response: String,
        requests: Arc<std::sync::Mutex<Vec<LLMRequest>>>,
    }

    #[async_trait]
    impl LLMClient for MockLLMClient {
        async fn generate(&self, request: &LLMRequest) -> anyhow::Result<LLMResponse> {
            self.requests.lock().unwrap().push(request.clone());
            Ok(LLMResponse {
                text: self.response.clone(),
            })
        }
    }

    #[tokio::test]
    async fn test_llm_translation_provider() {
        let requests = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mock_client = Arc::new(MockLLMClient {
            response: "Bonjour".to_string(),
            requests,
        });

        let provider = LLMTranslationProvider::new(
            mock_client,
            "test-llm".to_string(),
            "Test LLM".to_string(),
            None,
            ProviderPromptStrategy::default(),
            Arc::new(SqliteConfigStore::new_temp()),
        );

        let request = TranslationRequest {
            text: "Hello".to_string(),
            source_lang: "en".to_string(),
            target_lang: "fr".to_string(),
        };

        let result = provider.translate(&request).await.unwrap();
        assert_eq!(result.translated_text, "Bonjour");
    }

    #[tokio::test]
    async fn test_llm_translation_with_reasoning() {
        let requests = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mock_client = Arc::new(MockLLMClient {
            response: "Bonjour le monde".to_string(),
            requests,
        });

        let provider = LLMTranslationProvider::new(
            mock_client,
            "test-llm-reasoning".to_string(),
            "Test LLM Reasoning".to_string(),
            Some(ReasoningLevel::High),
            ProviderPromptStrategy::default(),
            Arc::new(SqliteConfigStore::new_temp()),
        );

        let request = TranslationRequest {
            text: "Hello world".to_string(),
            source_lang: "auto".to_string(),
            target_lang: "fr".to_string(),
        };

        let result = provider.translate(&request).await.unwrap();
        assert_eq!(result.translated_text, "Bonjour le monde");
    }

    #[tokio::test]
    async fn llm_translation_uses_edited_general_prompt_from_config() {
        let config_file = Arc::new(SqliteConfigStore::new_temp());
        config_file
            .save(
                "translation_prompt_strategies",
                &TranslationPromptStrategyConfig {
                    strategies: vec![crate::application::providers::TranslationPromptStrategy {
                        id: crate::application::providers::DEFAULT_PROMPT_STRATEGY_ID.to_string(),
                        name: "通用翻译".to_string(),
                        description: "".to_string(),
                        system_prompt: "Custom prompt to {target_lang}".to_string(),
                        is_builtin: true,
                        is_deletable: false,
                    }],
                },
            )
            .unwrap();
        let requests = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mock_client = Arc::new(MockLLMClient {
            response: "Bonjour".to_string(),
            requests: requests.clone(),
        });

        let provider = LLMTranslationProvider::new(
            mock_client,
            "test-llm".to_string(),
            "Test LLM".to_string(),
            None,
            ProviderPromptStrategy {
                strategy_id: crate::application::providers::DEFAULT_PROMPT_STRATEGY_ID.to_string(),
                fallback_strategy_id: crate::application::providers::DEFAULT_PROMPT_STRATEGY_ID
                    .to_string(),
            },
            config_file,
        );

        provider
            .translate(&TranslationRequest {
                text: "Hello".to_string(),
                source_lang: "en".to_string(),
                target_lang: "fr".to_string(),
            })
            .await
            .unwrap();

        assert_eq!(
            requests.lock().unwrap()[0].system_prompt.as_deref(),
            Some("Custom prompt to fr")
        );
    }
}

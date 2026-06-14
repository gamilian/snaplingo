use crate::application::providers::common::Provider;
use crate::application::providers::translation::TranslationProvider;
use crate::domain::translation::{TranslationRequest, TranslationResult};
use crate::infrastructure::llm::{LLMClient, LLMOptions, LLMRequest, ReasoningLevel};
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
}

impl LLMTranslationProvider {
    pub fn new(
        llm_client: Arc<dyn LLMClient>,
        id: String,
        name: String,
        reasoning_level: Option<ReasoningLevel>,
    ) -> Self {
        Self {
            llm_client,
            id,
            name,
            reasoning_level,
        }
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
            system_prompt: Some(format!(
                "You are a translation engine. Translate the user's text to {}. Return only the translation.",
                request.target_lang
            )),
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
    use crate::infrastructure::llm::{LLMClient, LLMRequest, LLMResponse};
    use async_trait::async_trait;

    struct MockLLMClient {
        response: String,
    }

    #[async_trait]
    impl LLMClient for MockLLMClient {
        async fn generate(&self, _request: &LLMRequest) -> anyhow::Result<LLMResponse> {
            Ok(LLMResponse {
                text: self.response.clone(),
            })
        }
    }

    #[tokio::test]
    async fn test_llm_translation_provider() {
        let mock_client = Arc::new(MockLLMClient {
            response: "Bonjour".to_string(),
        });

        let provider = LLMTranslationProvider::new(
            mock_client,
            "test-llm".to_string(),
            "Test LLM".to_string(),
            None,
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
        let mock_client = Arc::new(MockLLMClient {
            response: "Bonjour le monde".to_string(),
        });

        let provider = LLMTranslationProvider::new(
            mock_client,
            "test-llm-reasoning".to_string(),
            "Test LLM Reasoning".to_string(),
            Some(ReasoningLevel::High),
        );

        let request = TranslationRequest {
            text: "Hello world".to_string(),
            source_lang: "auto".to_string(),
            target_lang: "fr".to_string(),
        };

        let result = provider.translate(&request).await.unwrap();
        assert_eq!(result.translated_text, "Bonjour le monde");
    }
}

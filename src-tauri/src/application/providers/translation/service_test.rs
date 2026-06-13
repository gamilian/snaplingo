#[cfg(test)]
mod tests {
    use super::super::service::TranslationService;
    use super::super::registry::TranslationRegistry;
    use super::super::TranslationProvider;
    use crate::application::providers::common::Provider;
    use crate::domain::translation::{TranslationRequest, TranslationResult};
    use crate::Result;
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};

    // Mock provider for testing
    struct MockTranslationProvider {
        id: String,
        name: String,
        response_text: String,
    }

    impl MockTranslationProvider {
        fn new(id: &str, name: &str, response_text: &str) -> Self {
            Self {
                id: id.to_string(),
                name: name.to_string(),
                response_text: response_text.to_string(),
            }
        }
    }

    impl Provider for MockTranslationProvider {
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
            false
        }
    }

    #[async_trait]
    impl TranslationProvider for MockTranslationProvider {
        async fn translate(&self, _request: &TranslationRequest) -> Result<TranslationResult> {
            Ok(TranslationResult {
                translated_text: self.response_text.clone(),
                detected_language: Some("en".to_string()),
                confidence: Some(1.0),
            })
        }
    }

    #[tokio::test]
    async fn test_translate_with_single_provider() {
        let mut registry = TranslationRegistry::new();
        let provider = Arc::new(MockTranslationProvider::new(
            "google",
            "Google Translate",
            "Hola",
        ));

        registry.register(provider.clone()).unwrap();
        registry.activate("google").unwrap();

        let service = TranslationService::new(Arc::new(Mutex::new(registry)));

        let request = TranslationRequest {
            text: "Hello".to_string(),
            source_lang: "en".to_string(),
            target_lang: "es".to_string(),
        };

        let results = service.translate(&request).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].translated_text, "Hola");
    }

    #[tokio::test]
    async fn test_translate_with_multiple_providers() {
        let mut registry = TranslationRegistry::new();
        let provider1 = Arc::new(MockTranslationProvider::new(
            "google",
            "Google Translate",
            "Hola (Google)",
        ));
        let provider2 = Arc::new(MockTranslationProvider::new(
            "deepl",
            "DeepL",
            "Hola (DeepL)",
        ));

        registry.register(provider1.clone()).unwrap();
        registry.register(provider2.clone()).unwrap();
        registry.activate("google").unwrap();
        registry.activate("deepl").unwrap();

        let service = TranslationService::new(Arc::new(Mutex::new(registry)));

        let request = TranslationRequest {
            text: "Hello".to_string(),
            source_lang: "en".to_string(),
            target_lang: "es".to_string(),
        };

        let results = service.translate(&request).await.unwrap();

        assert_eq!(results.len(), 2);

        // Results should contain both provider responses
        let texts: Vec<String> = results.iter().map(|r| r.translated_text.clone()).collect();
        assert!(texts.contains(&"Hola (Google)".to_string()));
        assert!(texts.contains(&"Hola (DeepL)".to_string()));
    }

    #[tokio::test]
    async fn test_translate_with_no_active_provider() {
        let registry = TranslationRegistry::new();
        let service = TranslationService::new(Arc::new(Mutex::new(registry)));

        let request = TranslationRequest {
            text: "Hello".to_string(),
            source_lang: "en".to_string(),
            target_lang: "es".to_string(),
        };

        let result = service.translate(&request).await;

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "No active translation providers"
        );
    }
}

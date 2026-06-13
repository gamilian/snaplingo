#[cfg(test)]
mod tests {
    use super::super::registry::TranslationRegistry;
    use super::super::TranslationProvider;
    use crate::application::providers::common::Provider;
    use crate::domain::translation::{TranslationRequest, TranslationResult};
    use crate::Result;
    use async_trait::async_trait;
    use std::sync::Arc;

    // Mock provider for testing
    struct MockTranslationProvider {
        id: String,
        name: String,
    }

    impl MockTranslationProvider {
        fn new(id: &str, name: &str) -> Self {
            Self {
                id: id.to_string(),
                name: name.to_string(),
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
                translated_text: "translated".to_string(),
                detected_language: Some("en".to_string()),
                confidence: Some(1.0),
            })
        }
    }

    #[test]
    fn test_register_provider() {
        let mut registry = TranslationRegistry::new();
        let provider = Arc::new(MockTranslationProvider::new("google", "Google Translate"));

        registry.register(provider.clone()).unwrap();

        let all_providers = registry.list_all();
        assert_eq!(all_providers.len(), 1);
        assert_eq!(all_providers[0].id(), "google");
    }

    #[test]
    fn test_activate_multiple_providers() {
        let mut registry = TranslationRegistry::new();
        let provider1 = Arc::new(MockTranslationProvider::new("google", "Google Translate"));
        let provider2 = Arc::new(MockTranslationProvider::new("deepl", "DeepL"));

        registry.register(provider1.clone()).unwrap();
        registry.register(provider2.clone()).unwrap();

        registry.activate("google").unwrap();
        registry.activate("deepl").unwrap();

        let active = registry.get_active();
        assert_eq!(active.len(), 2);
        assert_eq!(active[0].id(), "google");
        assert_eq!(active[1].id(), "deepl");
    }

    #[test]
    fn test_deactivate_provider() {
        let mut registry = TranslationRegistry::new();
        let provider = Arc::new(MockTranslationProvider::new("google", "Google Translate"));

        registry.register(provider.clone()).unwrap();
        registry.activate("google").unwrap();

        let active = registry.get_active();
        assert_eq!(active.len(), 1);

        registry.deactivate("google").unwrap();

        let active_after = registry.get_active();
        assert_eq!(active_after.len(), 0);
    }

    #[test]
    fn test_activate_nonexistent_provider() {
        let mut registry = TranslationRegistry::new();

        let result = registry.activate("nonexistent");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Provider not found: nonexistent"
        );
    }
}

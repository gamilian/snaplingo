#[cfg(test)]
mod tests {
    use super::super::coordinator::TranslationCoordinator;
    use super::super::TranslationProvider;
    use crate::application::providers::common::Provider;
    use crate::domain::translation::{TranslationRequest, TranslationResult};
    use crate::infrastructure::storage::SqliteConfigStore;
    use crate::Result;
    use async_trait::async_trait;
    use std::sync::Arc; // Still needed for Arc<SqliteConfigStore>

    // Mock provider for testing
    struct MockTranslationProvider {
        id: String,
        name: String,
        response_text: String,
    }

    struct FailingTranslationProvider {
        id: String,
        name: String,
    }

    impl MockTranslationProvider {
        fn new(id: &str, name: &str) -> Self {
            Self {
                id: id.to_string(),
                name: name.to_string(),
                response_text: "translated".to_string(),
            }
        }

        fn with_response(id: &str, name: &str, response_text: &str) -> Self {
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
                provider_id: self.id().to_string(),
                translated_text: self.response_text.clone(),
                detected_language: Some("en".to_string()),
                confidence: Some(1.0),
            })
        }
    }

    impl Provider for FailingTranslationProvider {
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
    impl TranslationProvider for FailingTranslationProvider {
        async fn translate(&self, _request: &TranslationRequest) -> Result<TranslationResult> {
            Err("upstream rejected request".into())
        }
    }

    fn sample_request() -> TranslationRequest {
        TranslationRequest {
            text: "Hello".to_string(),
            source_lang: "en".to_string(),
            target_lang: "es".to_string(),
        }
    }

    // ---- Registration & activation tests ----

    #[test]
    fn test_register_provider() {
        let config = Arc::new(SqliteConfigStore::new_temp());
        let coordinator = TranslationCoordinator::new(config);
        let provider = MockTranslationProvider::new("google", "Google Translate");

        coordinator.register(provider).unwrap();

        let all = coordinator.list_all();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].read().id(), "google");
    }

    #[test]
    fn test_register_duplicate_provider_fails() {
        let config = Arc::new(SqliteConfigStore::new_temp());
        let coordinator = TranslationCoordinator::new(config);

        coordinator
            .register(MockTranslationProvider::new("google", "Google"))
            .unwrap();
        let result = coordinator.register(MockTranslationProvider::new("google", "Google"));

        assert!(result.is_err());
    }

    #[test]
    fn test_replace_provider_preserves_active_order() {
        let config = Arc::new(SqliteConfigStore::new_temp());
        let coordinator = TranslationCoordinator::new(config);
        coordinator
            .register(MockTranslationProvider::new("google", "Google Translate"))
            .unwrap();
        coordinator
            .register(MockTranslationProvider::new("custom-gpt", "gpt-5-mini"))
            .unwrap();
        coordinator.activate("google").unwrap();
        coordinator.activate("custom-gpt").unwrap();

        coordinator
            .replace(MockTranslationProvider::new(
                "custom-gpt",
                "gpt-5-mini updated",
            ))
            .unwrap();

        let active = coordinator.get_active();
        let active_names: Vec<_> = active
            .iter()
            .map(|provider| provider.read().name().to_string())
            .collect();
        assert_eq!(
            active_names,
            vec![
                "Google Translate".to_string(),
                "gpt-5-mini updated".to_string()
            ]
        );
    }

    #[test]
    fn test_activate_multiple_providers() {
        let config = Arc::new(SqliteConfigStore::new_temp());
        let coordinator = TranslationCoordinator::new(config);
        coordinator
            .register(MockTranslationProvider::new("google", "Google Translate"))
            .unwrap();
        coordinator
            .register(MockTranslationProvider::new("deepl", "DeepL"))
            .unwrap();

        coordinator.activate("google").unwrap();
        coordinator.activate("deepl").unwrap();

        let active = coordinator.get_active();
        assert_eq!(active.len(), 2);
        assert_eq!(active[0].read().id(), "google");
        assert_eq!(active[1].read().id(), "deepl");
    }

    #[test]
    fn test_deactivate_provider() {
        let config = Arc::new(SqliteConfigStore::new_temp());
        let coordinator = TranslationCoordinator::new(config);
        coordinator
            .register(MockTranslationProvider::new("google", "Google Translate"))
            .unwrap();

        coordinator.activate("google").unwrap();
        assert_eq!(coordinator.get_active().len(), 1);

        coordinator.deactivate("google").unwrap();
        assert_eq!(coordinator.get_active().len(), 0);
    }

    #[test]
    fn test_activate_nonexistent_provider() {
        let config = Arc::new(SqliteConfigStore::new_temp());
        let coordinator = TranslationCoordinator::new(config);

        let result = coordinator.activate("nonexistent");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Provider not found: nonexistent"
        );
    }

    // ---- Persistence tests ----

    #[test]
    fn test_activate_persists() {
        let config = Arc::new(SqliteConfigStore::new_temp());
        let coordinator = TranslationCoordinator::new(config.clone());
        coordinator
            .register(MockTranslationProvider::new("google", "Google"))
            .unwrap();

        coordinator.activate("google").unwrap();

        let saved: Vec<String> = config.load("active_translation_providers").unwrap();
        assert_eq!(saved, vec!["google".to_string()]);
    }

    #[test]
    fn test_restore_from_config_skips_unregistered() {
        let config = Arc::new(SqliteConfigStore::new_temp());
        config
            .save(
                "active_translation_providers",
                &vec!["google".to_string(), "ghost".to_string()],
            )
            .unwrap();

        let coordinator = TranslationCoordinator::new(config);
        coordinator
            .register(MockTranslationProvider::new("google", "Google"))
            .unwrap();

        coordinator.restore_from_config().unwrap();

        let active = coordinator.get_active();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].read().id(), "google");
    }

    #[test]
    fn test_restore_from_config_maps_legacy_deepl_to_deeplx() {
        let config = Arc::new(SqliteConfigStore::new_temp());
        config
            .save(
                "active_translation_providers",
                &vec!["google".to_string(), "deepl".to_string()],
            )
            .unwrap();

        let coordinator = TranslationCoordinator::new(config);
        coordinator
            .register(MockTranslationProvider::new("google", "Google"))
            .unwrap();
        coordinator
            .register(MockTranslationProvider::new("deeplx", "DeepLX"))
            .unwrap();

        coordinator.restore_from_config().unwrap();

        let active = coordinator.get_active();
        let active_ids: Vec<_> = active
            .iter()
            .map(|provider| provider.read().id().to_string())
            .collect();
        assert_eq!(active_ids, vec!["google".to_string(), "deeplx".to_string()]);
    }

    // ---- Translation execution tests ----

    #[tokio::test]
    async fn test_translate_with_single_provider() {
        let config = Arc::new(SqliteConfigStore::new_temp());
        let coordinator = TranslationCoordinator::new(config);
        coordinator
            .register(MockTranslationProvider::with_response(
                "google",
                "Google Translate",
                "Hola",
            ))
            .unwrap();
        coordinator.activate("google").unwrap();

        let results = coordinator.translate(&sample_request()).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].translated_text, "Hola");
    }

    #[tokio::test]
    async fn test_translate_with_specific_provider() {
        let config = Arc::new(SqliteConfigStore::new_temp());
        let coordinator = TranslationCoordinator::new(config);
        coordinator
            .register(MockTranslationProvider::with_response(
                "google",
                "Google Translate",
                "Hola (Google)",
            ))
            .unwrap();
        coordinator
            .register(MockTranslationProvider::with_response(
                "deepl",
                "DeepL",
                "Hola (DeepL)",
            ))
            .unwrap();

        let result = coordinator
            .translate_with_provider("deepl", &sample_request())
            .await
            .unwrap();

        assert_eq!(result.provider_id, "deepl");
        assert_eq!(result.translated_text, "Hola (DeepL)");
    }

    #[tokio::test]
    async fn test_translate_with_specific_provider_reports_missing_provider() {
        let config = Arc::new(SqliteConfigStore::new_temp());
        let coordinator = TranslationCoordinator::new(config);

        let result = coordinator
            .translate_with_provider("ghost", &sample_request())
            .await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Provider not found: ghost");
    }

    #[tokio::test]
    async fn test_translate_with_multiple_providers() {
        let config = Arc::new(SqliteConfigStore::new_temp());
        let coordinator = TranslationCoordinator::new(config);
        coordinator
            .register(MockTranslationProvider::with_response(
                "google",
                "Google Translate",
                "Hola (Google)",
            ))
            .unwrap();
        coordinator
            .register(MockTranslationProvider::with_response(
                "deepl",
                "DeepL",
                "Hola (DeepL)",
            ))
            .unwrap();
        coordinator.activate("google").unwrap();
        coordinator.activate("deepl").unwrap();

        let results = coordinator.translate(&sample_request()).await.unwrap();

        assert_eq!(results.len(), 2);
        let texts: Vec<String> = results.iter().map(|r| r.translated_text.clone()).collect();
        assert!(texts.contains(&"Hola (Google)".to_string()));
        assert!(texts.contains(&"Hola (DeepL)".to_string()));
    }

    #[tokio::test]
    async fn test_translate_includes_active_provider_failure_results() {
        let config = Arc::new(SqliteConfigStore::new_temp());
        let coordinator = TranslationCoordinator::new(config);
        coordinator
            .register(MockTranslationProvider::with_response(
                "google",
                "Google Translate",
                "Hola (Google)",
            ))
            .unwrap();
        coordinator
            .register(FailingTranslationProvider {
                id: "custom-gpt".to_string(),
                name: "gpt-5-mini".to_string(),
            })
            .unwrap();
        coordinator.activate("google").unwrap();
        coordinator.activate("custom-gpt").unwrap();

        let results = coordinator.translate(&sample_request()).await.unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].provider_id, "google");
        assert_eq!(results[0].translated_text, "Hola (Google)");
        assert_eq!(results[1].provider_id, "custom-gpt");
        assert!(results[1].translated_text.contains("Translation failed"));
        assert!(results[1]
            .translated_text
            .contains("upstream rejected request"));
    }

    #[tokio::test]
    async fn test_translate_with_no_active_provider() {
        let config = Arc::new(SqliteConfigStore::new_temp());
        let coordinator = TranslationCoordinator::new(config);

        let result = coordinator.translate(&sample_request()).await;

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "No active translation providers"
        );
    }

    #[tokio::test]
    async fn test_coordinator_publishes_event_when_translation_completes() {
        use crate::application::history::EventSubscriber;
        use crate::domain::events::DomainEvent;
        use crate::infrastructure::events::EventBus;
        use tokio::sync::Mutex as TokioMutex;

        // Mock subscriber to capture events
        struct MockSubscriber {
            events: Arc<TokioMutex<Vec<DomainEvent>>>,
        }

        #[async_trait]
        impl EventSubscriber for MockSubscriber {
            async fn handle(&self, event: &DomainEvent) {
                self.events.lock().await.push(event.clone());
            }
            fn name(&self) -> &str {
                "mock"
            }
        }

        // Arrange
        let config = Arc::new(SqliteConfigStore::new_temp());
        let event_bus = Arc::new(EventBus::new());
        let subscriber = Arc::new(MockSubscriber {
            events: Arc::new(TokioMutex::new(Vec::new())),
        });

        event_bus.subscribe(subscriber.clone()).await;

        let coordinator = TranslationCoordinator::new(config);
        coordinator
            .register(MockTranslationProvider::with_response(
                "google",
                "Google Translate",
                "Hola",
            ))
            .unwrap();
        coordinator.activate("google").unwrap();

        // Inject event bus
        let coordinator = coordinator.with_event_bus(event_bus);

        // Act
        let _results = coordinator.translate(&sample_request()).await.unwrap();

        // Wait for async event processing
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Assert - event was published
        let events = subscriber.events.lock().await;
        assert_eq!(events.len(), 1);

        match &events[0] {
            DomainEvent::TranslationCompleted {
                request,
                results,
                providers_used,
                ..
            } => {
                assert_eq!(request.text, "Hello");
                assert_eq!(results.len(), 1);
                assert_eq!(results[0].translated_text, "Hola");
                assert_eq!(providers_used.len(), 1);
                assert_eq!(providers_used[0], "google");
            }
            _ => panic!("Expected TranslationCompleted event"),
        }
    }

    #[tokio::test]
    async fn test_unregister_provider() {
        // Arrange
        let config = Arc::new(SqliteConfigStore::new_temp());
        let coordinator = TranslationCoordinator::new(config);

        coordinator
            .register(MockTranslationProvider::new("google", "Google"))
            .unwrap();

        // Act
        coordinator.unregister("google").unwrap();

        // Assert
        assert!(coordinator.get("google").is_none());
        assert_eq!(coordinator.list_all().len(), 0);
    }

    #[tokio::test]
    async fn test_unregister_active_provider() {
        // Arrange
        let config = Arc::new(SqliteConfigStore::new_temp());
        let coordinator = TranslationCoordinator::new(config);

        coordinator
            .register(MockTranslationProvider::new("google", "Google"))
            .unwrap();
        coordinator.activate("google").unwrap();

        // Act
        coordinator.unregister("google").unwrap();

        // Assert - provider removed and deactivated
        assert!(coordinator.get("google").is_none());
        assert_eq!(coordinator.get_active().len(), 0);
    }

    #[tokio::test]
    async fn test_unregister_nonexistent_provider_fails() {
        // Arrange
        let config = Arc::new(SqliteConfigStore::new_temp());
        let coordinator = TranslationCoordinator::new(config);

        // Act & Assert
        assert!(coordinator.unregister("nonexistent").is_err());
    }
}

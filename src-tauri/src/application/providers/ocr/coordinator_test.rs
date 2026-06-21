#[cfg(test)]
mod tests {
    use super::super::coordinator::OcrCoordinator;
    use super::super::OcrProvider;
    use crate::application::providers::common::Provider;
    use crate::domain::ocr::{OcrRequest, OcrResult};
    use crate::infrastructure::storage::ConfigFile;
    use crate::Result;
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};

    // Mock OCR Provider for testing
    struct MockOcrProvider {
        id: String,
        name: String,
        text_to_return: String,
        observed_request: Option<Arc<Mutex<Option<OcrRequest>>>>,
    }

    impl MockOcrProvider {
        fn new(id: &str, name: &str) -> Self {
            Self {
                id: id.to_string(),
                name: name.to_string(),
                text_to_return: "recognized".to_string(),
                observed_request: None,
            }
        }

        fn with_text(id: &str, name: &str, text_to_return: &str) -> Self {
            Self {
                id: id.to_string(),
                name: name.to_string(),
                text_to_return: text_to_return.to_string(),
                observed_request: None,
            }
        }

        fn with_request_recorder(
            id: &str,
            name: &str,
            observed_request: Arc<Mutex<Option<OcrRequest>>>,
        ) -> Self {
            Self {
                id: id.to_string(),
                name: name.to_string(),
                text_to_return: "recognized".to_string(),
                observed_request: Some(observed_request),
            }
        }
    }

    impl Provider for MockOcrProvider {
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

        fn reconfigure_credentials(
            &mut self,
            credentials: &std::collections::HashMap<String, String>,
        ) -> Result<()> {
            self.text_to_return = credentials
                .get("text_to_return")
                .cloned()
                .unwrap_or_else(|| self.text_to_return.clone());
            Ok(())
        }
    }

    #[async_trait]
    impl OcrProvider for MockOcrProvider {
        async fn recognize(&self, request: &OcrRequest) -> Result<OcrResult> {
            if let Some(observed_request) = &self.observed_request {
                *observed_request.lock().unwrap() = Some(request.clone());
            }

            Ok(OcrResult {
                text: self.text_to_return.clone(),
                confidence: Some(0.95),
            })
        }
    }

    fn sample_request() -> OcrRequest {
        OcrRequest {
            image_data: vec![1, 2, 3],
            language: None,
        }
    }

    // ---- Registration & activation tests ----

    #[test]
    fn test_register_provider() {
        let config = Arc::new(ConfigFile::new_temp());
        let coordinator = OcrCoordinator::new(config);
        coordinator
            .register(MockOcrProvider::new("tesseract", "Tesseract OCR"))
            .unwrap();

        let all = coordinator.list_all();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].read().id(), "tesseract");
    }

    #[test]
    fn test_activate_single_provider() {
        let config = Arc::new(ConfigFile::new_temp());
        let coordinator = OcrCoordinator::new(config);
        coordinator
            .register(MockOcrProvider::new("tesseract", "Tesseract OCR"))
            .unwrap();

        coordinator.activate("tesseract").unwrap();

        let active = coordinator.get_active();
        assert!(active.is_some());
        assert_eq!(active.unwrap().read().id(), "tesseract");
    }

    #[test]
    fn test_switch_provider() {
        let config = Arc::new(ConfigFile::new_temp());
        let coordinator = OcrCoordinator::new(config);
        coordinator
            .register(MockOcrProvider::new("tesseract", "Tesseract OCR"))
            .unwrap();
        coordinator
            .register(MockOcrProvider::new("baidu", "Baidu OCR"))
            .unwrap();

        // Activate first provider
        coordinator.activate("tesseract").unwrap();
        assert_eq!(coordinator.get_active().unwrap().read().id(), "tesseract");

        // Switch to second provider (should replace the first)
        coordinator.activate("baidu").unwrap();
        assert_eq!(coordinator.get_active().unwrap().read().id(), "baidu");
    }

    #[test]
    fn test_activate_nonexistent_provider() {
        let config = Arc::new(ConfigFile::new_temp());
        let coordinator = OcrCoordinator::new(config);

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
        let config = Arc::new(ConfigFile::new_temp());
        let coordinator = OcrCoordinator::new(config.clone());
        coordinator
            .register(MockOcrProvider::new("tesseract", "Tesseract"))
            .unwrap();

        coordinator.activate("tesseract").unwrap();

        let saved: String = config.load("active_ocr_provider").unwrap();
        assert_eq!(saved, "tesseract");
    }

    #[test]
    fn test_restore_from_config_skips_unregistered() {
        let config = Arc::new(ConfigFile::new_temp());
        config.save("active_ocr_provider", &"ghost".to_string()).unwrap();

        let coordinator = OcrCoordinator::new(config);
        coordinator
            .register(MockOcrProvider::new("tesseract", "Tesseract"))
            .unwrap();

        coordinator.restore_from_config().unwrap();

        // "ghost" is not registered, so nothing should be active
        assert!(coordinator.get_active().is_none());
    }

    // ---- Recognition execution tests ----

    #[tokio::test]
    async fn test_recognize_with_active_provider() {
        let config = Arc::new(ConfigFile::new_temp());
        let coordinator = OcrCoordinator::new(config);
        coordinator
            .register(MockOcrProvider::with_text(
                "tesseract",
                "Tesseract OCR",
                "Hello World",
            ))
            .unwrap();
        coordinator.activate("tesseract").unwrap();

        let result = coordinator.recognize(&sample_request()).await.unwrap();

        assert_eq!(result.text, "Hello World");
        assert_eq!(result.confidence, Some(0.95));
    }

    #[tokio::test]
    async fn test_reconfigure_provider_updates_runtime_provider() {
        let config = Arc::new(ConfigFile::new_temp());
        let coordinator = OcrCoordinator::new(config);
        coordinator
            .register(MockOcrProvider::with_text(
                "mock",
                "Mock OCR",
                "before",
            ))
            .unwrap();
        coordinator.activate("mock").unwrap();

        let mut credentials = std::collections::HashMap::new();
        credentials.insert("text_to_return".to_string(), "after".to_string());

        coordinator
            .reconfigure_provider("mock", &credentials)
            .unwrap();

        let result = coordinator.recognize(&sample_request()).await.unwrap();

        assert_eq!(result.text, "after");
    }

    #[tokio::test]
    async fn test_recognize_image_builds_default_request() {
        let config = Arc::new(ConfigFile::new_temp());
        let coordinator = OcrCoordinator::new(config);
        let observed_request = Arc::new(Mutex::new(None));
        coordinator
            .register(MockOcrProvider::with_request_recorder(
                "tesseract",
                "Tesseract OCR",
                observed_request.clone(),
            ))
            .unwrap();
        coordinator.activate("tesseract").unwrap();

        let result = coordinator.recognize_image(vec![7, 8, 9]).await.unwrap();

        let request = observed_request.lock().unwrap().clone().unwrap();
        assert_eq!(result.text, "recognized");
        assert_eq!(request.image_data, vec![7, 8, 9]);
        assert_eq!(request.language, None);
    }

    #[tokio::test]
    async fn test_recognize_with_no_active_provider() {
        let config = Arc::new(ConfigFile::new_temp());
        let coordinator = OcrCoordinator::new(config);

        let result = coordinator.recognize(&sample_request()).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "No active OCR provider");
    }
}

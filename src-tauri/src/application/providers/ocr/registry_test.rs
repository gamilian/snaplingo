#[cfg(test)]
mod tests {
    use super::super::registry::OcrRegistry;
    use super::super::OcrProvider;
    use crate::application::providers::common::Provider;
    use crate::domain::ocr::{OcrRequest, OcrResult};
    use crate::Result;
    use async_trait::async_trait;
    use std::sync::Arc;

    // Mock provider for testing
    struct MockOcrProvider {
        id: String,
        name: String,
    }

    impl MockOcrProvider {
        fn new(id: &str, name: &str) -> Self {
            Self {
                id: id.to_string(),
                name: name.to_string(),
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
    }

    #[async_trait]
    impl OcrProvider for MockOcrProvider {
        async fn recognize(&self, _request: &OcrRequest) -> Result<OcrResult> {
            Ok(OcrResult {
                text: "recognized".to_string(),
                confidence: Some(0.95),
            })
        }
    }

    #[test]
    fn test_register_provider() {
        let mut registry = OcrRegistry::new();
        let provider = Arc::new(MockOcrProvider::new("tesseract", "Tesseract OCR"));

        registry.register(provider.clone()).unwrap();

        let all_providers = registry.list_all();
        assert_eq!(all_providers.len(), 1);
        assert_eq!(all_providers[0].id(), "tesseract");
    }

    #[test]
    fn test_activate_single_provider() {
        let mut registry = OcrRegistry::new();
        let provider = Arc::new(MockOcrProvider::new("tesseract", "Tesseract OCR"));

        registry.register(provider.clone()).unwrap();
        registry.activate("tesseract").unwrap();

        let active = registry.get_active();
        assert!(active.is_some());
        assert_eq!(active.unwrap().id(), "tesseract");
    }

    #[test]
    fn test_switch_provider() {
        let mut registry = OcrRegistry::new();
        let provider1 = Arc::new(MockOcrProvider::new("tesseract", "Tesseract OCR"));
        let provider2 = Arc::new(MockOcrProvider::new("baidu", "Baidu OCR"));

        registry.register(provider1.clone()).unwrap();
        registry.register(provider2.clone()).unwrap();

        // Activate first provider
        registry.activate("tesseract").unwrap();
        let active = registry.get_active();
        assert!(active.is_some());
        assert_eq!(active.unwrap().id(), "tesseract");

        // Switch to second provider (should replace the first)
        registry.activate("baidu").unwrap();
        let active_after = registry.get_active();
        assert!(active_after.is_some());
        assert_eq!(active_after.unwrap().id(), "baidu");
    }

    #[test]
    fn test_activate_nonexistent_provider() {
        let mut registry = OcrRegistry::new();

        let result = registry.activate("nonexistent");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Provider not found: nonexistent"
        );
    }
}

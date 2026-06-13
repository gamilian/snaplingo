#[cfg(test)]
mod tests {
    use super::super::service::OcrService;
    use super::super::registry::OcrRegistry;
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
    }

    impl MockOcrProvider {
        fn new(id: &str, name: &str, text_to_return: &str) -> Self {
            Self {
                id: id.to_string(),
                name: name.to_string(),
                text_to_return: text_to_return.to_string(),
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
                text: self.text_to_return.clone(),
                confidence: Some(0.95),
            })
        }
    }

    #[tokio::test]
    async fn test_recognize_with_active_provider() {
        // Arrange
        let config = Arc::new(ConfigFile::new_temp());
        let mut registry = OcrRegistry::new(config);
        let provider = Arc::new(MockOcrProvider::new(
            "tesseract",
            "Tesseract OCR",
            "Hello World",
        ));

        registry.register(provider).unwrap();
        registry.activate("tesseract").unwrap();

        let service = OcrService::new(Arc::new(Mutex::new(registry)));
        let request = OcrRequest {
            image_data: vec![1, 2, 3],
            language: None,
        };

        // Act
        let result = service.recognize(&request).await;

        // Assert
        assert!(result.is_ok());
        let ocr_result = result.unwrap();
        assert_eq!(ocr_result.text, "Hello World");
        assert_eq!(ocr_result.confidence, Some(0.95));
    }

    #[tokio::test]
    async fn test_recognize_with_no_active_provider() {
        // Arrange
        let config = Arc::new(ConfigFile::new_temp());
        let registry = OcrRegistry::new(config);
        let service = OcrService::new(Arc::new(Mutex::new(registry)));
        let request = OcrRequest {
            image_data: vec![1, 2, 3],
            language: None,
        };

        // Act
        let result = service.recognize(&request).await;

        // Assert
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "No active OCR provider");
    }

    #[tokio::test]
    async fn test_recognize_updates_provider_in_result() {
        // Arrange
        let config = Arc::new(ConfigFile::new_temp());
        let mut registry = OcrRegistry::new(config);
        let provider = Arc::new(MockOcrProvider::new(
            "google-vision",
            "Google Vision API",
            "Test Text",
        ));

        registry.register(provider).unwrap();
        registry.activate("google-vision").unwrap();

        let service = OcrService::new(Arc::new(Mutex::new(registry)));
        let request = OcrRequest {
            image_data: vec![4, 5, 6],
            language: Some("en".to_string()),
        };

        // Act
        let result = service.recognize(&request).await;

        // Assert
        assert!(result.is_ok());
        let ocr_result = result.unwrap();
        assert_eq!(ocr_result.text, "Test Text");
        assert_eq!(ocr_result.confidence, Some(0.95));
    }
}

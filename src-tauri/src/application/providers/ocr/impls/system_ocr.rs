use crate::application::providers::common::Provider;
use crate::application::providers::ocr::{OcrProvider, SystemOcrEngine};
use crate::domain::ocr::{OcrRequest, OcrResult};
use crate::Result;
use async_trait::async_trait;
use std::sync::Arc;

#[derive(Clone)]
pub struct SystemOcrProvider {
    engine: Arc<dyn SystemOcrEngine>,
}

impl SystemOcrProvider {
    pub(crate) fn new(engine: Arc<dyn SystemOcrEngine>) -> Self {
        Self { engine }
    }
}

impl Provider for SystemOcrProvider {
    fn id(&self) -> &str {
        "system-ocr"
    }

    fn name(&self) -> &str {
        "System OCR"
    }

    fn is_configured(&self) -> bool {
        true
    }

    fn requires_api_key(&self) -> bool {
        false
    }
}

#[async_trait]
impl OcrProvider for SystemOcrProvider {
    async fn recognize(&self, request: &OcrRequest) -> Result<OcrResult> {
        self.engine.recognize(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubSystemOcrEngine;

    impl SystemOcrEngine for StubSystemOcrEngine {
        fn recognize(&self, request: &OcrRequest) -> Result<OcrResult> {
            Ok(OcrResult {
                text: format!("{} bytes", request.image_data.len()),
                confidence: Some(0.9),
            })
        }
    }

    #[test]
    fn system_ocr_provider_is_local_and_ready_without_credentials() {
        let provider = SystemOcrProvider::new(Arc::new(StubSystemOcrEngine));

        assert_eq!(provider.id(), "system-ocr");
        assert_eq!(provider.name(), "System OCR");
        assert!(provider.is_configured());
        assert!(!provider.requires_api_key());
    }

    #[tokio::test]
    async fn system_ocr_provider_delegates_recognition_to_infrastructure_engine() {
        let provider = SystemOcrProvider::new(Arc::new(StubSystemOcrEngine));
        let request = OcrRequest {
            image_data: vec![1, 2, 3],
            language: Some("en".to_string()),
        };

        let result = provider.recognize(&request).await.unwrap();

        assert_eq!(result.text, "3 bytes");
        assert_eq!(result.confidence, Some(0.9));
    }
}

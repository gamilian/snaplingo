use crate::application::providers::common::Provider;
use crate::application::providers::ocr::{OcrProvider, SystemOcrEngine};
use crate::domain::ocr::{OcrRequest, OcrResult};
use crate::Result;
use async_trait::async_trait;
use std::sync::Arc;

#[derive(Clone)]
pub struct SystemOcrProvider {
    engine: Arc<dyn SystemOcrEngine>,
    is_available: bool,
}

impl SystemOcrProvider {
    pub(crate) fn new(engine: Arc<dyn SystemOcrEngine>) -> Self {
        let is_available = engine.is_available();
        Self {
            engine,
            is_available,
        }
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
        self.is_available
    }

    fn requires_api_key(&self) -> bool {
        false
    }

    fn is_local(&self) -> bool {
        true
    }
}

#[async_trait]
impl OcrProvider for SystemOcrProvider {
    async fn recognize(&self, request: &OcrRequest) -> Result<OcrResult> {
        let engine = self.engine.clone();
        let request = request.clone();
        tokio::task::spawn_blocking(move || engine.recognize(&request))
            .await
            .map_err(|error| crate::AppError::System(format!("System OCR task failed: {error}")))?
    }

    async fn recognize_with_recovery(&self, request: &OcrRequest) -> Result<OcrResult> {
        let engine = self.engine.clone();
        let request = request.clone();
        tokio::task::spawn_blocking(move || engine.recognize_with_recovery(&request))
            .await
            .map_err(|error| crate::AppError::System(format!("System OCR task failed: {error}")))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubSystemOcrEngine {
        available: bool,
    }

    impl SystemOcrEngine for StubSystemOcrEngine {
        fn is_available(&self) -> bool {
            self.available
        }

        fn recognize(&self, request: &OcrRequest) -> Result<OcrResult> {
            Ok(OcrResult::from_text(
                format!("{} bytes", request.image_data.len()),
                Some(0.9),
            ))
        }
    }

    #[test]
    fn system_ocr_provider_is_local_and_ready_when_engine_is_available() {
        let provider = SystemOcrProvider::new(Arc::new(StubSystemOcrEngine { available: true }));

        assert_eq!(provider.id(), "system-ocr");
        assert_eq!(provider.name(), "System OCR");
        assert!(provider.is_configured());
        assert!(!provider.requires_api_key());
    }

    #[test]
    fn system_ocr_provider_is_not_configured_when_engine_is_unavailable() {
        let provider = SystemOcrProvider::new(Arc::new(StubSystemOcrEngine { available: false }));

        assert!(!provider.is_configured());
    }

    #[tokio::test]
    async fn system_ocr_provider_delegates_recognition_to_infrastructure_engine() {
        let provider = SystemOcrProvider::new(Arc::new(StubSystemOcrEngine { available: true }));
        let request = OcrRequest {
            image_data: vec![1, 2, 3],
            language: Some("en".to_string()),
        };

        let result = provider.recognize(&request).await.unwrap();

        assert_eq!(result.text, "3 bytes");
        assert_eq!(result.confidence, Some(0.9));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn native_ocr_does_not_block_other_async_work() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::time::{Duration, Instant};

        struct WaitingEngine(Arc<AtomicBool>);
        impl SystemOcrEngine for WaitingEngine {
            fn is_available(&self) -> bool {
                true
            }
            fn recognize(&self, _: &OcrRequest) -> Result<OcrResult> {
                let started = Instant::now();
                while !self.0.load(Ordering::SeqCst) {
                    if started.elapsed() > Duration::from_millis(200) {
                        return Err("OCR blocked the async runtime".into());
                    }
                    std::thread::sleep(Duration::from_millis(1));
                }
                Ok(OcrResult::from_text("recognized", None))
            }
        }

        for recovery in [false, true] {
            let progressed = Arc::new(AtomicBool::new(false));
            let provider = SystemOcrProvider::new(Arc::new(WaitingEngine(progressed.clone())));
            let heartbeat = tokio::spawn(async move {
                progressed.store(true, Ordering::SeqCst);
            });
            let request = OcrRequest {
                image_data: vec![],
                language: None,
            };
            let result = if recovery {
                provider.recognize_with_recovery(&request).await
            } else {
                provider.recognize(&request).await
            };
            heartbeat.await.unwrap();
            assert_eq!(result.unwrap().text, "recognized");
        }
    }
}

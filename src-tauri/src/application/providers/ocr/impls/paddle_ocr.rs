use std::sync::Arc;

use async_trait::async_trait;

use crate::application::providers::common::Provider;
use crate::application::providers::ocr::{OcrAccelerationStatus, OcrProvider, PaddleOcrEngine};
use crate::application::providers::ProviderChangeNotifier;
use crate::domain::ocr::{OcrRequest, OcrResult};
use crate::{Result, SettingsConfiguration};

pub struct PaddleOcrProvider {
    engine: Arc<dyn PaddleOcrEngine>,
    settings: Arc<SettingsConfiguration>,
    change_notifier: Arc<dyn ProviderChangeNotifier>,
}

impl PaddleOcrProvider {
    pub(crate) fn new(
        engine: Arc<dyn PaddleOcrEngine>,
        settings: Arc<SettingsConfiguration>,
        change_notifier: Arc<dyn ProviderChangeNotifier>,
    ) -> Self {
        Self {
            engine,
            settings,
            change_notifier,
        }
    }
}

impl Provider for PaddleOcrProvider {
    fn id(&self) -> &str {
        "paddle-ocr"
    }
    fn name(&self) -> &str {
        "PaddleOCR"
    }
    fn is_configured(&self) -> bool {
        self.engine.is_available()
    }
    fn requires_api_key(&self) -> bool {
        false
    }
    fn is_local(&self) -> bool {
        true
    }
}

#[async_trait]
impl OcrProvider for PaddleOcrProvider {
    fn acceleration_status(&self) -> Option<OcrAccelerationStatus> {
        let mut status = self.engine.status();
        if !self.engine.supports_gpu() {
            status.gpu_enabled = false;
            return Some(status);
        }
        let enabled = self.settings.snapshot().ok()?.ocr.gpu_acceleration;
        if status.gpu_enabled != enabled {
            status = OcrAccelerationStatus {
                gpu_enabled: enabled,
                device: None,
                fallback_reason: None,
            };
        }
        Some(status)
    }

    async fn recognize(&self, request: &OcrRequest) -> Result<OcrResult> {
        let before = self.engine.status();
        let gpu_enabled =
            self.engine.supports_gpu() && self.settings.snapshot()?.ocr.gpu_acceleration;
        let result = self.engine.recognize(request, gpu_enabled).await;
        if before != self.engine.status() {
            self.change_notifier.providers_changed();
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::providers::ocr::{OcrCoordinator, OcrExecutionDevice};
    use crate::domain::OcrSettings;
    use crate::infrastructure::storage::SqliteConfigStore;
    use parking_lot::Mutex;

    struct StubEngine {
        calls: Mutex<Vec<bool>>,
        status: Mutex<OcrAccelerationStatus>,
        gpu_supported: bool,
    }

    #[async_trait]
    impl PaddleOcrEngine for StubEngine {
        fn is_available(&self) -> bool {
            true
        }
        fn supports_gpu(&self) -> bool {
            self.gpu_supported
        }
        async fn recognize(&self, _: &OcrRequest, gpu_enabled: bool) -> Result<OcrResult> {
            self.calls.lock().push(gpu_enabled);
            *self.status.lock() = OcrAccelerationStatus {
                gpu_enabled,
                device: Some(if gpu_enabled {
                    OcrExecutionDevice::Gpu
                } else {
                    OcrExecutionDevice::Cpu
                }),
                fallback_reason: None,
            };
            Ok(OcrResult::from_text("English 中文", Some(0.9)))
        }
        fn status(&self) -> OcrAccelerationStatus {
            self.status.lock().clone()
        }
    }

    struct NoopNotifier;
    impl ProviderChangeNotifier for NoopNotifier {
        fn providers_changed(&self) {}
    }

    #[tokio::test]
    async fn provider_uses_live_persisted_settings_through_the_existing_coordinator() {
        let store = Arc::new(SqliteConfigStore::new_temp());
        let settings = Arc::new(SettingsConfiguration::new(store.clone()));
        let engine = Arc::new(StubEngine {
            gpu_supported: true,
            calls: Mutex::new(vec![]),
            status: Mutex::new(OcrAccelerationStatus {
                gpu_enabled: true,
                device: None,
                fallback_reason: None,
            }),
        });
        let provider =
            PaddleOcrProvider::new(engine.clone(), settings.clone(), Arc::new(NoopNotifier));
        assert!(provider.is_local());
        assert!(provider.is_configured());
        assert!(provider.credential_fields().is_empty());
        let coordinator = OcrCoordinator::new(store);
        coordinator.register(provider).unwrap();
        coordinator.activate("paddle-ocr").unwrap();
        let request = OcrRequest {
            image_data: vec![1],
            language: None,
        };
        settings
            .update_ocr(OcrSettings {
                gpu_acceleration: false,
                ..OcrSettings::default()
            })
            .unwrap();
        let result = coordinator.recognize(&request).await.unwrap();
        assert_eq!(result.provider_id.as_deref(), Some("paddle-ocr"));
        assert_eq!(result.detected_language.as_deref(), Some("zh"));
        settings.update_ocr(OcrSettings::default()).unwrap();
        coordinator.recognize(&request).await.unwrap();
        assert_eq!(*engine.calls.lock(), vec![false, true]);
        assert_eq!(
            coordinator
                .get_active()
                .unwrap()
                .acceleration_status()
                .unwrap()
                .device,
            Some(OcrExecutionDevice::Gpu)
        );
    }

    #[test]
    fn existing_settings_default_to_gpu_enabled_and_preserve_an_explicit_disable() {
        let old: OcrSettings = serde_json::from_str("{}").unwrap();
        assert!(old.gpu_acceleration);
        let settings = OcrSettings {
            gpu_acceleration: false,
            ..old
        };
        let restored: OcrSettings =
            serde_json::from_value(serde_json::to_value(settings).unwrap()).unwrap();
        assert!(!restored.gpu_acceleration);
    }

    #[tokio::test]
    async fn cpu_only_engine_ignores_gpu_preference_without_changing_saved_settings() {
        let store = Arc::new(SqliteConfigStore::new_temp());
        let settings = Arc::new(SettingsConfiguration::new(store));
        let engine = Arc::new(StubEngine {
            gpu_supported: false,
            calls: Mutex::new(vec![]),
            status: Mutex::new(OcrAccelerationStatus {
                gpu_enabled: false,
                device: Some(OcrExecutionDevice::Cpu),
                fallback_reason: Some("tiny CPU".into()),
            }),
        });
        let provider =
            PaddleOcrProvider::new(engine.clone(), settings.clone(), Arc::new(NoopNotifier));
        assert!(settings.snapshot().unwrap().ocr.gpu_acceleration);
        assert_eq!(
            provider.acceleration_status().unwrap().device,
            Some(OcrExecutionDevice::Cpu)
        );
        provider
            .recognize(&OcrRequest {
                image_data: vec![1],
                language: None,
            })
            .await
            .unwrap();
        assert_eq!(*engine.calls.lock(), vec![false]);
        assert!(!provider.acceleration_status().unwrap().gpu_enabled);
        assert!(settings.snapshot().unwrap().ocr.gpu_acceleration);
    }
}

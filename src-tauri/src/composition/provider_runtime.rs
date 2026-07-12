use std::collections::HashMap;
use std::sync::Arc;

#[cfg(target_os = "macos")]
use crate::application::providers::ocr::impls::SystemOcrProvider;
use crate::application::providers::ocr::{
    impls::{BaiduOcrProvider, TesseractProvider},
    OcrCoordinator,
};
use crate::application::providers::translation::{
    BaiduTranslateProvider, DeepLProvider, GoogleTranslateProvider as GoogleTranslateProviderV2,
    TranslationCoordinator,
};
use crate::application::providers::{LlmIntrospection, ProviderConfiguration};
use crate::infrastructure::events::EventBus;
use crate::infrastructure::http::HttpClient;
use crate::infrastructure::storage::{ConfigFile, Keychain};
use crate::infrastructure::system::ocr::get_tesseract_engine;
#[cfg(target_os = "macos")]
use crate::infrastructure::system::ocr::MacOSVisionOcrEngine;

pub(crate) fn build_llm_introspection(http_client: Arc<dyn HttpClient>) -> Arc<LlmIntrospection> {
    Arc::new(LlmIntrospection::new(http_client))
}

pub(crate) fn build_provider_configuration(
    config_file: Arc<ConfigFile>,
    keychain: Arc<Keychain>,
    http_client: Arc<dyn HttpClient>,
    translation_coordinator: Arc<
        crate::application::providers::translation::TranslationCoordinator,
    >,
    llm_introspection: Arc<LlmIntrospection>,
) -> Arc<ProviderConfiguration> {
    Arc::new(ProviderConfiguration::new(
        config_file,
        keychain,
        http_client,
        translation_coordinator,
        llm_introspection,
    ))
}

pub(crate) fn build_translation_coordinator(
    config_file: Arc<ConfigFile>,
    _keychain: Arc<Keychain>,
    http_client: Arc<dyn HttpClient>,
    event_bus: Arc<EventBus>,
) -> Arc<TranslationCoordinator> {
    let translation_coordinator = TranslationCoordinator::new(config_file.clone());

    let google_provider = GoogleTranslateProviderV2::new(http_client.clone());
    if let Err(e) = translation_coordinator.register(google_provider) {
        log::warn!("Failed to register Google Translate provider: {}", e);
    }

    let deeplx_provider = DeepLProvider::new(http_client.clone());
    if let Err(e) = translation_coordinator.register(deeplx_provider) {
        log::warn!("Failed to register DeepLX provider: {}", e);
    }

    let baidu_provider = BaiduTranslateProvider::new(http_client.clone());
    translation_coordinator
        .register(baidu_provider)
        .map_err(|e| log::warn!("Failed to register Baidu Translate provider: {}", e))
        .ok();

    if let Err(e) = translation_coordinator.restore_from_config() {
        log::warn!("Failed to restore active providers from config: {}", e);
    }

    Arc::new(translation_coordinator.with_event_bus(event_bus))
}

pub(crate) fn build_ocr_coordinator(
    config_file: Arc<ConfigFile>,
    _keychain: Arc<Keychain>,
    http_client: Arc<dyn HttpClient>,
    event_bus: Arc<EventBus>,
) -> Arc<OcrCoordinator> {
    let ocr_coordinator = OcrCoordinator::new(config_file);

    let tesseract_provider = TesseractProvider::new(get_tesseract_engine());
    ocr_coordinator.register(tesseract_provider).ok();

    #[cfg(target_os = "macos")]
    ocr_coordinator
        .register(SystemOcrProvider::new(
            Arc::new(MacOSVisionOcrEngine::new()),
        ))
        .ok();

    let baidu_ocr_provider = BaiduOcrProvider::new(http_client);
    ocr_coordinator.register(baidu_ocr_provider).ok();
    ocr_coordinator.restore_from_config().ok();

    Arc::new(ocr_coordinator.with_event_bus(event_bus))
}

pub(crate) fn hydrate_provider_credentials(
    provider_configuration: Arc<ProviderConfiguration>,
    keychain: Arc<Keychain>,
    ocr_coordinator: Arc<OcrCoordinator>,
) {
    if let Err(e) = provider_configuration.hydrate_credentials() {
        log::warn!("Failed to hydrate translation provider credentials: {}", e);
    }
    hydrate_ocr_provider_credentials(&ocr_coordinator, &keychain);
}

fn hydrate_ocr_provider_credentials(ocr_coordinator: &OcrCoordinator, keychain: &Keychain) {
    if let Some(credentials) = load_baidu_ocr_credentials(keychain) {
        if let Err(e) = ocr_coordinator.reconfigure_provider("baidu-ocr", &credentials) {
            log::warn!("Failed to hydrate Baidu OCR credentials: {}", e);
        }
    }
}

fn load_baidu_ocr_credentials(keychain: &Keychain) -> Option<HashMap<String, String>> {
    keychain
        .load_provider_credentials(
            "baidu-ocr",
            &["api_key".to_string(), "secret_key".to_string()],
        )
        .ok()
        .or_else(|| {
            let api_key = keychain
                .load_provider_credential("baidu_ocr_api_key")
                .ok()?;
            let secret_key = keychain
                .load_provider_credential("baidu_ocr_secret_key")
                .ok()?;
            Some(HashMap::from([
                ("api_key".to_string(), api_key),
                ("secret_key".to_string(), secret_key),
            ]))
        })
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use crate::infrastructure::http::HttpResponse;
    use async_trait::async_trait;
    use std::collections::HashMap;

    struct StubHttpClient;

    #[async_trait]
    impl HttpClient for StubHttpClient {
        async fn post(
            &self,
            _url: &str,
            _headers: HashMap<String, String>,
            _body: String,
        ) -> anyhow::Result<HttpResponse> {
            unimplemented!("OCR provider registration should not make HTTP requests")
        }

        async fn get(
            &self,
            _url: &str,
            _headers: HashMap<String, String>,
        ) -> anyhow::Result<HttpResponse> {
            unimplemented!("OCR provider registration should not make HTTP requests")
        }
    }

    #[test]
    fn build_ocr_coordinator_registers_system_ocr_provider_on_macos() {
        let coordinator = build_ocr_coordinator(
            Arc::new(ConfigFile::new_temp()),
            Arc::new(Keychain::new()),
            Arc::new(StubHttpClient),
            Arc::new(EventBus::new()),
        );

        assert!(coordinator.get("system-ocr").is_some());
    }
}

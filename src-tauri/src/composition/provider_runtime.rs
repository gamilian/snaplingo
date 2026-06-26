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
use crate::application::providers::{
    create_llm_translation_provider, CustomTranslationProviderDef,
};
use crate::infrastructure::events::EventBus;
use crate::infrastructure::http::HttpClient;
use crate::infrastructure::storage::{ConfigFile, Keychain};

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

    let deepl_provider = DeepLProvider::new(http_client.clone());
    if let Err(e) = translation_coordinator.register(deepl_provider) {
        log::warn!("Failed to register DeepL provider: {}", e);
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

    let tesseract_provider = TesseractProvider::new();
    ocr_coordinator.register(tesseract_provider).ok();

    #[cfg(target_os = "macos")]
    ocr_coordinator.register(SystemOcrProvider::new()).ok();

    let baidu_ocr_provider = BaiduOcrProvider::new(http_client);
    ocr_coordinator.register(baidu_ocr_provider).ok();
    ocr_coordinator.restore_from_config().ok();

    Arc::new(ocr_coordinator.with_event_bus(event_bus))
}

pub(crate) fn hydrate_provider_credentials(
    config_file: Arc<ConfigFile>,
    keychain: Arc<Keychain>,
    http_client: Arc<dyn HttpClient>,
    translation_coordinator: Arc<TranslationCoordinator>,
    ocr_coordinator: Arc<OcrCoordinator>,
) {
    hydrate_translation_provider_credentials(
        &translation_coordinator,
        &config_file,
        &keychain,
        http_client,
    );
    hydrate_ocr_provider_credentials(&ocr_coordinator, &keychain);
}

fn hydrate_translation_provider_credentials(
    translation_coordinator: &TranslationCoordinator,
    config_file: &ConfigFile,
    keychain: &Keychain,
    http_client: Arc<dyn HttpClient>,
) {
    if let Ok(api_key) = keychain.load_provider_credential("deepl") {
        let credentials = HashMap::from([("api_key".to_string(), api_key)]);
        if let Err(e) = translation_coordinator.reconfigure_provider("deepl", &credentials) {
            log::warn!("Failed to hydrate DeepL credentials: {}", e);
        }
    }

    if let Some(credentials) = load_baidu_translation_credentials(keychain) {
        if let Err(e) =
            translation_coordinator.reconfigure_provider("baidu-translate", &credentials)
        {
            log::warn!("Failed to hydrate Baidu Translate credentials: {}", e);
        }
    }

    register_custom_translation_providers(
        translation_coordinator,
        config_file,
        keychain,
        http_client,
    );

    if let Err(e) = translation_coordinator.restore_from_config() {
        log::warn!(
            "Failed to restore active providers after credential hydration: {}",
            e
        );
    }
}

fn hydrate_ocr_provider_credentials(ocr_coordinator: &OcrCoordinator, keychain: &Keychain) {
    if let Some(credentials) = load_baidu_ocr_credentials(keychain) {
        if let Err(e) = ocr_coordinator.reconfigure_provider("baidu-ocr", &credentials) {
            log::warn!("Failed to hydrate Baidu OCR credentials: {}", e);
        }
    }
}

fn load_baidu_translation_credentials(keychain: &Keychain) -> Option<HashMap<String, String>> {
    keychain
        .load_provider_credentials(
            "baidu-translate",
            &["app_id".to_string(), "secret_key".to_string()],
        )
        .ok()
        .or_else(|| {
            let app_id = keychain.load_provider_credential("baidu_app_id").ok()?;
            let secret_key = keychain.load_provider_credential("baidu_secret_key").ok()?;
            Some(HashMap::from([
                ("app_id".to_string(), app_id),
                ("secret_key".to_string(), secret_key),
            ]))
        })
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

fn register_custom_translation_providers(
    translation_coordinator: &TranslationCoordinator,
    config_file: &ConfigFile,
    keychain: &Keychain,
    http_client: Arc<dyn HttpClient>,
) {
    let Ok(custom_defs) =
        config_file.load::<Vec<CustomTranslationProviderDef>>("custom_translation_providers")
    else {
        return;
    };

    for def in custom_defs {
        let Ok(api_key) = keychain.load_provider_credential(&def.id) else {
            continue;
        };

        let provider = create_llm_translation_provider(&def, http_client.clone(), api_key);
        if let Err(e) = translation_coordinator.register(provider) {
            log::warn!(
                "Failed to register custom LLM provider '{}': {}",
                def.name,
                e
            );
        }
    }
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

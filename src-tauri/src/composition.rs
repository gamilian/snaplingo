use std::path::PathBuf;
use std::sync::Arc;

use tauri::AppHandle;

use crate::application::providers::ocr::{
    impls::{BaiduOcrProvider, TesseractProvider},
    OcrCoordinator,
};
use crate::application::providers::translation::{
    BaiduTranslateProvider, DeepLProvider, GoogleTranslateProvider as GoogleTranslateProviderV2,
    LLMTranslationProvider, TranslationCoordinator,
};
use crate::application::providers::CustomTranslationProviderDef;
use crate::application::providers::Provider;
use crate::infrastructure::events::{EventBus, EventSubscriber};
use crate::infrastructure::http::HttpClient;
use crate::infrastructure::llm::{
    AnthropicLLMClient, GeminiLLMClient, LLMClient, LLMProtocol, OpenAILLMClient,
};
use crate::infrastructure::storage::{ConfigFile, Keychain};
use crate::AppState;

pub(crate) fn build_app_state(config_path: PathBuf, app: AppHandle) -> AppState {
    AppState::new(config_path, app)
}

pub(crate) fn build_translation_coordinator(
    config_file: Arc<ConfigFile>,
    keychain: Arc<Keychain>,
    http_client: Arc<dyn HttpClient>,
    event_bus: Arc<EventBus>,
) -> Arc<TranslationCoordinator> {
    let translation_coordinator = TranslationCoordinator::new(config_file.clone());

    let google_provider = GoogleTranslateProviderV2::new(http_client.clone());
    if let Err(e) = translation_coordinator.register(google_provider) {
        log::warn!("Failed to register Google Translate provider: {}", e);
    }

    let mut deepl_provider = DeepLProvider::new(http_client.clone());
    if let Ok(api_key) = keychain.load_provider_credential("deepl") {
        deepl_provider.set_api_key(api_key);
    }
    if let Err(e) = translation_coordinator.register(deepl_provider) {
        log::warn!("Failed to register DeepL provider: {}", e);
    }

    let mut baidu_provider = BaiduTranslateProvider::new(http_client.clone());
    let credentials_result = keychain.load_provider_credentials(
        "baidu-translate",
        &["app_id".to_string(), "secret_key".to_string()],
    );
    if let Ok(creds) = credentials_result {
        let _ = baidu_provider.configure_from_map(&creds);
    } else if let Ok(app_id) = keychain.load_provider_credential("baidu_app_id") {
        if let Ok(secret_key) = keychain.load_provider_credential("baidu_secret_key") {
            baidu_provider.configure(app_id, secret_key);
        }
    }

    translation_coordinator
        .register(baidu_provider)
        .map_err(|e| log::warn!("Failed to register Baidu Translate provider: {}", e))
        .ok();

    register_custom_translation_providers(
        &translation_coordinator,
        &config_file,
        &keychain,
        http_client,
    );

    if let Err(e) = translation_coordinator.restore_from_config() {
        log::warn!("Failed to restore active providers from config: {}", e);
    }

    Arc::new(translation_coordinator.with_event_bus(event_bus))
}

pub(crate) fn build_ocr_coordinator(
    config_file: Arc<ConfigFile>,
    keychain: Arc<Keychain>,
    http_client: Arc<dyn HttpClient>,
    event_bus: Arc<EventBus>,
) -> Arc<OcrCoordinator> {
    let ocr_coordinator = OcrCoordinator::new(config_file);

    let tesseract_provider = TesseractProvider::new();
    ocr_coordinator.register(tesseract_provider).ok();

    let mut baidu_ocr_provider = BaiduOcrProvider::new(http_client);
    let credentials_result = keychain.load_provider_credentials(
        "baidu-ocr",
        &["api_key".to_string(), "secret_key".to_string()],
    );
    if let Ok(creds) = credentials_result {
        let _ = baidu_ocr_provider.reconfigure_credentials(&creds);
    } else if let Ok(api_key) = keychain.load_provider_credential("baidu_ocr_api_key") {
        if let Ok(secret_key) = keychain.load_provider_credential("baidu_ocr_secret_key") {
            baidu_ocr_provider.configure(api_key, secret_key);
        }
    }

    ocr_coordinator.register(baidu_ocr_provider).ok();
    ocr_coordinator.restore_from_config().ok();

    Arc::new(ocr_coordinator.with_event_bus(event_bus))
}

pub(crate) fn subscribe_history_service(app_state: &AppState) {
    let history_service_subscriber = app_state.history_service.clone() as Arc<dyn EventSubscriber>;
    let event_bus = app_state.event_bus.clone();
    tauri::async_runtime::spawn(async move {
        event_bus.subscribe(history_service_subscriber).await;
    });
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

        let llm_client: Arc<dyn LLMClient> = match def.protocol {
            LLMProtocol::OpenAI => Arc::new(OpenAILLMClient::new(
                http_client.clone(),
                def.endpoint.clone(),
                def.model.clone(),
                api_key,
            )),
            LLMProtocol::Anthropic => Arc::new(AnthropicLLMClient::new(
                http_client.clone(),
                def.endpoint.clone(),
                def.model.clone(),
                api_key,
            )),
            LLMProtocol::Gemini => Arc::new(GeminiLLMClient::new(
                http_client.clone(),
                def.endpoint.clone(),
                def.model.clone(),
                api_key,
            )),
        };

        let provider = LLMTranslationProvider::new(
            llm_client,
            def.id.clone(),
            def.name.clone(),
            def.reasoning_level,
        );
        if let Err(e) = translation_coordinator.register(provider) {
            log::warn!(
                "Failed to register custom LLM provider '{}': {}",
                def.name,
                e
            );
        }
    }
}

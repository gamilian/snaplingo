use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::Mutex as ParkingLotMutex;
use tauri::AppHandle;

use crate::application::providers::ocr::{
    impls::{BaiduOcrProvider, TesseractProvider},
    OcrCoordinator,
};
use crate::application::providers::translation::{
    BaiduTranslateProvider, DeepLProvider, GoogleTranslateProvider as GoogleTranslateProviderV2,
    TranslationCoordinator,
};
use crate::application::providers::Provider;
use crate::application::providers::{
    create_llm_translation_provider, CustomTranslationProviderDef,
};
use crate::infrastructure::events::{EventBus, EventSubscriber};
use crate::infrastructure::http::{HttpClient, ReqwestHttpClient};
use crate::infrastructure::storage::{ConfigFile, HistoryDatabase, Keychain};
use crate::infrastructure::system::paths::get_history_db_path;
use crate::infrastructure::system::screenshot::get_screenshot_backend;
use crate::infrastructure::system::selection::{
    platform_selection_provider, SelectionMethodRegistry, SystemSelectionProvider,
};
use crate::{
    AppState, CaptureOutputService, CaptureService, CaptureSessionRuntime, CaptureSessionService,
    HistoryService, ImageCompositionService, PinnedImageService, ScreenshotState,
    SelectedTextAcquirer, SelectionScheme, WorkflowService,
};

pub(crate) fn build_app_state(config_path: PathBuf, _app: AppHandle) -> AppState {
    // Phase 1: Infrastructure
    let config_file = Arc::new(ConfigFile::new(config_path.clone()));
    let keychain = Arc::new(Keychain::new());
    let http_client: Arc<dyn HttpClient> = Arc::new(ReqwestHttpClient::new());

    // Phase 5: EventBus & History
    let event_bus = Arc::new(EventBus::new());

    let history_db_path = get_history_db_path().expect("Failed to get history database path");
    let history_db = Arc::new(
        HistoryDatabase::new(history_db_path).expect("Failed to initialize history database"),
    );
    let history_service = Arc::new(HistoryService::new(history_db));

    // Subscribe history service to events (will be done in setup hook)
    // Note: Cannot block_on here as Tokio runtime may not be ready yet

    let translation_coordinator = build_translation_coordinator(
        config_file.clone(),
        keychain.clone(),
        http_client.clone(),
        event_bus.clone(),
    );
    let ocr_coordinator = build_ocr_coordinator(
        config_file.clone(),
        keychain.clone(),
        http_client.clone(),
        event_bus.clone(),
    );

    // Phase 4: Capture
    let screenshot_backend = get_screenshot_backend();
    let capture_service = Arc::new(CaptureService::new(screenshot_backend.clone()));
    let capture_session_service = Arc::new(CaptureSessionService::new(screenshot_backend));
    let image_composition_service = Arc::new(ImageCompositionService::new());
    let capture_output_service = Arc::new(CaptureOutputService::new());
    let capture_session_runtime = Arc::new(CaptureSessionRuntime::new(
        capture_session_service.clone(),
        image_composition_service.clone(),
        capture_output_service.clone(),
        ocr_coordinator.clone(),
    ));
    let pinned_image_service = Arc::new(PinnedImageService::new());
    let screenshot_state = Arc::new(ParkingLotMutex::new(ScreenshotState::default()));

    // Phase 6: Workflows
    let workflow_service = Arc::new(WorkflowService::new(
        capture_service.clone(),
        ocr_coordinator.clone(),
        translation_coordinator.clone(),
    ));

    // Phase 7: Selected text acquisition
    let self_bundle_id = Some(_app.config().identifier.clone());
    let selection_provider = Arc::new(platform_selection_provider(_app.clone(), self_bundle_id));
    let selection_scheme = SelectionScheme::new(selection_provider.default_scheme());
    let selected_text_acquirer = Arc::new(SelectedTextAcquirer::new(
        selection_scheme,
        SelectionMethodRegistry::new(selection_provider.methods()),
        selection_provider,
    ));

    AppState {
        config_file,
        keychain,
        http_client,
        translation_coordinator,
        ocr_coordinator,
        capture_service,
        capture_session_service,
        image_composition_service,
        capture_output_service,
        capture_session_runtime,
        pinned_image_service,
        screenshot_state,
        history_service,
        event_bus,
        workflow_service,
        selected_text_acquirer,
    }
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

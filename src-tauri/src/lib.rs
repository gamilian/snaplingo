// Module declarations
mod error;
mod domain;
mod infrastructure;
mod application;
mod commands;

// Public exports for new infrastructure layer
pub use error::{AppError, Result};
pub use domain::*;
pub use infrastructure::*;
pub use application::*;

use std::sync::Arc;
use std::path::PathBuf;
use tauri::Manager;
use serde::{Deserialize, Serialize};

// Phase 1, 2 & 3 imports
use infrastructure::storage::{ConfigFile, Keychain, HistoryDatabase};
use infrastructure::http::{HttpClient, ReqwestHttpClient};
use infrastructure::events::{EventBus, EventSubscriber};
use infrastructure::llm::{
    LLMClient, LLMProtocol, OpenAILLMClient, AnthropicLLMClient, GeminiLLMClient, ReasoningLevel,
};
use application::providers::translation::{
    TranslationCoordinator, TranslationProvider,
    GoogleTranslateProvider as GoogleTranslateProviderV2,
    DeepLProvider,
    BaiduTranslateProvider,
    LLMTranslationProvider,
};
use application::providers::ocr::{
    OcrCoordinator,
    impls::{TesseractProvider, BaiduOcrProvider},
};
use application::{CaptureService, HotkeyService, HistoryService};
use infrastructure::system::screenshot::get_screenshot_backend;
use infrastructure::system::hotkey::get_hotkey_backend;
use infrastructure::system::paths::get_history_db_path;

/// Custom translation provider definition (for persistence)
#[derive(Clone, Serialize, Deserialize)]
pub struct CustomTranslationProviderDef {
    pub id: String,
    pub name: String,
    pub protocol: LLMProtocol,
    pub endpoint: String,
    pub model: String,
    pub reasoning_level: Option<ReasoningLevel>,
}

/// Factory function to create LLM translation provider
pub fn create_llm_translation_provider(
    def: &CustomTranslationProviderDef,
    http_client: Arc<dyn HttpClient>,
    api_key: String,
) -> Arc<dyn TranslationProvider> {
    let llm_client: Arc<dyn LLMClient> = match def.protocol {
        LLMProtocol::OpenAI => Arc::new(OpenAILLMClient::new(
            http_client,
            def.endpoint.clone(),
            def.model.clone(),
            api_key,
        )),
        LLMProtocol::Anthropic => Arc::new(AnthropicLLMClient::new(
            http_client,
            def.endpoint.clone(),
            def.model.clone(),
            api_key,
        )),
        LLMProtocol::Gemini => Arc::new(GeminiLLMClient::new(
            http_client,
            def.endpoint.clone(),
            def.model.clone(),
            api_key,
        )),
    };

    Arc::new(LLMTranslationProvider::new(
        llm_client,
        def.id.clone(),
        def.name.clone(),
        def.reasoning_level,
    ))
}

pub struct AppState {
    // Phase 1: Infrastructure
    pub config_file: Arc<ConfigFile>,
    pub keychain: Arc<Keychain>,
    pub http_client: Arc<dyn HttpClient>,

    // Phase 2: Translation
    pub translation_coordinator: Arc<TranslationCoordinator>,

    // Phase 3: OCR
    pub ocr_coordinator: Arc<OcrCoordinator>,

    // Phase 4: Capture
    pub capture_service: Arc<CaptureService>,
    pub hotkey_service: Arc<HotkeyService>,

    // Phase 5: History
    pub history_service: Arc<HistoryService>,
    pub event_bus: Arc<EventBus>,
}

impl AppState {
    pub fn new(config_path: PathBuf, _app: tauri::AppHandle) -> Self {
        // Phase 1: Infrastructure
        let config_file = Arc::new(ConfigFile::new(config_path.clone()));
        let keychain = Arc::new(Keychain::new());
        let http_client: Arc<dyn HttpClient> = Arc::new(ReqwestHttpClient::new());

        // Phase 5: EventBus & History
        let event_bus = Arc::new(EventBus::new());

        let history_db_path = get_history_db_path().expect("Failed to get history database path");
        let history_db = Arc::new(HistoryDatabase::new(history_db_path).expect("Failed to initialize history database"));
        let history_service = Arc::new(HistoryService::new(history_db));

        // Subscribe history service to events
        let history_service_subscriber = history_service.clone() as Arc<dyn EventSubscriber>;
        tokio::runtime::Handle::current().block_on(async {
            event_bus.subscribe(history_service_subscriber).await;
        });

        // Phase 2: Translation
        let translation_coordinator = TranslationCoordinator::new(config_file.clone());

        // Register Google Translate (no credentials needed)
        let google_provider = GoogleTranslateProviderV2::new(http_client.clone());
        translation_coordinator.register(Arc::new(google_provider)).ok();

        // Register DeepL (load credentials from keychain using provider ID)
        let mut deepl_provider = DeepLProvider::new(http_client.clone());
        if let Ok(api_key) = keychain.load_provider_credential("deepl") {
            deepl_provider.set_api_key(api_key);
        }
        translation_coordinator.register(Arc::new(deepl_provider)).ok();

        // Register Baidu (load credentials from keychain)
        // Try new multi-field format first, fallback to legacy keys for backward compatibility
        let mut baidu_provider = BaiduTranslateProvider::new(http_client.clone());

        // Try new format: provider:baidu-translate:credential:{app_id,secret_key}
        let credentials_result = keychain.load_provider_credentials(
            "baidu-translate",
            &["app_id".to_string(), "secret_key".to_string()],
        );

        if let Ok(creds) = credentials_result {
            let _ = baidu_provider.configure_from_map(&creds);
        } else {
            // Fallback to legacy keys: provider:baidu_app_id:api_key, provider:baidu_secret_key:api_key
            if let Ok(app_id) = keychain.load_provider_credential("baidu_app_id") {
                if let Ok(secret_key) = keychain.load_provider_credential("baidu_secret_key") {
                    baidu_provider.configure(app_id, secret_key);
                }
            }
        }

        translation_coordinator.register(Arc::new(baidu_provider)).ok();

        // Load and register custom LLM providers
        if let Ok(custom_defs) = config_file.load::<Vec<CustomTranslationProviderDef>>("custom_translation_providers") {
            for def in custom_defs {
                // Load API key from keychain
                if let Ok(api_key) = keychain.load_provider_credential(&def.id) {
                    let provider = create_llm_translation_provider(&def, http_client.clone(), api_key);
                    translation_coordinator.register(provider).ok();
                }
            }
        }

        // Restore active providers from config
        translation_coordinator.restore_from_config().ok();

        // Attach event bus to coordinator
        let translation_coordinator = Arc::new(
            translation_coordinator.with_event_bus(event_bus.clone())
        );

        // Phase 3: OCR
        let mut ocr_coordinator = OcrCoordinator::new(config_file.clone());

        // Register Tesseract (no credentials needed)
        let tesseract_provider = TesseractProvider::new();
        ocr_coordinator.register(Arc::new(tesseract_provider)).ok();

        // Register Baidu OCR (load credentials from keychain)
        // Try new multi-field format first, fallback to legacy keys for backward compatibility
        let mut baidu_ocr_provider = BaiduOcrProvider::new(http_client.clone());

        // Try new format: provider:baidu-ocr:credential:{api_key,secret_key}
        let credentials_result = keychain.load_provider_credentials(
            "baidu-ocr",
            &["api_key".to_string(), "secret_key".to_string()],
        );

        if let Ok(creds) = credentials_result {
            let _ = baidu_ocr_provider.configure_from_map(&creds);
        } else {
            // Fallback to legacy keys: provider:baidu_ocr_api_key:api_key, provider:baidu_ocr_secret_key:api_key
            if let Ok(api_key) = keychain.load_provider_credential("baidu_ocr_api_key") {
                if let Ok(secret_key) = keychain.load_provider_credential("baidu_ocr_secret_key") {
                    baidu_ocr_provider.configure(api_key, secret_key);
                }
            }
        }

        ocr_coordinator.register(Arc::new(baidu_ocr_provider)).ok();

        // Restore active OCR provider from config
        ocr_coordinator.restore_from_config().ok();

        // Attach event bus to coordinator
        let ocr_coordinator = Arc::new(
            ocr_coordinator.with_event_bus(event_bus.clone())
        );

        // Phase 4: Capture
        let screenshot_backend = get_screenshot_backend();
        let capture_service = Arc::new(CaptureService::new(screenshot_backend));

        let hotkey_backend = get_hotkey_backend();
        let hotkey_service = Arc::new(HotkeyService::new(hotkey_backend));

        Self {
            config_file,
            keychain,
            http_client,
            translation_coordinator,
            ocr_coordinator,
            capture_service,
            hotkey_service,
            history_service,
            event_bus,
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  let config_dir = dirs::home_dir()
      .unwrap()
      .join(".snaplingo");
  std::fs::create_dir_all(&config_dir).unwrap();
  let config_path = config_dir.join("config.json");

  tauri::Builder::default()
    .plugin(tauri_plugin_shell::init())
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }

      let app_state = AppState::new(config_path, app.handle().clone());
      app.manage(app_state);

      // TODO: Register global hotkeys
      // TODO: Create system tray

      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      commands::open_result_window,
      commands::translate_text_v2,
      commands::list_translation_providers,
      commands::activate_translation_provider,
      commands::deactivate_translation_provider,
      commands::reorder_active_translation_providers,
      commands::configure_translation_provider,
      commands::get_provider_credential_schema,
      commands::configure_translation_provider_credentials,
      commands::add_custom_translation_provider,
      commands::remove_custom_translation_provider,
      commands::recognize_image,
      commands::list_ocr_providers,
      commands::activate_ocr_provider,
      commands::configure_ocr_provider,
      commands::capture_full_screen,
      commands::capture_region,
      commands::save_screenshot,
      commands::get_translation_history,
      commands::get_ocr_history,
      commands::search_history,
      commands::delete_history,
      commands::clear_all_history,
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

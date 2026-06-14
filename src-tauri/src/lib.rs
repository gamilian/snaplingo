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

// Phase 1, 2 & 3 imports
use infrastructure::storage::{ConfigFile, Keychain, HistoryDatabase};
use infrastructure::http::{HttpClient, ReqwestHttpClient};
use infrastructure::events::{EventBus, EventSubscriber};
use application::providers::translation::{
    TranslationCoordinator,
    GoogleTranslateProvider as GoogleTranslateProviderV2,
    DeepLProvider,
    BaiduTranslateProvider,
};
use application::providers::ocr::{
    OcrCoordinator,
    impls::{TesseractProvider, BaiduOcrProvider},
};
use application::{CaptureService, HotkeyService, HistoryService};
use infrastructure::system::screenshot::get_screenshot_backend;
use infrastructure::system::hotkey::get_hotkey_backend;
use infrastructure::system::paths::get_history_db_path;

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
        let mut translation_coordinator = TranslationCoordinator::new(config_file.clone());

        // Register Google Translate (no credentials needed)
        let google_provider = GoogleTranslateProviderV2::new(http_client.clone());
        translation_coordinator.register(Arc::new(google_provider)).ok();

        // Register DeepL (load credentials from keychain)
        let mut deepl_provider = DeepLProvider::new(http_client.clone());
        if let Ok(api_key) = keychain.load_provider_credential("deepl") {
            deepl_provider.set_api_key(api_key);
        }
        translation_coordinator.register(Arc::new(deepl_provider)).ok();

        // Register Baidu (load credentials from keychain)
        let mut baidu_provider = BaiduTranslateProvider::new(http_client.clone());
        if let Ok(app_id) = keychain.load_provider_credential("baidu_app_id") {
            if let Ok(secret_key) = keychain.load_provider_credential("baidu_secret_key") {
                baidu_provider.configure(app_id, secret_key);
            }
        }
        translation_coordinator.register(Arc::new(baidu_provider)).ok();

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
        let mut baidu_ocr_provider = BaiduOcrProvider::new(http_client.clone());
        if let Ok(api_key) = keychain.load_provider_credential("baidu_ocr_api_key") {
            if let Ok(secret_key) = keychain.load_provider_credential("baidu_ocr_secret_key") {
                baidu_ocr_provider.configure(api_key, secret_key);
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
      commands::configure_translation_provider,
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

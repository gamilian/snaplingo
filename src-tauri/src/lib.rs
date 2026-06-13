// Module declarations
mod error;
mod domain;
mod infrastructure;
mod application;
mod commands;
mod config;
mod language;
mod ocr;
mod translate;
mod capture;
mod history;
mod utils;
mod hotkeys;

// Public exports for new infrastructure layer
pub use error::{AppError, Result};
pub use domain::*;
pub use infrastructure::*;
pub use application::*;

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::path::PathBuf;
use config::Config;
use translate::{GoogleTranslateProvider, TranslationProvider};
use language::LanguageDetector;
use hotkeys::HotkeyManager;
use tauri::Manager;

// Phase 1 & 2 imports
use infrastructure::storage::{ConfigFile, Keychain};
use infrastructure::http::{HttpClient, ReqwestHttpClient};
use application::providers::translation::{
    TranslationRegistry,
    TranslationService,
    GoogleTranslateProvider as GoogleTranslateProviderV2,
    DeepLProvider,
    BaiduTranslateProvider,
};

pub struct AppState {
    // Phase 1: Infrastructure
    pub config_file: Arc<ConfigFile>,
    pub keychain: Arc<Keychain>,
    pub http_client: Arc<dyn HttpClient>,

    // Phase 2: Translation
    pub translation_registry: Arc<Mutex<TranslationRegistry>>,
    pub translation_service: Arc<TranslationService>,

    // Legacy (Phase 5: remove these)
    pub config: Arc<Mutex<Config>>,
    pub config_path: PathBuf,
    translation_providers: Arc<Mutex<HashMap<String, Arc<dyn TranslationProvider>>>>,
    pub language_detector: LanguageDetector,
    pub hotkey_manager: HotkeyManager,
}

impl AppState {
    pub fn new(config_path: PathBuf, app: tauri::AppHandle) -> Self {
        // Phase 1: Infrastructure
        let config_file = Arc::new(ConfigFile::new(config_path.clone()));
        let keychain = Arc::new(Keychain::new());
        let http_client: Arc<dyn HttpClient> = Arc::new(ReqwestHttpClient::new());

        // Phase 2: Translation
        let mut translation_registry = TranslationRegistry::new();

        // Register Google Translate (no credentials needed)
        let google_provider = GoogleTranslateProviderV2::new(http_client.clone());
        translation_registry.register(Arc::new(google_provider)).ok();

        // Register DeepL (load credentials from keychain)
        let mut deepl_provider = DeepLProvider::new(http_client.clone());
        if let Ok(api_key) = keychain.load_provider_credential("deepl") {
            deepl_provider.set_api_key(api_key);
        }
        translation_registry.register(Arc::new(deepl_provider)).ok();

        // Register Baidu (load credentials from keychain)
        let mut baidu_provider = BaiduTranslateProvider::new(http_client.clone());
        if let Ok(app_id) = keychain.load_provider_credential("baidu_app_id") {
            if let Ok(secret_key) = keychain.load_provider_credential("baidu_secret_key") {
                baidu_provider.configure(app_id, secret_key);
            }
        }
        translation_registry.register(Arc::new(baidu_provider)).ok();

        // Restore active providers from config
        if let Ok(active_ids) = config_file.load::<Vec<String>>("translation.active_providers") {
            for id in active_ids {
                translation_registry.activate(&id).ok();
            }
        }

        let translation_registry = Arc::new(Mutex::new(translation_registry));
        let translation_service = Arc::new(TranslationService::new(translation_registry.clone()));

        // Legacy initialization
        let config = Config::load_or_default(&config_path).unwrap_or_default();
        let mut providers: HashMap<String, Arc<dyn TranslationProvider>> = HashMap::new();
        providers.insert(
            "google-translate".to_string(),
            Arc::new(GoogleTranslateProvider::default()),
        );

        Self {
            config_file,
            keychain,
            http_client,
            translation_registry,
            translation_service,
            config: Arc::new(Mutex::new(config)),
            config_path,
            translation_providers: Arc::new(Mutex::new(providers)),
            language_detector: LanguageDetector::new(),
            hotkey_manager: HotkeyManager::new(app),
        }
    }

    pub fn get_translation_provider(&self, id: &str) -> Option<Arc<dyn TranslationProvider>> {
        self.translation_providers.lock().unwrap().get(id).cloned()
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
      commands::translate_text,
      commands::detect_language,
      commands::get_config,
      commands::update_config,
      commands::open_result_window,
      commands::translate_text_v2,
      commands::list_translation_providers,
      commands::activate_translation_provider,
      commands::deactivate_translation_provider,
      commands::configure_translation_provider,
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

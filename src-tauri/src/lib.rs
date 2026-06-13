// Module declarations
mod error;
mod domain;
mod infrastructure;
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

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::path::PathBuf;
use config::Config;
use translate::{GoogleTranslateProvider, TranslationProvider};
use language::LanguageDetector;
use hotkeys::HotkeyManager;
use tauri::Manager;

pub struct AppState {
    pub config: Arc<Mutex<Config>>,
    pub config_path: PathBuf,
    translation_providers: Arc<Mutex<HashMap<String, Arc<dyn TranslationProvider>>>>,
    pub language_detector: LanguageDetector,
    pub hotkey_manager: HotkeyManager,
}

impl AppState {
    pub fn new(config_path: PathBuf, app: tauri::AppHandle) -> Self {
        let config = Config::load_or_default(&config_path).unwrap_or_default();
        let mut providers: HashMap<String, Arc<dyn TranslationProvider>> = HashMap::new();

        // Register built-in providers
        providers.insert(
            "google-translate".to_string(),
            Arc::new(GoogleTranslateProvider::default()),
        );

        Self {
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
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

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
use parking_lot::Mutex as ParkingLotMutex;
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
use application::{
    CaptureOutputService, CaptureService, CaptureSessionService, HistoryService,
    ImageCompositionService, PinnedImageService, WorkflowService,
};
use infrastructure::system::screenshot::get_screenshot_backend;
use infrastructure::system::paths::get_history_db_path;
use domain::HotkeyAction;

use std::collections::HashMap;
use std::sync::Mutex;

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

/// Screenshot state for storing captured image data
#[derive(Default)]
pub struct ScreenshotState {
    pub data: Option<Vec<u8>>,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
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
    pub capture_session_service: Arc<CaptureSessionService>,
    pub image_composition_service: Arc<ImageCompositionService>,
    pub capture_output_service: Arc<CaptureOutputService>,
    pub pinned_image_service: Arc<PinnedImageService>,
    pub screenshot_state: Arc<ParkingLotMutex<ScreenshotState>>,

    // Phase 5: History
    pub history_service: Arc<HistoryService>,
    pub event_bus: Arc<EventBus>,

    // Phase 6: Workflows
    pub workflow_service: Arc<WorkflowService>,
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

        // Subscribe history service to events (will be done in setup hook)
        // Note: Cannot block_on here as Tokio runtime may not be ready yet

        // Phase 2: Translation
        let translation_coordinator = TranslationCoordinator::new(config_file.clone());

        // Register Google Translate (no credentials needed)
        let google_provider = GoogleTranslateProviderV2::new(http_client.clone());
        if let Err(e) = translation_coordinator.register(google_provider) {
            log::warn!("Failed to register Google Translate provider: {}", e);
        }

        // Register DeepL (load credentials from keychain using provider ID)
        let mut deepl_provider = DeepLProvider::new(http_client.clone());
        if let Ok(api_key) = keychain.load_provider_credential("deepl") {
            deepl_provider.set_api_key(api_key);
        }
        if let Err(e) = translation_coordinator.register(deepl_provider) {
            log::warn!("Failed to register DeepL provider: {}", e);
        }

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

        translation_coordinator.register(baidu_provider)
            .map_err(|e| log::warn!("Failed to register Baidu Translate provider: {}", e))
            .ok();

        // Load and register custom LLM providers
        if let Ok(custom_defs) = config_file.load::<Vec<CustomTranslationProviderDef>>("custom_translation_providers") {
            for def in custom_defs {
                // Load API key from keychain
                if let Ok(api_key) = keychain.load_provider_credential(&def.id) {
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
                        log::warn!("Failed to register custom LLM provider '{}': {}", def.name, e);
                    }
                }
            }
        }

        // Restore active providers from config
        if let Err(e) = translation_coordinator.restore_from_config() {
            log::warn!("Failed to restore active providers from config: {}", e);
        }

        // Attach event bus to coordinator
        let translation_coordinator = Arc::new(
            translation_coordinator.with_event_bus(event_bus.clone())
        );

        // Phase 3: OCR
        let ocr_coordinator = OcrCoordinator::new(config_file.clone());

        // Register Tesseract (no credentials needed)
        let tesseract_provider = TesseractProvider::new();
        ocr_coordinator.register(tesseract_provider).ok();

        // Register Baidu OCR (load credentials from keychain)
        // Try new multi-field format first, fallback to legacy keys for backward compatibility
        let mut baidu_ocr_provider = BaiduOcrProvider::new(http_client.clone());

        // Try new format: provider:baidu-ocr:credential:{api_key,secret_key}
        let credentials_result = keychain.load_provider_credentials(
            "baidu-ocr",
            &["api_key".to_string(), "secret_key".to_string()],
        );

        if let Ok(creds) = credentials_result {
            let _ = baidu_ocr_provider.reconfigure_credentials(&creds);
        } else {
            // Fallback to legacy keys: provider:baidu_ocr_api_key:api_key, provider:baidu_ocr_secret_key:api_key
            if let Ok(api_key) = keychain.load_provider_credential("baidu_ocr_api_key") {
                if let Ok(secret_key) = keychain.load_provider_credential("baidu_ocr_secret_key") {
                    baidu_ocr_provider.configure(api_key, secret_key);
                }
            }
        }

        ocr_coordinator.register(baidu_ocr_provider).ok();

        // Restore active OCR provider from config
        ocr_coordinator.restore_from_config().ok();

        // Attach event bus to coordinator
        let ocr_coordinator = Arc::new(
            ocr_coordinator.with_event_bus(event_bus.clone())
        );

        // Phase 4: Capture
        let screenshot_backend = get_screenshot_backend();
        let capture_service = Arc::new(CaptureService::new(screenshot_backend.clone()));
        let capture_session_service = Arc::new(CaptureSessionService::new(screenshot_backend));
        let image_composition_service = Arc::new(ImageCompositionService::new());
        let capture_output_service = Arc::new(CaptureOutputService::new());
        let pinned_image_service = Arc::new(PinnedImageService::new());
        let screenshot_state = Arc::new(ParkingLotMutex::new(ScreenshotState::default()));

        // Phase 6: Workflows
        let workflow_service = Arc::new(WorkflowService::new(
            capture_service.clone(),
            ocr_coordinator.clone(),
            translation_coordinator.clone(),
        ));

        Self {
            config_file,
            keychain,
            http_client,
            translation_coordinator,
            ocr_coordinator,
            capture_service,
            capture_session_service,
            image_composition_service,
            capture_output_service,
            pinned_image_service,
            screenshot_state,
            history_service,
            event_bus,
            workflow_service,
        }
    }

    /// Gracefully shutdown the application, waiting for pending events to complete
    pub async fn shutdown(&self) -> Result<()> {
        log::info!("Starting graceful shutdown...");

        // Wait for all pending events to complete (max 5 seconds)
        let drained = self.event_bus.drain(std::time::Duration::from_secs(5)).await;

        if !drained {
            log::warn!("Shutdown: Some events did not complete in time");
        }

        log::info!("Graceful shutdown complete");
        Ok(())
    }
}

/// Register screenshot shortcut
fn register_screenshot_shortcut(app: &tauri::AppHandle) -> Result<()> {
    let app_clone = app.clone();

    infrastructure::system::register_shortcut(app, "Cmd+Shift+R", move || {
        log::info!("Screenshot shortcut triggered!");

        let app = app_clone.clone();
        tauri::async_runtime::spawn(commands::open_capture_window_from_shortcut(
            app,
            "screenshot",
        ));
    })?;

    Ok(())
}

/// Register screenshot OCR shortcut
fn register_screenshot_ocr_shortcut(app: &tauri::AppHandle) -> Result<()> {
    let app_clone = app.clone();

    infrastructure::system::register_shortcut(app, "Cmd+Shift+S", move || {
        log::info!("Screenshot OCR shortcut triggered!");

        let app = app_clone.clone();
        tauri::async_runtime::spawn(commands::open_capture_window_from_shortcut(
            app,
            "screenshot-ocr",
        ));
    })?;

    Ok(())
}

/// Register pinned image toggle shortcut
fn register_pin_toggle_shortcut(app: &tauri::AppHandle) -> Result<()> {
    let app_clone = app.clone();

    infrastructure::system::register_shortcut(app, "Shift+F3", move || {
        log::info!("Pinned image toggle shortcut triggered!");

        if let Err(err) = commands::toggle_pinned_images_visibility(app_clone.clone()) {
            log::error!("Failed to toggle pinned images: {}", err);
        }
    })?;

    Ok(())
}

/// Register pinned image group switch shortcut
fn register_pin_group_switch_shortcut(app: &tauri::AppHandle) -> Result<()> {
    let app_clone = app.clone();

    infrastructure::system::register_shortcut(app, "Cmd+F3", move || {
        log::info!("Pinned image group switch shortcut triggered!");

        let state = app_clone.state::<AppState>();
        if let Err(err) = commands::switch_pinned_image_group_for_state(&app_clone, state.inner()) {
            log::error!("Failed to switch pinned image group: {}", err);
        }
    })?;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  let config_dir = dirs::home_dir()
      .expect("Cannot determine home directory")
      .join(".snaplingo");
  std::fs::create_dir_all(&config_dir)
      .expect("Failed to create config directory");
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

      // Subscribe history service to event bus (must be done after Tokio runtime is ready)
      let history_service_subscriber = app_state.history_service.clone() as Arc<dyn EventSubscriber>;
      let event_bus = app_state.event_bus.clone();
      tauri::async_runtime::spawn(async move {
        event_bus.subscribe(history_service_subscriber).await;
      });

      app.manage(app_state);

      // Register global shortcuts using Tauri plugin after setup completes
      let app_handle = app.handle().clone();
      tauri::async_runtime::spawn(async move {
          tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

          if let Err(e) = register_screenshot_shortcut(&app_handle) {
              log::error!("Failed to register screenshot shortcut: {}", e);
          } else {
              log::info!("Screenshot shortcut registered: Cmd+Shift+R");
          }

          if let Err(e) = register_screenshot_ocr_shortcut(&app_handle) {
              log::error!("Failed to register screenshot OCR shortcut: {}", e);
          } else {
              log::info!("Screenshot OCR shortcut registered: Cmd+Shift+S");
          }

          if let Err(e) = register_pin_toggle_shortcut(&app_handle) {
              log::error!("Failed to register pinned image toggle shortcut: {}", e);
          } else {
              log::info!("Pinned image toggle shortcut registered: Shift+F3");
          }

          if let Err(e) = register_pin_group_switch_shortcut(&app_handle) {
              log::error!("Failed to register pinned image group switch shortcut: {}", e);
          } else {
              log::info!("Pinned image group switch shortcut registered: Cmd+F3");
          }
      });

      // TODO: Create system tray

      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      commands::open_result_window,
      commands::trigger_screenshot,
      commands::open_capture_window,
      commands::create_screenshot_window,
      commands::create_screenshot_window_simple,
      commands::close_screenshot_window,
      commands::crop_screenshot,
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
      commands::create_capture_session,
      commands::get_capture_session,
      commands::cancel_capture_session,
      commands::render_capture_output,
      commands::default_capture_save_path,
      commands::output_capture,
      commands::get_pinned_image,
      commands::copy_pinned_image,
      commands::save_pinned_image,
      commands::remove_pinned_image,
      commands::toggle_pinned_images_visibility,
      commands::switch_pinned_image_group,
      commands::move_pinned_image_to_next_group,
      commands::run_capture_ocr,
      commands::get_translation_history,
      commands::get_ocr_history,
      commands::search_history,
      commands::delete_history,
      commands::clear_all_history,
      commands::trigger_workflow,
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

// Module declarations
mod error;
mod domain;
mod infrastructure;
mod application;
mod commands;
mod composition;

// Public exports for new infrastructure layer
pub use error::{AppError, Result};
pub use domain::*;
pub use infrastructure::*;
pub use application::*;

use std::sync::Arc;
use parking_lot::Mutex as ParkingLotMutex;
use std::path::PathBuf;
use tauri::Manager;

// Phase 1, 2 & 3 imports
use infrastructure::storage::{ConfigFile, Keychain, HistoryDatabase};
use infrastructure::http::{HttpClient, ReqwestHttpClient};
use infrastructure::events::EventBus;
use application::providers::translation::TranslationCoordinator;
use application::providers::ocr::OcrCoordinator;
use application::{
    CaptureOutputService, CaptureService, CaptureSessionRuntime, CaptureSessionService,
    HistoryService, ImageCompositionService, PinnedImageService, WorkflowService,
};
use infrastructure::system::screenshot::get_screenshot_backend;
use infrastructure::system::paths::get_history_db_path;
use domain::HotkeyAction;

use std::collections::HashMap;
use std::sync::Mutex;

const SCREENSHOT_SHORTCUT: &str = "CmdOrCtrl+Shift+KeyR";
const SCREENSHOT_OCR_SHORTCUT: &str = "CmdOrCtrl+Shift+KeyS";

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
    pub capture_session_runtime: Arc<CaptureSessionRuntime>,
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

        let translation_coordinator = composition::build_translation_coordinator(
            config_file.clone(),
            keychain.clone(),
            http_client.clone(),
            event_bus.clone(),
        );
        let ocr_coordinator = composition::build_ocr_coordinator(
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
            capture_session_runtime,
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

    infrastructure::system::register_shortcut(app, SCREENSHOT_SHORTCUT, move || {
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

    infrastructure::system::register_shortcut(app, SCREENSHOT_OCR_SHORTCUT, move || {
        log::info!("Screenshot OCR shortcut triggered!");

        let app = app_clone.clone();
        tauri::async_runtime::spawn(commands::open_capture_window_from_shortcut(
            app,
            "screenshot-ocr",
        ));
    })?;

    Ok(())
}

/// Register pinned image shortcut
fn register_pin_shortcut(app: &tauri::AppHandle) -> Result<()> {
    let app_clone = app.clone();

    infrastructure::system::register_shortcut(app, "F3", move || {
        log::info!("Pinned image shortcut triggered!");

        let state = app_clone.state::<AppState>();
        if let Err(err) = commands::pin_clipboard_image_for_state(&app_clone, state.inner()) {
            log::error!("Failed to pin clipboard image: {}", err);
        }
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
    .plugin(tauri_plugin_global_shortcut::Builder::new().build())
    .plugin(tauri_plugin_screenshots::init())
    .plugin(tauri_plugin_log::Builder::default()
      .level(log::LevelFilter::Info)
      .build())
    .setup(|app| {
      let app_state = composition::build_app_state(config_path, app.handle().clone());
      composition::subscribe_history_service(&app_state);

      app.manage(app_state);

      // Register global shortcuts using Tauri plugin after setup completes
      let app_handle = app.handle().clone();
      tauri::async_runtime::spawn(async move {
          tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

          if let Err(e) = register_screenshot_shortcut(&app_handle) {
              log::error!("Failed to register screenshot shortcut: {}", e);
          } else {
              log::info!("Screenshot shortcut registered: {}", SCREENSHOT_SHORTCUT);
          }

          if let Err(e) = register_screenshot_ocr_shortcut(&app_handle) {
              log::error!("Failed to register screenshot OCR shortcut: {}", e);
          } else {
              log::info!("Screenshot OCR shortcut registered: {}", SCREENSHOT_OCR_SHORTCUT);
          }

          if let Err(e) = register_pin_shortcut(&app_handle) {
              log::error!("Failed to register pinned image shortcut: {}", e);
          } else {
              log::info!("Pinned image shortcut registered: F3");
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
      commands::open_translation_result_window,
      commands::trigger_screenshot,
      commands::open_capture_window,
      commands::create_screenshot_window,
      commands::create_screenshot_window_simple,
      commands::screenshot_overlay_ready,
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
      commands::restore_capture_snapshot_windows_for_session,
      commands::render_capture_output,
      commands::default_capture_save_path,
      commands::quick_capture_save_path,
      commands::output_capture,
      commands::pin_clipboard_image,
      commands::get_pinned_image,
      commands::copy_pinned_image,
      commands::replace_pinned_image_from_clipboard,
      commands::save_pinned_image,
      commands::close_pinned_image,
      commands::remove_pinned_image,
      commands::toggle_pinned_images_visibility,
      commands::switch_pinned_image_group,
      commands::move_pinned_image_to_next_group,
      commands::hide_pinned_image_group,
      commands::destroy_pinned_image_group,
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

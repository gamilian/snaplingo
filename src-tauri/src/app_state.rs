use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::Mutex as ParkingLotMutex;

use crate::application::providers::ocr::OcrCoordinator;
use crate::application::providers::translation::TranslationCoordinator;
use crate::application::{
    CaptureOutputService, CaptureService, CaptureSessionRuntime, CaptureSessionService,
    HistoryService, ImageCompositionService, PinnedImageService, WorkflowService,
};
use crate::composition;
use crate::infrastructure::events::EventBus;
use crate::infrastructure::http::{HttpClient, ReqwestHttpClient};
use crate::infrastructure::storage::{ConfigFile, HistoryDatabase, Keychain};
use crate::infrastructure::system::paths::get_history_db_path;
use crate::infrastructure::system::screenshot::get_screenshot_backend;
use crate::Result;

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
        let history_db = Arc::new(
            HistoryDatabase::new(history_db_path).expect("Failed to initialize history database"),
        );
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
        let drained = self
            .event_bus
            .drain(std::time::Duration::from_secs(5))
            .await;

        if !drained {
            log::warn!("Shutdown: Some events did not complete in time");
        }

        log::info!("Graceful shutdown complete");
        Ok(())
    }
}

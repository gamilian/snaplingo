use std::sync::Arc;

use parking_lot::Mutex as ParkingLotMutex;

use crate::application::providers::ocr::OcrCoordinator;
use crate::application::providers::translation::TranslationCoordinator;
use crate::application::{
    CaptureOutputService, CaptureService, CaptureSessionRuntime, CaptureSessionService,
    HistoryService, ImageCompositionService, PinnedImageService, SelectedTextAcquirer,
};
use crate::infrastructure::events::EventBus;
use crate::infrastructure::http::HttpClient;
use crate::infrastructure::storage::{ConfigFile, Keychain};
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

    // Phase 6: Selected text acquisition
    pub selected_text_acquirer: Arc<SelectedTextAcquirer>,
}

impl AppState {
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

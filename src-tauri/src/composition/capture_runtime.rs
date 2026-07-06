use std::sync::Arc;

use parking_lot::Mutex as ParkingLotMutex;
use tauri::AppHandle;

use crate::application::providers::ocr::OcrCoordinator;
use crate::application::services::capture_session_runtime::TauriCaptureSessionRuntimeHost;
use crate::infrastructure::system::screenshot::get_screenshot_backend;
use crate::{
    CaptureOutputService, CaptureService, CaptureSessionRuntime, CaptureSessionService,
    ImageCompositionService, PinnedImageService, ScreenshotState,
};

pub(crate) struct CaptureRuntimeParts {
    pub capture_service: Arc<CaptureService>,
    pub capture_session_service: Arc<CaptureSessionService>,
    pub image_composition_service: Arc<ImageCompositionService>,
    pub capture_output_service: Arc<CaptureOutputService>,
    pub capture_session_runtime: Arc<CaptureSessionRuntime>,
    pub pinned_image_service: Arc<PinnedImageService>,
    pub screenshot_state: Arc<ParkingLotMutex<ScreenshotState>>,
}

pub(crate) fn build_capture_runtime(
    app: AppHandle,
    ocr_coordinator: Arc<OcrCoordinator>,
) -> CaptureRuntimeParts {
    let screenshot_backend = get_screenshot_backend();
    let capture_service = Arc::new(CaptureService::new(screenshot_backend.clone()));
    let capture_session_service = Arc::new(CaptureSessionService::new(screenshot_backend));
    let image_composition_service = Arc::new(ImageCompositionService::new());
    let capture_output_service = Arc::new(CaptureOutputService::new());
    let capture_session_runtime = Arc::new(CaptureSessionRuntime::with_host(
        capture_session_service.clone(),
        image_composition_service.clone(),
        capture_output_service.clone(),
        ocr_coordinator,
        Arc::new(TauriCaptureSessionRuntimeHost::new(app)),
    ));
    let pinned_image_service = Arc::new(PinnedImageService::new());
    let screenshot_state = Arc::new(ParkingLotMutex::new(ScreenshotState::default()));

    CaptureRuntimeParts {
        capture_service,
        capture_session_service,
        image_composition_service,
        capture_output_service,
        capture_session_runtime,
        pinned_image_service,
        screenshot_state,
    }
}

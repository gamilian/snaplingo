use std::sync::Arc;

use tauri::AppHandle;

use crate::application::capture::CaptureImageComposer;
use crate::application::pinned_image::PinnedImageState;
use crate::application::providers::ocr::OcrCoordinator;
use crate::infrastructure::system::capture_window::TauriCaptureSessionRuntimeHost;
use crate::infrastructure::system::pinned_window::TauriPinnedImageRuntimeHost;
use crate::infrastructure::system::screenshot::get_capture_session_source;
use crate::{CaptureOutput, CaptureSessionRuntime, CaptureSessions, PinnedImageRuntime};

pub(crate) struct CaptureRuntimeParts {
    pub sessions: Arc<CaptureSessions>,
    pub output: Arc<CaptureOutput>,
    pub runtime: Arc<CaptureSessionRuntime>,
    pub pinned_images: Arc<PinnedImageRuntime>,
}

pub(crate) fn build_capture_runtime(
    app: AppHandle,
    ocr_coordinator: Arc<OcrCoordinator>,
) -> CaptureRuntimeParts {
    let capture_session_source = get_capture_session_source();
    let sessions = Arc::new(CaptureSessions::new(capture_session_source));
    let image_composer = Arc::new(CaptureImageComposer::new());
    let output = Arc::new(CaptureOutput::new());
    let runtime = Arc::new(CaptureSessionRuntime::with_host(
        sessions.clone(),
        image_composer.clone(),
        output.clone(),
        ocr_coordinator,
        Arc::new(TauriCaptureSessionRuntimeHost::new(app.clone())),
    ));
    let pinned_image_state = Arc::new(PinnedImageState::new());
    let pinned_images = Arc::new(PinnedImageRuntime::new(
        pinned_image_state,
        image_composer,
        output.clone(),
        Arc::new(TauriPinnedImageRuntimeHost::new(app)),
    ));

    CaptureRuntimeParts {
        sessions,
        output,
        runtime,
        pinned_images,
    }
}

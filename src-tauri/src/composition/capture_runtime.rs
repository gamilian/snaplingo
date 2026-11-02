use std::sync::Arc;

use tauri::AppHandle;

use crate::application::capture::{CaptureImageComposer, CaptureOcrDelivery, CaptureSessionSource};
use crate::application::pinned_image::PinnedImageState;
use crate::application::providers::ocr::OcrCoordinator;
use crate::application::result_window::ResultWindowRuntime;
use crate::infrastructure::system::capture_output::SystemCaptureOutputHost;
use crate::infrastructure::system::capture_window::{
    TauriCaptureOcrEffects, TauriCaptureSessionRuntimeHost,
};
use crate::infrastructure::system::pinned_window::TauriPinnedImageRuntimeHost;
#[cfg(target_os = "linux")]
use crate::infrastructure::system::screenshot::linux::LinuxCaptureSessionSource;
#[cfg(target_os = "macos")]
use crate::infrastructure::system::screenshot::macos::MacOSCaptureSessionSource;
#[cfg(target_os = "windows")]
use crate::infrastructure::system::screenshot::windows::WindowsCaptureSessionSource;
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
    result_window: Arc<ResultWindowRuntime>,
    settings: Arc<crate::application::settings::SettingsConfiguration>,
) -> CaptureRuntimeParts {
    let capture_session_source = build_capture_session_source();
    let scrolling_host = Arc::new(
        crate::infrastructure::system::scrolling_window::TauriScrollingCaptureHost::new(
            app.clone(),
            capture_session_source.clone(),
        ),
    );
    let sessions = Arc::new(CaptureSessions::new(capture_session_source));
    let image_composer = Arc::new(CaptureImageComposer::with_font_provider(Arc::new(
        crate::infrastructure::system::annotation_fonts::SystemAnnotationFonts::default(),
    )));
    let output = Arc::new(CaptureOutput::with_host(Arc::new(SystemCaptureOutputHost)));
    let pinned_image_state = Arc::new(PinnedImageState::new());
    let pinned_images = Arc::new(
        PinnedImageRuntime::new(
            pinned_image_state,
            image_composer.clone(),
            output.clone(),
            Arc::new(TauriPinnedImageRuntimeHost::new(app.clone())),
        )
        .with_settings(settings.clone()),
    );

    let runtime = Arc::new(
        CaptureSessionRuntime::with_host(
            sessions.clone(),
            image_composer.clone(),
            output.clone(),
            ocr_coordinator,
            Arc::new(TauriCaptureSessionRuntimeHost::new(
                app.clone(),
                Arc::new(CaptureOcrDelivery::new(
                    result_window,
                    Arc::new(TauriCaptureOcrEffects(app)),
                )),
            )),
            pinned_images.clone(),
        )
        .with_scrolling_host(scrolling_host)
        .with_settings(settings),
    );

    CaptureRuntimeParts {
        sessions,
        output,
        runtime,
        pinned_images,
    }
}

fn build_capture_session_source() -> Arc<dyn CaptureSessionSource> {
    #[cfg(target_os = "macos")]
    {
        Arc::new(MacOSCaptureSessionSource::new())
    }

    #[cfg(target_os = "windows")]
    {
        Arc::new(WindowsCaptureSessionSource::new())
    }

    #[cfg(target_os = "linux")]
    {
        Arc::new(LinuxCaptureSessionSource::new())
    }
}

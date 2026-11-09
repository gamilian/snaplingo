use std::sync::Arc;

#[cfg(target_os = "macos")]
use crate::application::capture::{CaptureImageComposer, CaptureOcrDelivery, CaptureSessionSource};
#[cfg(target_os = "macos")]
use crate::application::pinned_image::PinnedImageState;
#[cfg(target_os = "macos")]
use crate::application::providers::ocr::OcrCoordinator;
#[cfg(target_os = "macos")]
use crate::application::result_window::ResultWindowRuntime;
#[cfg(target_os = "macos")]
use crate::infrastructure::system::capture_output::SystemCaptureOutputHost;
#[cfg(target_os = "macos")]
use crate::infrastructure::system::capture_window::NativeCaptureOcrEffects;
#[cfg(target_os = "macos")]
use crate::infrastructure::system::capture_window::NativeCaptureSessionRuntimeHost;
#[cfg(target_os = "macos")]
use crate::infrastructure::system::screenshot::macos::MacOSCaptureSessionSource;
use crate::{CaptureOutput, CaptureSessionRuntime, CaptureSessions, PinnedImageRuntime};

pub(crate) struct CaptureRuntimeParts {
    pub sessions: Arc<CaptureSessions>,
    pub output: Arc<CaptureOutput>,
    pub runtime: Arc<CaptureSessionRuntime>,
    pub pinned_images: Arc<PinnedImageRuntime>,
    #[cfg(target_os = "macos")]
    pub capture_host: Arc<NativeCaptureSessionRuntimeHost>,
    #[cfg(target_os = "macos")]
    pub scrolling_host: Arc<crate::window::scrolling::macos::NativeScrollingCaptureHost>,
    #[cfg(all(target_os = "windows", feature = "native-app"))]
    pub native_capture_host:
        Arc<crate::infrastructure::system::capture_window::WindowsCaptureSessionRuntimeHost>,
    #[cfg(all(target_os = "windows", feature = "native-app"))]
    pub native_scrolling_host: Arc<crate::window::scrolling::windows::NativeScrollingCaptureHost>,
}

#[cfg(all(target_os = "windows", feature = "native-app"))]
pub(crate) fn build_native_windows_runtime(
    history_path: std::path::PathBuf,
    ocr: Arc<crate::application::providers::ocr::OcrCoordinator>,
    result: Arc<crate::application::result_window::ResultWindowRuntime>,
    settings: Arc<crate::SettingsConfiguration>,
    pins: Arc<crate::window::pinned::NativePinnedImageRuntimeHost>,
) -> CaptureRuntimeParts {
    use crate::application::{
        capture::{CaptureImageComposer, CaptureOcrDelivery},
        pinned_image::PinnedImageState,
    };
    use crate::infrastructure::system::{
        capture_output::SystemCaptureOutputHost,
        capture_window::{WindowsCaptureOcrEffects, WindowsCaptureSessionRuntimeHost},
        screenshot::windows::WindowsCaptureSessionSource,
    };
    use crate::window::scrolling::windows::NativeScrollingCaptureHost;
    let source = Arc::new(WindowsCaptureSessionSource::new());
    let scrolling = Arc::new(NativeScrollingCaptureHost::new(source.clone()));
    let sessions = Arc::new(CaptureSessions::new(source));
    let composer = Arc::new(CaptureImageComposer::with_font_provider(Arc::new(
        crate::infrastructure::system::annotation_fonts::SystemAnnotationFonts::default(),
    )));
    let output = Arc::new(CaptureOutput::with_host(Arc::new(SystemCaptureOutputHost)));
    let pinned_images = Arc::new(
        PinnedImageRuntime::new(
            Arc::new(PinnedImageState::new()),
            composer.clone(),
            output.clone(),
            pins,
        )
        .with_settings(settings.clone()),
    );
    let delivery = Arc::new(CaptureOcrDelivery::new(
        result,
        Arc::new(WindowsCaptureOcrEffects),
    ));
    let host = Arc::new(WindowsCaptureSessionRuntimeHost::new(
        delivery,
        history_path,
    ));
    let runtime = Arc::new(
        CaptureSessionRuntime::with_host(
            sessions.clone(),
            composer,
            output.clone(),
            ocr,
            host.clone(),
            pinned_images.clone(),
        )
        .with_scrolling_host(scrolling.clone())
        .with_settings(settings),
    );
    CaptureRuntimeParts {
        sessions,
        output,
        runtime,
        pinned_images,
        native_capture_host: host,
        native_scrolling_host: scrolling,
    }
}

#[cfg(target_os = "macos")]
pub(crate) fn build_capture_runtime(
    history_suite: String,
    ocr_coordinator: Arc<OcrCoordinator>,
    result_window: Arc<ResultWindowRuntime>,
    settings: Arc<crate::SettingsConfiguration>,
    native_pins: Arc<crate::window::pinned::NativePinnedImageRuntimeHost>,
) -> CaptureRuntimeParts {
    let capture_session_source = build_capture_session_source();
    #[cfg(target_os = "macos")]
    let scrolling_host = Arc::new(
        crate::window::scrolling::macos::NativeScrollingCaptureHost::new(
            capture_session_source.clone(),
        ),
    );
    let sessions = Arc::new(CaptureSessions::new(capture_session_source));
    let image_composer = Arc::new(CaptureImageComposer::with_font_provider(Arc::new(
        crate::infrastructure::system::annotation_fonts::SystemAnnotationFonts::default(),
    )));
    let output = Arc::new(CaptureOutput::with_host(Arc::new(SystemCaptureOutputHost)));
    let pinned_image_state = Arc::new(PinnedImageState::new());
    #[cfg(target_os = "macos")]
    let pinned_host = native_pins;
    let pinned_images = Arc::new(
        PinnedImageRuntime::new(
            pinned_image_state,
            image_composer.clone(),
            output.clone(),
            pinned_host,
        )
        .with_settings(settings.clone()),
    );

    #[cfg(target_os = "macos")]
    let effects: Arc<dyn crate::application::capture::CaptureOcrEffects> =
        Arc::new(NativeCaptureOcrEffects);
    let delivery = Arc::new(CaptureOcrDelivery::new(result_window, effects));
    #[cfg(target_os = "macos")]
    let capture_host = Arc::new(NativeCaptureSessionRuntimeHost::new(
        history_suite,
        delivery,
        crate::app_shell::set_capture_ocr_status_native,
    ));

    let runtime = Arc::new(
        CaptureSessionRuntime::with_host(
            sessions.clone(),
            image_composer.clone(),
            output.clone(),
            ocr_coordinator,
            capture_host.clone(),
            pinned_images.clone(),
        )
        .with_scrolling_host(scrolling_host.clone())
        .with_settings(settings),
    );

    CaptureRuntimeParts {
        sessions,
        output,
        runtime,
        pinned_images,
        #[cfg(target_os = "macos")]
        scrolling_host,
        #[cfg(target_os = "macos")]
        capture_host,
    }
}

#[cfg(target_os = "macos")]
fn build_capture_session_source() -> Arc<dyn CaptureSessionSource> {
    Arc::new(MacOSCaptureSessionSource::new())
}

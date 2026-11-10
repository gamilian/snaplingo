use std::sync::Arc;

#[cfg(target_os = "macos")]
use crate::features::capture::NativeCaptureOcrEffects;
#[cfg(target_os = "macos")]
use crate::features::capture::NativeCaptureSessionRuntimeHost;
#[cfg(target_os = "macos")]
use snaplingo_core::capture::CaptureImageComposer;
use snaplingo_core::capture::CaptureOcrDelivery;
use snaplingo_core::capture::CaptureOutput;
use snaplingo_core::capture::CaptureSessionRuntime;
use snaplingo_core::capture::CaptureSessionSource;
use snaplingo_core::capture::CaptureSessions;
use snaplingo_core::pinned::PinnedImageRuntime;
#[cfg(target_os = "macos")]
use snaplingo_core::pinned::PinnedImageState;
#[cfg(target_os = "macos")]
use snaplingo_core::providers::ocr::OcrCoordinator;
#[cfg(target_os = "macos")]
use snaplingo_core::result::ResultWindowRuntime;
#[cfg(target_os = "macos")]
use snaplingo_platform::capture_output::SystemCaptureOutputHost;
#[cfg(target_os = "macos")]
use snaplingo_platform::screenshot::macos::MacOSCaptureSessionSource;

pub(crate) struct CaptureRuntimeParts {
    pub sessions: Arc<CaptureSessions>,
    pub output: Arc<CaptureOutput>,
    pub runtime: Arc<CaptureSessionRuntime>,
    pub pinned_images: Arc<PinnedImageRuntime>,
    #[cfg(target_os = "macos")]
    pub capture_host: Arc<NativeCaptureSessionRuntimeHost>,
    #[cfg(target_os = "macos")]
    pub scrolling_host: Arc<crate::features::scrolling::macos::NativeScrollingCaptureHost>,
    #[cfg(all(target_os = "windows", feature = "native-app"))]
    pub native_capture_host: Arc<crate::features::capture::WindowsCaptureSessionRuntimeHost>,
    #[cfg(all(target_os = "windows", feature = "native-app"))]
    pub native_scrolling_host: Arc<crate::features::scrolling::windows::NativeScrollingCaptureHost>,
}

#[cfg(all(target_os = "windows", feature = "native-app"))]
pub(crate) fn build_native_windows_runtime(
    history_path: std::path::PathBuf,
    ocr: Arc<snaplingo_core::providers::ocr::OcrCoordinator>,
    result: Arc<snaplingo_core::result::ResultWindowRuntime>,
    settings: Arc<snaplingo_core::settings::SettingsConfiguration>,
    pins: Arc<crate::features::pinned::NativePinnedImageRuntimeHost>,
) -> CaptureRuntimeParts {
    use crate::features::capture::WindowsCaptureOcrEffects;
    use crate::features::capture::WindowsCaptureSessionRuntimeHost;
    use crate::features::scrolling::windows::NativeScrollingCaptureHost;
    use snaplingo_core::capture::CaptureImageComposer;
    use snaplingo_core::capture::CaptureOcrDelivery;
    use snaplingo_core::pinned::PinnedImageState;
    use snaplingo_platform::capture_output::SystemCaptureOutputHost;
    use snaplingo_platform::screenshot::windows::WindowsCaptureSessionSource;
    let source = Arc::new(WindowsCaptureSessionSource::new());
    let scrolling = Arc::new(NativeScrollingCaptureHost::new(source.clone()));
    let sessions = Arc::new(CaptureSessions::new(source));
    let composer = Arc::new(CaptureImageComposer::with_font_provider(Arc::new(
        snaplingo_platform::annotation_fonts::SystemAnnotationFonts::default(),
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
        crate::shell::shell::set_capture_ocr_status_native,
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
    settings: Arc<snaplingo_core::settings::SettingsConfiguration>,
    native_pins: Arc<crate::features::pinned::NativePinnedImageRuntimeHost>,
) -> CaptureRuntimeParts {
    let capture_session_source = build_capture_session_source();
    #[cfg(target_os = "macos")]
    let scrolling_host = Arc::new(
        crate::features::scrolling::macos::NativeScrollingCaptureHost::new(
            capture_session_source.clone(),
        ),
    );
    let sessions = Arc::new(CaptureSessions::new(capture_session_source));
    let image_composer = Arc::new(CaptureImageComposer::with_font_provider(Arc::new(
        snaplingo_platform::annotation_fonts::SystemAnnotationFonts::default(),
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
    let effects: Arc<dyn snaplingo_core::capture::CaptureOcrEffects> =
        Arc::new(NativeCaptureOcrEffects);
    let delivery = Arc::new(CaptureOcrDelivery::new(result_window, effects));
    #[cfg(target_os = "macos")]
    let capture_host = Arc::new(NativeCaptureSessionRuntimeHost::new(
        history_suite,
        delivery,
        crate::shell::shell::set_capture_ocr_status_native,
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

mod backend;
#[cfg(target_os = "macos")]
pub(crate) mod macos;
#[cfg(any(all(target_os = "windows", feature = "native-app"), test))]
mod preview;
mod shared_presentation;
#[cfg(all(target_os = "windows", feature = "native-app"))]
mod windows;
#[cfg(all(test, target_os = "macos"))]
#[allow(unused_imports)]
pub(crate) use macos::presentation::is_capture_presentation_active;
#[cfg(target_os = "macos")]
pub(crate) use macos::presentation::restore_capture_window_activation as yield_capture_window_focus;
#[cfg(target_os = "macos")]
pub(crate) use macos::{NativeCaptureOcrEffects, NativeCaptureSessionRuntimeHost};
#[cfg(all(target_os = "windows", feature = "native-app"))]
pub(crate) use windows::{
    windows_accessibility, windows_close_all, windows_scrolling_ended, windows_window_event,
    yield_capture_window_focus, WindowsCaptureOcrEffects, WindowsCaptureSessionRuntimeHost,
};
#[cfg(target_os = "macos")]
pub fn hide_capture_window() -> Result<(), String> {
    if let Some(id) = macos::view::session_id() {
        macos::view::suspend(&id);
    }
    Ok(())
}

#[derive(Clone)]
pub(crate) struct Services {
    pub runtime: std::sync::Weak<snaplingo_core::capture::CaptureSessionRuntime>,
    pub sessions: std::sync::Arc<snaplingo_core::capture::CaptureSessions>,
    pub settings: std::sync::Arc<snaplingo_core::settings::SettingsConfiguration>,
    pub output: std::sync::Arc<snaplingo_core::capture::CaptureOutput>,
    pub cursor: std::sync::Arc<dyn snaplingo_core::capture::CaptureCursorMover>,
    pub favorites:
        std::sync::Weak<snaplingo_core::library::screenshot_favorites::ScreenshotFavoriteCapture>,
}
impl Services {
    fn runtime(
        &self,
    ) -> crate::Result<std::sync::Arc<snaplingo_core::capture::CaptureSessionRuntime>> {
        self.runtime
            .upgrade()
            .ok_or_else(|| "Capture runtime has stopped".into())
    }
}

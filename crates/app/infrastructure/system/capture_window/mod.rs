mod backend;
#[cfg(any(all(target_os = "windows", feature = "native-app"), test))]
mod shared_presentation;
#[cfg(all(target_os = "windows", feature = "native-app"))]
mod windows;
#[cfg(all(target_os = "windows", feature = "native-app"))]
pub(crate) use windows::{
    windows_accessibility, windows_close_all, windows_scrolling_ended, windows_window_event,
    yield_capture_window_focus, WindowsCaptureOcrEffects, WindowsCaptureSessionRuntimeHost,
};
#[cfg(target_os = "macos")]
pub(crate) mod editor_canvas_macos;
#[cfg(target_os = "macos")]
pub(crate) mod editor_style_macos;
#[cfg(target_os = "macos")]
pub(crate) mod editor_toolbar_macos;
#[cfg(target_os = "macos")]
mod native_host;
#[cfg(target_os = "macos")]
mod native_ocr_effects;
#[cfg(target_os = "macos")]
pub(crate) mod native_view;
#[cfg(target_os = "macos")]
mod presentation_macos;

pub use backend::capture_snapshot_hide_settle_delay_ms;
#[cfg(target_os = "macos")]
pub(crate) use native_host::NativeCaptureSessionRuntimeHost;
#[cfg(target_os = "macos")]
pub(crate) use native_ocr_effects::NativeCaptureOcrEffects;
#[cfg(target_os = "macos")]
pub(crate) use presentation_macos::restore_capture_window_activation as yield_capture_window_focus;
#[cfg(target_os = "macos")]
pub use presentation_macos::{
    begin_capture_presentation, end_capture_presentation, is_capture_presentation_active,
};
#[cfg(target_os = "macos")]
pub fn hide_capture_window() -> Result<(), String> {
    if let Some(id) = native_view::session_id() {
        native_view::suspend(&id);
    }
    Ok(())
}

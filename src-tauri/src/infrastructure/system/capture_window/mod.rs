mod backend;
#[cfg(target_os = "macos")]
pub(crate) mod editor_style_macos;
#[cfg(target_os = "macos")]
pub(crate) mod editor_toolbar_macos;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
mod native_host;
#[cfg(target_os = "macos")]
pub(crate) mod native_view;
mod runtime_host;
mod tauri;

pub use backend::capture_snapshot_hide_settle_delay_ms;
#[cfg(target_os = "macos")]
pub(crate) use native_host::NativeCaptureSessionRuntimeHost;
pub(crate) use runtime_host::TauriCaptureOcrEffects;
#[cfg(any(not(target_os = "macos"), test))]
#[cfg_attr(test, allow(unused_imports))]
pub(crate) use runtime_host::TauriCaptureSessionRuntimeHost;
pub(crate) use tauri::yield_capture_window_focus;
pub use tauri::{
    begin_capture_presentation, destroy_inactive_capture_window, end_capture_presentation,
    hide_capture_snapshot_windows, hide_capture_window, is_capture_presentation_active,
    prepare_capture_window_for_reveal, prewarm_capture_window, restore_capture_snapshot_windows,
    reveal_capture_window, wait_for_capture_window_ready,
};
pub(crate) mod image_protocol;

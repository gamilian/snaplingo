mod backend;
#[cfg(target_os = "macos")]
mod macos;
mod runtime_host;
mod tauri;

pub use backend::{capture_snapshot_hide_settle_delay_ms, capture_window_bounds};
pub(crate) use runtime_host::TauriCaptureSessionRuntimeHost;
pub use tauri::{
    begin_capture_presentation, destroy_inactive_capture_window, end_capture_presentation,
    hide_capture_snapshot_windows, hide_capture_window, is_capture_presentation_active,
    open_capture_window_for_session, prepare_capture_window_for_reveal, prewarm_capture_window,
    restore_capture_snapshot_windows, reveal_capture_window,
};

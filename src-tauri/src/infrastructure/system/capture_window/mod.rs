mod backend;
#[cfg(target_os = "macos")]
mod macos;
mod tauri;

pub use backend::{capture_snapshot_hide_settle_delay_ms, capture_window_bounds};
pub use tauri::{
    begin_capture_presentation, end_capture_presentation, hide_capture_snapshot_windows,
    open_capture_window_for_session, restore_capture_snapshot_windows, reveal_capture_window,
};

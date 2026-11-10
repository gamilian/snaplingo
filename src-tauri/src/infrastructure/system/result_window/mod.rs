mod backend;
pub(crate) mod drag;
#[cfg(target_os = "macos")]
mod drag_macos;
mod runtime_host;
mod tauri;

pub(crate) use runtime_host::{TauriResultWindowNotifier, TauriResultWindowRuntimeHost};
pub use tauri::show_or_create_result_window;

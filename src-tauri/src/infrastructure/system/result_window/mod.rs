mod backend;
mod runtime_host;
mod tauri;

pub(crate) use runtime_host::{TauriResultWindowNotifier, TauriResultWindowRuntimeHost};
pub use tauri::show_or_create_result_window;

mod backend;
pub(crate) mod drag;
#[cfg(target_os = "macos")]
mod drag_macos;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub(crate) mod native_view;
mod runtime_host;
#[cfg(any(not(target_os = "macos"), test))]
#[cfg_attr(target_os = "macos", allow(dead_code))]
mod tauri;
#[cfg(target_os = "macos")]
mod tokens_macos;

#[cfg(target_os = "macos")]
pub use macos::show_or_create_result_window;
#[cfg(target_os = "macos")]
pub(crate) use macos::NativeResultWindowPresentation;
#[cfg(target_os = "macos")]
pub(crate) use native_view::{
    has_key_window as has_key_native_window,
    visible_on_active_space as has_visible_native_window_on_active_space,
};
pub(crate) use runtime_host::{TauriResultWindowNotifier, TauriResultWindowRuntimeHost};
#[cfg(not(target_os = "macos"))]
pub use tauri::show_or_create_result_window;

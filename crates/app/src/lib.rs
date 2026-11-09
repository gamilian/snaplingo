pub(crate) mod window;
// Module declarations
mod app_actions;
mod app_shell;
mod app_state;
mod composition;
pub use snaplingo_core::{application, domain, error};
mod infrastructure;
#[cfg(all(target_os = "macos", feature = "native-app"))]
mod native_app;
#[cfg(all(target_os = "macos", feature = "native-app"))]
mod native_capacity;
#[cfg(all(target_os = "windows", feature = "native-app"))]
mod native_capacity_windows;
#[cfg(all(target_os = "windows", feature = "native-app"))]
mod native_notices_windows;
#[cfg(all(target_os = "macos", feature = "native-app"))]
mod native_permission_prompt;
#[cfg(all(target_os = "macos", feature = "native-app"))]
mod native_settings_window;
#[cfg(all(target_os = "windows", feature = "native-app"))]
mod native_settings_windows;
#[cfg(all(target_os = "windows", feature = "native-app"))]
mod native_windows;
mod runtime_lifecycle;
mod startup_shortcuts;

// Public exports for new infrastructure layer
pub use app_state::AppState;
#[allow(ambiguous_glob_reexports)]
pub use application::*;
#[cfg(target_os = "macos")]
pub use composition::{
    build_app_state as build_native_app_state, CompositionOptions as NativeApplicationOptions,
};
#[allow(ambiguous_glob_reexports)]
pub use domain::*;
pub use error::{AppError, Result};
#[allow(ambiguous_glob_reexports)]
pub use infrastructure::*;
#[cfg(all(target_os = "macos", feature = "native-app"))]
pub use native_app::{
    run as run_native, run_with_options as run_native_with_options, NativeAppOptions,
};
#[cfg(all(target_os = "windows", feature = "native-app"))]
pub use native_windows::{
    run as run_native, run_with_options as run_native_with_options, NativeAppOptions,
};
pub use runtime_lifecycle::{should_show_settings_on_startup, RuntimeLifecycle};

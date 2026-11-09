pub(crate) mod window;
// Module declarations
mod actions;
mod app_state;
mod composition;
pub use snaplingo_core::{application, domain, error};
mod infrastructure;
mod lifecycle;

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
pub use lifecycle::macos::{
    run as run_native, run_with_options as run_native_with_options, NativeAppOptions,
};
#[cfg(all(target_os = "windows", feature = "native-app"))]
pub use lifecycle::windows::{
    run as run_native, run_with_options as run_native_with_options, NativeAppOptions,
};
pub use lifecycle::{should_show_settings_on_startup, RuntimeLifecycle};

//! Native desktop product: composition, resident shell and feature hosts.
mod bootstrap;
pub mod features;
pub mod integrations;
mod shell;

#[cfg(feature = "test-support")]
#[doc(hidden)]
pub mod test_support;

pub use bootstrap::state::AppState;
#[cfg(target_os = "macos")]
pub use bootstrap::{
    build_app_state as build_native_app_state, CompositionOptions as NativeApplicationOptions,
};
#[cfg(all(target_os = "macos", feature = "native-app"))]
pub use shell::macos::{
    run as run_native, run_with_options as run_native_with_options, NativeAppOptions,
};
#[cfg(all(target_os = "windows", feature = "native-app"))]
pub use shell::windows::{
    run as run_native, run_with_options as run_native_with_options, NativeAppOptions,
};
pub use shell::{should_show_settings_on_startup, RuntimeLifecycle};
pub use snaplingo_core::{AppError, Result};

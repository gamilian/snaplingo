//! Runs the standalone application with an isolated database on AppKit's thread.
#![allow(dead_code, unused_imports, ambiguous_glob_reexports)]
#[cfg(any(
    target_os = "macos",
    all(target_os = "windows", feature = "native-app")
))]
#[path = "../src/app_actions.rs"]
mod app_actions;
#[cfg(any(
    target_os = "macos",
    all(target_os = "windows", feature = "native-app")
))]
#[path = "../src/app_shell.rs"]
mod app_shell;
#[cfg(any(
    target_os = "macos",
    all(target_os = "windows", feature = "native-app")
))]
#[path = "../src/app_state.rs"]
mod app_state;
#[cfg(any(
    target_os = "macos",
    all(target_os = "windows", feature = "native-app")
))]
#[cfg(target_os = "macos")]
mod composition {
    include!(concat!(env!("OUT_DIR"), "/native_ui_composition.rs"));
}
#[cfg(any(
    target_os = "macos",
    all(target_os = "windows", feature = "native-app")
))]
#[path = "../src/infrastructure/mod.rs"]
mod infrastructure;
#[cfg(target_os = "macos")]
#[path = "../src/native_app.rs"]
mod native_app;
#[cfg(target_os = "macos")]
#[path = "../src/native_capacity.rs"]
mod native_capacity;
#[cfg(target_os = "macos")]
#[path = "../src/native_permission_prompt.rs"]
mod native_permission_prompt;
#[cfg(target_os = "macos")]
#[path = "../src/native_settings_window.rs"]
mod native_settings_window;
#[cfg(any(
    target_os = "macos",
    all(target_os = "windows", feature = "native-app")
))]
#[path = "../src/runtime_lifecycle.rs"]
mod runtime_lifecycle;
#[cfg(any(
    target_os = "macos",
    all(target_os = "windows", feature = "native-app")
))]
#[cfg(any(
    target_os = "macos",
    all(target_os = "windows", feature = "native-app")
))]
#[path = "../src/startup_shortcuts.rs"]
mod startup_shortcuts;
#[cfg(any(
    target_os = "macos",
    all(target_os = "windows", feature = "native-app")
))]
#[path = "../src/window/mod.rs"]
mod window;
#[cfg(any(
    target_os = "macos",
    all(target_os = "windows", feature = "native-app")
))]
pub use app_state::AppState;
#[cfg(any(
    target_os = "macos",
    all(target_os = "windows", feature = "native-app")
))]
pub use application::*;
#[cfg(any(
    target_os = "macos",
    all(target_os = "windows", feature = "native-app")
))]
pub use domain::*;
#[cfg(any(
    target_os = "macos",
    all(target_os = "windows", feature = "native-app")
))]
pub use error::{AppError, Result};
#[cfg(any(
    target_os = "macos",
    all(target_os = "windows", feature = "native-app")
))]
pub use infrastructure::*;
#[cfg(any(
    target_os = "macos",
    all(target_os = "windows", feature = "native-app")
))]
pub use runtime_lifecycle::{should_show_settings_on_startup, RuntimeLifecycle};
#[cfg(any(
    target_os = "macos",
    all(target_os = "windows", feature = "native-app")
))]
pub use snaplingo_core::{application, domain, error};

#[cfg(target_os = "macos")]
fn main() {
    if std::env::args().any(|arg| arg == "--permissions") {
        native_app::permission_smoke_test();
    } else {
        native_app::smoke_test();
    }
}
#[cfg(all(target_os = "windows", feature = "native-app"))]
fn main() {
    native_windows::smoke_test();
}
#[cfg(not(any(
    target_os = "macos",
    all(target_os = "windows", feature = "native-app")
)))]
fn main() {}

#[cfg(all(target_os = "windows", feature = "native-app"))]
mod composition {
    include!(concat!(env!("OUT_DIR"), "/native_ui_composition.rs"));
}
#[cfg(all(target_os = "windows", feature = "native-app"))]
#[path = "../src/native_capacity_windows.rs"]
mod native_capacity_windows;
#[cfg(all(target_os = "windows", feature = "native-app"))]
#[path = "../src/native_notices_windows.rs"]
mod native_notices_windows;
#[cfg(all(target_os = "windows", feature = "native-app"))]
#[path = "../src/native_settings_windows.rs"]
mod native_settings_windows;
#[cfg(all(target_os = "windows", feature = "native-app"))]
#[path = "../src/native_windows.rs"]
mod native_windows;

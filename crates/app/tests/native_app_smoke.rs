//! Runs the standalone application with an isolated database on AppKit's thread.
#![allow(dead_code, unused_imports, ambiguous_glob_reexports)]
#[cfg(any(
    target_os = "macos",
    all(target_os = "windows", feature = "native-app")
))]
#[path = "../actions/mod.rs"]
mod actions;
#[cfg(any(
    target_os = "macos",
    all(target_os = "windows", feature = "native-app")
))]
use lifecycle::shell;
#[cfg(any(
    target_os = "macos",
    all(target_os = "windows", feature = "native-app")
))]
#[path = "../app_state.rs"]
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
#[path = "../infrastructure/mod.rs"]
mod infrastructure;
#[cfg(target_os = "macos")]
use lifecycle::macos as native_app;
#[cfg(any(
    target_os = "macos",
    all(target_os = "windows", feature = "native-app")
))]
#[path = "../lifecycle/mod.rs"]
mod lifecycle;
#[cfg(any(
    target_os = "macos",
    all(target_os = "windows", feature = "native-app")
))]
#[cfg(any(
    target_os = "macos",
    all(target_os = "windows", feature = "native-app")
))]
use actions::shortcuts;
#[cfg(any(
    target_os = "macos",
    all(target_os = "windows", feature = "native-app")
))]
#[path = "../window/mod.rs"]
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
pub use lifecycle::{should_show_settings_on_startup, RuntimeLifecycle};
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
use lifecycle::windows as native_windows;

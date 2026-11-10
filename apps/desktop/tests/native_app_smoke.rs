//! Runs the standalone application with an isolated database on AppKit's thread.
#![allow(dead_code, unused_imports, ambiguous_glob_reexports)]
#[cfg(any(
    target_os = "macos",
    all(target_os = "windows", feature = "native-app")
))]
#[cfg(any(
    target_os = "macos",
    all(target_os = "windows", feature = "native-app")
))]
use shell::shell as resident;
#[cfg(any(
    target_os = "macos",
    all(target_os = "windows", feature = "native-app")
))]
#[cfg(any(
    target_os = "macos",
    all(target_os = "windows", feature = "native-app")
))]
#[cfg(target_os = "macos")]
mod bootstrap {
    include!(concat!(env!("OUT_DIR"), "/native_ui_composition.rs"));
}
#[cfg(any(
    target_os = "macos",
    all(target_os = "windows", feature = "native-app")
))]
#[path = "../src/integrations/mod.rs"]
mod integrations;
#[cfg(target_os = "macos")]
use shell::macos as native_app;
#[cfg(any(
    target_os = "macos",
    all(target_os = "windows", feature = "native-app")
))]
#[path = "../src/shell/mod.rs"]
mod shell;
#[cfg(any(
    target_os = "macos",
    all(target_os = "windows", feature = "native-app")
))]
#[cfg(any(
    target_os = "macos",
    all(target_os = "windows", feature = "native-app")
))]
use shell::actions::shortcuts;
#[cfg(any(
    target_os = "macos",
    all(target_os = "windows", feature = "native-app")
))]
#[path = "../src/features/mod.rs"]
mod features;
#[cfg(any(
    target_os = "macos",
    all(target_os = "windows", feature = "native-app")
))]
pub use bootstrap::state::AppState;
#[cfg(any(
    target_os = "macos",
    all(target_os = "windows", feature = "native-app")
))]
pub use error::{AppError, Result};
#[cfg(any(
    target_os = "macos",
    all(target_os = "windows", feature = "native-app")
))]
pub use integrations::*;
#[cfg(any(
    target_os = "macos",
    all(target_os = "windows", feature = "native-app")
))]
pub use shell::{should_show_settings_on_startup, RuntimeLifecycle};
#[cfg(any(
    target_os = "macos",
    all(target_os = "windows", feature = "native-app")
))]
pub use snaplingo_core::error;

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
mod bootstrap {
    include!(concat!(env!("OUT_DIR"), "/native_ui_composition.rs"));
}
#[cfg(all(target_os = "windows", feature = "native-app"))]
use shell::windows as native_windows;

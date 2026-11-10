//! Native window integration and application-specific desktop effects.
//! System capabilities live in `snaplingo-platform` and are imported directly.
pub(crate) mod dialogs;
pub(crate) mod external_open;
#[cfg(any(all(target_os = "windows", feature = "native-app"), test))]
pub(crate) mod image_print_windows;
#[cfg(target_os = "macos")]
pub(crate) mod image_save_dialog;
#[cfg(all(target_os = "windows", feature = "native-app"))]
pub(crate) mod layered_windows;
#[cfg(target_os = "macos")]
pub(crate) mod native_theme;
#[cfg(all(target_os = "macos", feature = "test-support"))]
pub(crate) mod native_view_snapshot;
#[cfg(target_os = "macos")]
pub mod shortcut;
pub mod updates;

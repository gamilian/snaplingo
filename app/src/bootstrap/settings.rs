//! Shared-drawing adapters call the same application workflows on either OS.
#[path = "settings/library.rs"]
pub(crate) mod library;
#[path = "settings/services.rs"]
pub(crate) mod services;

#[path = "settings/general.rs"]
pub(crate) mod general;
#[path = "settings/window.rs"]
pub(crate) mod window;
#[cfg(all(test, target_os = "macos"))]
#[path = "settings/window_tests.rs"]
mod window_tests;

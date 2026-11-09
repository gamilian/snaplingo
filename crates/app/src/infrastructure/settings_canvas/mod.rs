//! Shared-drawing adapters call the same application workflows on either OS.
pub(crate) mod capacity_usage;
pub(crate) mod library;
pub(crate) mod services;

pub(crate) mod general;
pub(crate) mod general_controller;
#[cfg(any(target_os = "macos", test))]
pub(crate) mod permission_prompt;
pub(crate) mod preferences;
pub(crate) mod window;
#[cfg(all(test, target_os = "macos"))]
mod window_tests;

#[cfg(any(all(target_os = "windows", feature = "native-app"), test))]
pub(crate) mod capacity;

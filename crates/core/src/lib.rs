//! Platform-independent application services and their inward adapter contracts.
pub mod application;
pub mod domain;
pub mod error;

#[allow(ambiguous_glob_reexports)]
pub use application::*;
#[allow(ambiguous_glob_reexports)]
pub use domain::*;
pub use error::{AppError, Result};

// Existing workflow tests exercise real portable persistence/network adapters.
// Loading their sources only in tests keeps production dependency direction
// inward while preserving access to workflow-private behavior during extraction.
#[cfg(test)]
extern crate self as snaplingo_core;
#[cfg(test)]
#[path = "test_infrastructure.rs"]
mod infrastructure;

/// Fixtures for adapter contract tests; absent from ordinary application builds.
#[cfg(feature = "test-support")]
pub mod test_support {
    use crate::application::{
        hotkeys::HotkeyConfiguration,
        settings::{SettingsConfiguration, SettingsStore},
    };
    use std::{path::PathBuf, sync::Arc};

    pub fn settings_configuration(
        store: Arc<dyn SettingsStore>,
        home: Option<PathBuf>,
        screenshots: PathBuf,
    ) -> SettingsConfiguration {
        SettingsConfiguration::with_paths(store, home, screenshots)
    }

    pub fn reset_hotkey_category(
        configuration: &HotkeyConfiguration,
        category: &str,
    ) -> crate::Result<crate::domain::HotkeySettingsSnapshot> {
        configuration.reset_category(category)
    }
}

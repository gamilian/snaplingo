//! Platform-independent application services and their inward adapter contracts.
pub mod application;
pub mod domain;
pub mod error;

#[allow(ambiguous_glob_reexports)]
pub use application::*;
#[allow(ambiguous_glob_reexports)]
pub use domain::*;
pub use error::{AppError, Result};

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

#[cfg(test)]
mod test_support_config;

#[cfg(test)]
mod test_support_credentials;

//! Portable business modules. Each feature owns its models, workflows and consumed ports.
pub mod capture;
pub mod error;
pub mod hotkeys;
pub mod library;
pub mod permissions;
pub mod pinned;
pub mod providers;
pub mod result;
pub mod selection;
pub mod settings;
pub mod speech;
pub mod updates;

pub use error::{AppError, Result};

/// Fixtures for adapter contract tests; absent from ordinary application builds.
#[cfg(feature = "test-support")]
pub mod test_support {
    use crate::hotkeys::HotkeyConfiguration;
    use crate::settings::SettingsConfiguration;
    use crate::settings::SettingsStore;
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
    ) -> crate::Result<crate::hotkeys::model::HotkeySettingsSnapshot> {
        configuration.reset_category(category)
    }
}

#[cfg(test)]
mod test_support_config;

#[cfg(test)]
mod test_support_credentials;

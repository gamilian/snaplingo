//! Isolated native preferences acceptance harness with real SQLite persistence.
//! Hotkeys use a recording-only registrar; no global shortcuts are installed.
use snaplingo_core::domain::SettingsSnapshot;
#[path = "support/preferences.rs"]
mod preferences;
use preferences::{PersistentSettings, RecordingOnlyRegistrar};
use snaplingo_storage::{Database, SqliteConfigStore};
use std::sync::Arc;

fn main() -> Result<(), String> {
    let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
    let path = directory.path().join("preferences.db");
    let store = Arc::new(SqliteConfigStore::new(Arc::new(
        Database::open(&path).map_err(|error| error.to_string())?,
    )));
    let mut settings = SettingsSnapshot::default();
    for argument in std::env::args().skip(1) {
        match argument.as_str() {
            "--studio" => settings.general.interface_theme = "studio".into(),
            "--paper" => settings.general.interface_theme = "paper".into(),
            "--dark" => settings.general.theme = "dark".into(),
            _ => return Err(format!("Unknown preview argument: {argument}")),
        }
    }
    store
        .save_settings(&settings)
        .map_err(|error| error.to_string())?;
    snaplingo_desktop::run_window_backend(PersistentSettings::open(
        &path,
        Arc::new(RecordingOnlyRegistrar),
    )?)
}

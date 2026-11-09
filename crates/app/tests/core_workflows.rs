//! Tests cross real persistence and filesystem adapters through Core interfaces.
#![allow(unused_imports, dead_code)]
pub use snaplingo_core::*;
pub use snaplingo_core::{application, domain, error, AppError, Result};
use std::sync::Arc;
mod infrastructure {
    pub use snaplingo_storage as storage;
}
fn history_repository() -> Result<snaplingo_storage::SqliteHistoryRepository> {
    Ok(snaplingo_storage::SqliteHistoryRepository::new(Arc::new(
        snaplingo_storage::Database::in_memory()?,
    )))
}
struct Quiet;
impl application::hotkeys::HotkeyChangeNotifier for Quiet {
    fn hotkeys_changed(&self) {}
}
fn hotkey_runtime(
    configuration: Arc<application::HotkeyConfiguration>,
    registrar: Arc<dyn application::hotkeys::HotkeyRegistrar>,
) -> application::HotkeyRuntime {
    application::HotkeyRuntime::with_change_notifier(configuration, registrar, Arc::new(Quiet))
}
struct Unlimited;
impl application::favorite_capacity::FavoriteCapacityPolicyProvider for Unlimited {
    fn maximum_favorites(&self) -> Result<u32> {
        Ok(0)
    }
}
fn unlimited_capacity() -> application::favorite_capacity::FavoriteCapacity {
    application::favorite_capacity::FavoriteCapacity::new(
        Arc::new(snaplingo_storage::SqliteFavoriteCapacityRepository::new(
            Arc::new(snaplingo_storage::Database::in_memory().unwrap()),
        )),
        Arc::new(Unlimited),
    )
}
struct FilesystemImageReader;
#[async_trait::async_trait]
impl application::providers::ocr::ImageFileReader for FilesystemImageReader {
    async fn read(&self, path: &str) -> Result<Vec<u8>> {
        Ok(tokio::fs::read(path).await?)
    }
}
#[path = "core_workflows/favorites.rs"]
mod favorites;
#[path = "core_workflows/history.rs"]
mod history;
#[path = "core_workflows/hotkeys_runtime_hotkey_runtime.rs"]
mod hotkeys_runtime_hotkey_runtime;
#[path = "core_workflows/providers_configuration_provider_configuration.rs"]
mod providers_configuration_provider_configuration;
#[path = "core_workflows/providers_llm_introspection.rs"]
mod providers_llm_introspection;
#[path = "core_workflows/providers_ocr_file_input.rs"]
mod providers_ocr_file_input;
#[path = "core_workflows/result_window_session.rs"]
mod result_window_session;
#[path = "core_workflows/settings_feature_preferences.rs"]
mod settings_feature_preferences;
#[path = "core_workflows/settings_services.rs"]
mod settings_services;
#[path = "core_workflows/settings_translation_services.rs"]
mod settings_translation_services;

//! Tests cross real persistence and filesystem adapters through Core interfaces.
#![allow(unused_imports, dead_code)]
pub use snaplingo_core::capture::model::CaptureConfig;
pub use snaplingo_core::capture::model::CaptureMode;
pub use snaplingo_core::capture::model::CaptureRegion;
pub use snaplingo_core::capture::model::ImageFormat;
pub use snaplingo_core::capture::CaptureCursorMover;
pub use snaplingo_core::capture::CaptureOutput;
pub use snaplingo_core::capture::CaptureSessionRuntime;
pub use snaplingo_core::capture::CaptureSessionSource;
pub use snaplingo_core::capture::CaptureSessions;
pub use snaplingo_core::hotkeys::model::HotkeySettingsSnapshot;
pub use snaplingo_core::hotkeys::HotkeyConfiguration;
pub use snaplingo_core::hotkeys::HotkeyRecording;
pub use snaplingo_core::hotkeys::HotkeyRuntime;
pub use snaplingo_core::hotkeys::HotkeyUpdateOutcome;
pub use snaplingo_core::library::capacity::FavoriteCapacity;
pub use snaplingo_core::library::favorites::Favorites;
pub use snaplingo_core::library::favorites::OcrFavoriteApplication;
pub use snaplingo_core::library::history::History;
pub use snaplingo_core::library::history::OcrHistoryReplay;
pub use snaplingo_core::library::index::LibraryIndex;
pub use snaplingo_core::library::screenshot_favorites::ScreenshotFavoriteCapture;
pub use snaplingo_core::library::screenshot_favorites::ScreenshotFavorites;
pub use snaplingo_core::permissions::RequiredPermissions;
pub use snaplingo_core::permissions::RequiredPermissionsContext;
pub use snaplingo_core::permissions::RequiredPermissionsHost;
pub use snaplingo_core::permissions::RequiredPermissionsStatus;
pub use snaplingo_core::permissions::SystemPermission;
pub use snaplingo_core::pinned::PinnedImageRuntime;
pub use snaplingo_core::providers::events::DomainEvent;
pub use snaplingo_core::providers::ocr::model::OcrBoundingBox;
pub use snaplingo_core::providers::ocr::model::OcrLine;
pub use snaplingo_core::providers::ocr::model::OcrRequest;
pub use snaplingo_core::providers::ocr::model::OcrResult;
pub use snaplingo_core::providers::translation::model::TranslationError;
pub use snaplingo_core::providers::translation::model::TranslationRequest;
pub use snaplingo_core::providers::translation::model::TranslationResult;
pub use snaplingo_core::providers::Provider;
pub use snaplingo_core::selection::model::FrontmostApp;
pub use snaplingo_core::selection::model::MethodAvailability;
pub use snaplingo_core::selection::model::SelectedTextSnapshot;
pub use snaplingo_core::selection::model::SelectionAttempt;
pub use snaplingo_core::selection::model::SelectionAttemptStatus;
pub use snaplingo_core::selection::model::SelectionContext;
pub use snaplingo_core::selection::model::SelectionMethodKind;
pub use snaplingo_core::selection::model::SelectionSource;
pub use snaplingo_core::selection::SelectedTextAcquirer;
pub use snaplingo_core::selection::SelectionContextProvider;
pub use snaplingo_core::selection::SelectionMethod;
pub use snaplingo_core::selection::SelectionScheme;
pub use snaplingo_core::selection::SelectionTextMode;
pub use snaplingo_core::selection::SystemSelectionProvider;
pub use snaplingo_core::settings::model::GeneralSettings;
pub use snaplingo_core::settings::model::HistorySettings;
pub use snaplingo_core::settings::model::OcrSettings;
pub use snaplingo_core::settings::model::ScreenshotSettings;
pub use snaplingo_core::settings::model::SettingsSnapshot;
pub use snaplingo_core::settings::model::TranslationSettings;
pub use snaplingo_core::settings::SettingsApplication;
pub use snaplingo_core::settings::SettingsConfiguration;
pub use snaplingo_core::settings::StartOnBoot;
pub use snaplingo_core::speech::SystemTtsHost;
pub use snaplingo_core::speech::SystemTtsVoice;
pub use snaplingo_core::speech::TtsRuntime;
pub use snaplingo_core::{error, AppError, Result};
use std::sync::Arc;
mod integrations {
    pub use snaplingo_storage as storage;
}
fn history_repository() -> Result<snaplingo_storage::SqliteHistoryRepository> {
    Ok(snaplingo_storage::SqliteHistoryRepository::new(Arc::new(
        snaplingo_storage::Database::in_memory()?,
    )))
}
struct Quiet;
impl snaplingo_core::hotkeys::HotkeyChangeNotifier for Quiet {
    fn hotkeys_changed(&self) {}
}
fn hotkey_runtime(
    configuration: Arc<snaplingo_core::hotkeys::HotkeyConfiguration>,
    registrar: Arc<dyn snaplingo_core::hotkeys::HotkeyRegistrar>,
) -> snaplingo_core::hotkeys::HotkeyRuntime {
    snaplingo_core::hotkeys::HotkeyRuntime::with_change_notifier(
        configuration,
        registrar,
        Arc::new(Quiet),
    )
}
struct Unlimited;
impl snaplingo_core::library::capacity::FavoriteCapacityPolicyProvider for Unlimited {
    fn maximum_favorites(&self) -> Result<u32> {
        Ok(0)
    }
}
fn unlimited_capacity() -> snaplingo_core::library::capacity::FavoriteCapacity {
    snaplingo_core::library::capacity::FavoriteCapacity::new(
        Arc::new(snaplingo_storage::SqliteFavoriteCapacityRepository::new(
            Arc::new(snaplingo_storage::Database::in_memory().unwrap()),
        )),
        Arc::new(Unlimited),
    )
}
struct FilesystemImageReader;
#[async_trait::async_trait]
impl snaplingo_core::providers::ocr::ImageFileReader for FilesystemImageReader {
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

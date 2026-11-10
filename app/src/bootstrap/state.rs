use std::sync::Arc;

use crate::integrations::events::EventBus;
use crate::Result;
use snaplingo_core::capture::CaptureCursorMover;
use snaplingo_core::capture::CaptureOutput;
use snaplingo_core::capture::CaptureSessionRuntime;
use snaplingo_core::capture::CaptureSessions;
use snaplingo_core::hotkeys::HotkeyRuntime;
use snaplingo_core::library::favorites::Favorites;
use snaplingo_core::library::history::History;
use snaplingo_core::library::history::OcrHistoryReplay;
use snaplingo_core::library::index::LibraryIndex;
use snaplingo_core::library::screenshot_favorites::ScreenshotFavoriteCapture;
use snaplingo_core::library::screenshot_favorites::ScreenshotFavorites;
use snaplingo_core::permissions::RequiredPermissions;
use snaplingo_core::pinned::PinnedImageRuntime;
use snaplingo_core::providers::ocr::OcrCoordinator;
use snaplingo_core::providers::translation::TranslationCoordinator;
use snaplingo_core::providers::ProviderAdministration;
use snaplingo_core::result::ResultWindowRuntime;
use snaplingo_core::settings::SettingsApplication;
use snaplingo_core::settings::SettingsConfiguration;
use snaplingo_core::speech::TtsRuntime;
use snaplingo_storage::SqliteAppLogRepository;

pub struct SettingsRuntime {
    pub configuration: Arc<SettingsConfiguration>,
    pub navigation: Arc<snaplingo_storage::SqliteConfigStore>,
    pub hotkeys: Arc<HotkeyRuntime>,
    pub application: Arc<SettingsApplication>,
}

pub struct ProviderRuntime {
    pub translation: Arc<TranslationCoordinator>,
    pub ocr: Arc<OcrCoordinator>,
    pub(crate) file_ocr: Arc<snaplingo_core::providers::ocr::FileOcr>,
    pub administration: Arc<ProviderAdministration>,
}

pub struct CaptureRuntimeState {
    pub sessions: Arc<CaptureSessions>,
    pub output: Arc<CaptureOutput>,
    pub runtime: Arc<CaptureSessionRuntime>,
    pub pinned_images: Arc<PinnedImageRuntime>,
    pub cursor: Arc<dyn CaptureCursorMover>,
}

pub struct HistoryRuntime {
    pub history: Arc<History>,
    pub ocr_replay: Arc<OcrHistoryReplay>,
    pub events: Arc<EventBus>,
}

pub struct ScreenshotFavoritesRuntime {
    pub favorites: Arc<ScreenshotFavorites>,
    pub capture: Arc<ScreenshotFavoriteCapture>,
}

pub struct FavoritesRuntime {
    pub favorites: Arc<Favorites>,
    pub ocr_application: Arc<snaplingo_core::library::favorites::OcrFavoriteApplication>,
}

pub struct SelectionRuntime {
    pub(crate) translation: Arc<snaplingo_core::selection::SelectionTranslation>,
}

pub struct LogsRuntime {
    pub repository: Arc<SqliteAppLogRepository>,
}

pub struct AppState {
    pub(crate) actions: crate::shell::actions::AppActionDispatcher,
    pub(crate) desktop_events: Arc<crate::integrations::desktop_events::DesktopEvents>,
    pub(crate) desktop_subscriptions: Vec<crate::integrations::desktop_events::DesktopSubscription>,
    pub updates: Arc<snaplingo_core::updates::AppUpdates>,
    pub permissions: Arc<RequiredPermissions>,
    pub settings: Arc<SettingsRuntime>,
    pub providers: Arc<ProviderRuntime>,
    pub capture: Arc<CaptureRuntimeState>,
    pub history: Arc<HistoryRuntime>,
    pub favorites: Arc<FavoritesRuntime>,
    pub screenshot_favorites: Arc<ScreenshotFavoritesRuntime>,
    pub library_index: Arc<LibraryIndex>,
    pub selection: Arc<SelectionRuntime>,
    pub logs: Arc<LogsRuntime>,
    pub tts: Arc<TtsRuntime>,
    pub(crate) result_window: Arc<ResultWindowRuntime>,
    #[cfg(any(
        target_os = "macos",
        all(target_os = "windows", feature = "native-app")
    ))]
    pub(crate) result_session: Arc<snaplingo_core::result::ResultWindowSession>,
    #[cfg(any(
        target_os = "macos",
        all(target_os = "windows", feature = "native-app")
    ))]
    pub(crate) native_result_window: Arc<crate::features::result::NativeResultWindowHost>,
    #[cfg(any(
        target_os = "macos",
        all(target_os = "windows", feature = "native-app")
    ))]
    pub(crate) native_pinned_images: Arc<crate::features::pinned::NativePinnedImageRuntimeHost>,
}

impl AppState {
    /// Gracefully shutdown the application, waiting for pending events to complete
    pub async fn shutdown(&self) -> Result<()> {
        log::info!("Starting graceful shutdown...");
        self.actions.shutdown();
        self.capture.runtime.cancel_background_ocr();
        #[cfg(any(
            target_os = "macos",
            all(target_os = "windows", feature = "native-app")
        ))]
        self.result_session.close();

        // Wait for all pending events to complete (max 5 seconds)
        let drained = self
            .history
            .events
            .drain(std::time::Duration::from_secs(5))
            .await;

        if !drained {
            log::warn!("Shutdown: Some events did not complete in time");
        }

        log::info!("Graceful shutdown complete");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use snaplingo_core::result::ResultWindowRuntime;

    use super::AppState;

    #[test]
    fn app_state_exposes_shared_result_window_runtime() {
        fn runtime(state: &AppState) -> &Arc<ResultWindowRuntime> {
            &state.result_window
        }

        let _ = runtime;
    }
}

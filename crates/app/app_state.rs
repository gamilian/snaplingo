use std::sync::Arc;

use crate::application::providers::ocr::OcrCoordinator;
use crate::application::providers::translation::TranslationCoordinator;
use crate::application::providers::ProviderAdministration;
use crate::application::result_window::ResultWindowRuntime;
use crate::application::{
    CaptureCursorMover, CaptureOutput, CaptureSessionRuntime, CaptureSessions, Favorites, History,
    HotkeyRuntime, LibraryIndex, OcrHistoryReplay, PinnedImageRuntime, RequiredPermissions,
    ScreenshotFavoriteCapture, ScreenshotFavorites, SettingsApplication, SettingsConfiguration,
    TtsRuntime,
};
use crate::infrastructure::events::EventBus;
use crate::infrastructure::storage::SqliteAppLogRepository;
use crate::Result;

pub struct SettingsRuntime {
    pub configuration: Arc<SettingsConfiguration>,
    pub navigation: Arc<crate::infrastructure::storage::SqliteConfigStore>,
    pub hotkeys: Arc<HotkeyRuntime>,
    pub application: Arc<SettingsApplication>,
}

pub struct ProviderRuntime {
    pub translation: Arc<TranslationCoordinator>,
    pub ocr: Arc<OcrCoordinator>,
    pub(crate) file_ocr: Arc<crate::application::providers::ocr::FileOcr>,
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
    pub ocr_application: Arc<crate::application::OcrFavoriteApplication>,
}

pub struct SelectionRuntime {
    pub(crate) translation: Arc<crate::application::selected_text::SelectionTranslation>,
}

pub struct LogsRuntime {
    pub repository: Arc<SqliteAppLogRepository>,
}

pub struct AppState {
    pub(crate) actions: crate::actions::AppActionDispatcher,
    pub(crate) desktop_events: Arc<crate::infrastructure::desktop_events::DesktopEvents>,
    pub(crate) desktop_subscriptions:
        Vec<crate::infrastructure::desktop_events::DesktopSubscription>,
    pub updates: Arc<crate::application::updates::AppUpdates>,
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
    pub(crate) result_session: Arc<crate::application::result_window::ResultWindowSession>,
    #[cfg(any(
        target_os = "macos",
        all(target_os = "windows", feature = "native-app")
    ))]
    pub(crate) native_result_window: Arc<crate::window::result::NativeResultWindowHost>,
    #[cfg(any(
        target_os = "macos",
        all(target_os = "windows", feature = "native-app")
    ))]
    pub(crate) native_pinned_images: Arc<crate::window::pinned::NativePinnedImageRuntimeHost>,
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

    use crate::application::result_window::ResultWindowRuntime;

    use super::AppState;

    #[test]
    fn app_state_exposes_shared_result_window_runtime() {
        fn runtime(state: &AppState) -> &Arc<ResultWindowRuntime> {
            &state.result_window
        }

        let _ = runtime;
    }
}

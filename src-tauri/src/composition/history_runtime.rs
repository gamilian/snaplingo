use std::sync::Arc;

use tauri::{AppHandle, Emitter};

use crate::application::history::{
    EventSubscriber, HistoryChangeNotifier, HistoryRepository, OcrHistoryRecognizer,
};
use crate::application::providers::ocr::OcrCoordinator;
use crate::application::settings::SettingsConfiguration;
use crate::application::History;
use crate::domain::ocr::OcrResult;
use crate::infrastructure::storage::{
    Database, FilesystemOcrHistoryAssets, JsonTranslationFavoritesWriter, SqliteHistoryRepository,
};
use crate::AppState;
use crate::Result;

struct TauriHistoryChangeNotifier {
    app: AppHandle,
}

pub(crate) struct OcrCoordinatorHistoryRecognizer {
    coordinator: Arc<OcrCoordinator>,
}

impl OcrCoordinatorHistoryRecognizer {
    pub(crate) fn new(coordinator: Arc<OcrCoordinator>) -> Self {
        Self { coordinator }
    }
}

#[async_trait::async_trait]
impl OcrHistoryRecognizer for OcrCoordinatorHistoryRecognizer {
    async fn recognize(&self, image: Vec<u8>) -> Result<OcrResult> {
        self.coordinator.recognize_image(image).await
    }
}

impl HistoryChangeNotifier for TauriHistoryChangeNotifier {
    fn history_changed(&self) {
        if let Err(error) = self.app.emit("history-changed", ()) {
            log::warn!("Failed to emit history-changed: {}", error);
        }
    }
}

pub(crate) fn build_history(
    database: Arc<Database>,
    asset_root: std::path::PathBuf,
    settings: Arc<SettingsConfiguration>,
    app: AppHandle,
) -> Arc<History> {
    let repository: Arc<dyn HistoryRepository> = Arc::new(SqliteHistoryRepository::new(database));
    Arc::new(History::with_dependencies_and_policy(
        repository,
        Arc::new(TauriHistoryChangeNotifier { app }),
        Arc::new(FilesystemOcrHistoryAssets::new(asset_root)),
        settings,
        Arc::new(JsonTranslationFavoritesWriter),
    ))
}

pub(crate) fn subscribe_history(app_state: &AppState) {
    let history_subscriber = app_state.history.history.clone() as Arc<dyn EventSubscriber>;
    let history = app_state.history.history.clone();
    let event_bus = app_state.history.events.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(error) = history.run_cleanup().await {
            log::warn!("Failed to run startup history cleanup: {}", error);
        }
        event_bus.subscribe(history_subscriber).await;
    });
}

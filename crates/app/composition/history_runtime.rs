use std::sync::Arc;

use crate::infrastructure::desktop_events::DesktopEvents;

use crate::infrastructure::storage::{
    Database, FilesystemOcrHistoryAssets, SqliteHistoryRepository,
};
use crate::AppState;
use crate::Result;
use snaplingo_core::application::history::{
    EventSubscriber, HistoryRepository, OcrHistoryRecognizer,
};
use snaplingo_core::application::providers::ocr::OcrCoordinator;
use snaplingo_core::application::settings::SettingsConfiguration;
use snaplingo_core::application::History;
use snaplingo_core::domain::ocr::OcrResult;

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

pub(crate) fn build_history(
    database: Arc<Database>,
    asset_root: std::path::PathBuf,
    settings: Arc<SettingsConfiguration>,
    events: Arc<DesktopEvents>,
) -> Arc<History> {
    let repository: Arc<dyn HistoryRepository> = Arc::new(SqliteHistoryRepository::new(database));
    Arc::new(History::with_dependencies_and_policy(
        repository,
        events,
        Arc::new(FilesystemOcrHistoryAssets::new(asset_root)),
        settings,
    ))
}

pub(crate) fn subscribe_history(app_state: &AppState) -> tokio::task::JoinHandle<()> {
    let history_subscriber = app_state.history.history.clone() as Arc<dyn EventSubscriber>;
    let history = app_state.history.history.clone();
    let event_bus = app_state.history.events.clone();
    crate::infrastructure::runtime::spawn(async move {
        if let Err(error) = history.run_cleanup().await {
            log::warn!("Failed to run startup history cleanup: {}", error);
        }
        event_bus.subscribe(history_subscriber).await;
    })
}

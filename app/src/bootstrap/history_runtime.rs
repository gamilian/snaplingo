use std::sync::Arc;

use crate::integrations::desktop_events::DesktopEvents;

use crate::AppState;
use crate::Result;
use snaplingo_core::library::history::EventSubscriber;
use snaplingo_core::library::history::History;
use snaplingo_core::library::history::HistoryRepository;
use snaplingo_core::library::history::OcrHistoryRecognizer;
use snaplingo_core::providers::ocr::model::OcrResult;
use snaplingo_core::providers::ocr::OcrCoordinator;
use snaplingo_core::settings::SettingsConfiguration;
use snaplingo_storage::Database;
use snaplingo_storage::FilesystemOcrHistoryAssets;
use snaplingo_storage::SqliteHistoryRepository;

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
    crate::integrations::runtime::spawn(async move {
        if let Err(error) = history.run_cleanup().await {
            log::warn!("Failed to run startup history cleanup: {}", error);
        }
        event_bus.subscribe(history_subscriber).await;
    })
}

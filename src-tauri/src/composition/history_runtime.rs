use std::sync::Arc;

use crate::application::history::{EventSubscriber, HistoryRepository};
use crate::application::History;
use crate::infrastructure::storage::HistoryDatabase;
use crate::infrastructure::system::paths::get_history_db_path;
use crate::AppState;

pub(crate) fn build_history() -> Arc<History> {
    let history_db_path = get_history_db_path().expect("Failed to get history database path");
    let history_db = Arc::new(
        HistoryDatabase::new(history_db_path).expect("Failed to initialize history database"),
    );
    let repository: Arc<dyn HistoryRepository> = history_db;
    Arc::new(History::new(repository))
}

pub(crate) fn subscribe_history(app_state: &AppState) {
    let history_subscriber = app_state.history.history.clone() as Arc<dyn EventSubscriber>;
    let event_bus = app_state.history.events.clone();
    tauri::async_runtime::spawn(async move {
        event_bus.subscribe(history_subscriber).await;
    });
}

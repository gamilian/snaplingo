use std::sync::Arc;

use crate::application::HistoryService;
use crate::infrastructure::events::EventSubscriber;
use crate::infrastructure::storage::HistoryDatabase;
use crate::infrastructure::system::paths::get_history_db_path;
use crate::AppState;

pub(crate) fn build_history_service() -> Arc<HistoryService> {
    let history_db_path = get_history_db_path().expect("Failed to get history database path");
    let history_db = Arc::new(
        HistoryDatabase::new(history_db_path).expect("Failed to initialize history database"),
    );
    Arc::new(HistoryService::new(history_db))
}

pub(crate) fn subscribe_history_service(app_state: &AppState) {
    let history_service_subscriber = app_state.history_service.clone() as Arc<dyn EventSubscriber>;
    let event_bus = app_state.event_bus.clone();
    tauri::async_runtime::spawn(async move {
        event_bus.subscribe(history_service_subscriber).await;
    });
}

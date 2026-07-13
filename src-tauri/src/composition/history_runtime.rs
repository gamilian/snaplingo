use std::sync::Arc;

use tauri::{AppHandle, Emitter};

use crate::application::history::{EventSubscriber, HistoryChangeNotifier, HistoryRepository};
use crate::application::History;
use crate::infrastructure::storage::{Database, SqliteHistoryRepository};
use crate::AppState;

struct TauriHistoryChangeNotifier {
    app: AppHandle,
}

impl HistoryChangeNotifier for TauriHistoryChangeNotifier {
    fn history_changed(&self) {
        if let Err(error) = self.app.emit("history-changed", ()) {
            log::warn!("Failed to emit history-changed: {}", error);
        }
    }
}

pub(crate) fn build_history(database: Arc<Database>, app: AppHandle) -> Arc<History> {
    let repository: Arc<dyn HistoryRepository> = Arc::new(SqliteHistoryRepository::new(database));
    Arc::new(History::with_change_notifier(
        repository,
        Arc::new(TauriHistoryChangeNotifier { app }),
    ))
}

pub(crate) fn subscribe_history(app_state: &AppState) {
    let history_subscriber = app_state.history.history.clone() as Arc<dyn EventSubscriber>;
    let event_bus = app_state.history.events.clone();
    tauri::async_runtime::spawn(async move {
        event_bus.subscribe(history_subscriber).await;
    });
}

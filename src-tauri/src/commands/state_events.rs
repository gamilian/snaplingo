use tauri::{AppHandle, Emitter};

pub const SETTINGS_CHANGED_EVENT: &str = "settings-changed";
pub const HOTKEYS_CHANGED_EVENT: &str = "hotkeys-changed";
pub const PROVIDERS_CHANGED_EVENT: &str = "providers-changed";
pub const HISTORY_CHANGED_EVENT: &str = "history-changed";

pub fn emit_state_changed(app: &AppHandle, event: &'static str) {
    if let Err(error) = app.emit(event, ()) {
        log::warn!("Failed to emit {}: {}", event, error);
    }
}

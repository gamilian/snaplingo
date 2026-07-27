use tauri::State;

use crate::domain::HotkeySettingsSnapshot;
use crate::HotkeyUpdateOutcome;

#[tauri::command]
pub fn get_hotkey_snapshot(
    state: State<'_, crate::AppState>,
) -> Result<HotkeySettingsSnapshot, String> {
    state
        .settings
        .hotkeys
        .snapshot()
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn get_default_hotkey_snapshot(state: State<'_, crate::AppState>) -> HotkeySettingsSnapshot {
    state.settings.hotkeys.default_snapshot()
}

#[tauri::command]
pub fn update_hotkey(
    category: String,
    action: String,
    hotkey: String,
    state: State<'_, crate::AppState>,
) -> Result<HotkeyUpdateOutcome, String> {
    state
        .settings
        .hotkeys
        .update_hotkey(category, action, hotkey)
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn reset_hotkey(
    category: String,
    action: String,
    state: State<'_, crate::AppState>,
) -> Result<HotkeyUpdateOutcome, String> {
    state
        .settings
        .hotkeys
        .reset_hotkey(category, action)
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn reset_hotkey_category(
    category: String,
    state: State<'_, crate::AppState>,
) -> Result<HotkeySettingsSnapshot, String> {
    state
        .settings
        .hotkeys
        .reset_category(category)
        .map_err(|err| err.to_string())
}

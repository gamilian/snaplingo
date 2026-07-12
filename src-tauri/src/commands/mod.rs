mod capture_session_commands;
mod clipboard_commands;
mod history_commands;
mod hotkey_commands;
mod ocr_commands;
mod pinned_image_commands;
mod provider_commands;
mod result_window_commands;
mod settings_commands;
mod translation_commands;

pub use capture_session_commands::*;
pub use clipboard_commands::*;
pub use history_commands::*;
pub use hotkey_commands::*;
pub use ocr_commands::*;
pub use pinned_image_commands::*;
pub use provider_commands::*;
pub use result_window_commands::*;
pub use settings_commands::*;
pub use translation_commands::*;

use tauri::State;

#[tauri::command]
pub fn configure_hotkey(
    category: String,
    action: String,
    hotkey: String,
    app: tauri::AppHandle,
    state: State<'_, crate::AppState>,
) -> Result<Option<String>, String> {
    let outcome = state
        .settings
        .hotkeys
        .update_hotkey_with(
            &crate::infrastructure::system::TauriHotkeyRegistrar::new(
                app,
                crate::startup_shortcuts::trigger_hotkey_action,
            ),
            category,
            action,
            hotkey,
        )
        .map_err(|e| e.to_string())?;
    Ok(outcome.accelerator)
}

#[tauri::command]
pub fn configure_translation_hotkey(
    action: String,
    hotkey: String,
    app: tauri::AppHandle,
    state: State<'_, crate::AppState>,
) -> Result<Option<String>, String> {
    let outcome = state
        .settings
        .hotkeys
        .update_hotkey_with(
            &crate::infrastructure::system::TauriHotkeyRegistrar::new(
                app,
                crate::startup_shortcuts::trigger_hotkey_action,
            ),
            "translation".to_string(),
            action,
            hotkey,
        )
        .map_err(|e| e.to_string())?;
    Ok(outcome.accelerator)
}

#[tauri::command]
pub async fn trigger_screenshot(
    app: tauri::AppHandle,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    let _ = app;
    open_capture_window_for_mode(state.inner(), "screenshot").await
}

mod capture_commands;
mod capture_session_commands;
mod history_commands;
mod ocr_commands;
mod pinned_image_commands;
mod provider_commands;
mod screenshot_window_commands;
mod translation_commands;
mod workflow_commands;

pub use capture_commands::*;
pub use capture_session_commands::*;
pub use history_commands::*;
pub use ocr_commands::*;
pub use pinned_image_commands::*;
pub use provider_commands::*;
pub use screenshot_window_commands::*;
pub use translation_commands::*;
pub use workflow_commands::*;

use serde::Serialize;
use tauri::{Emitter, Manager, State};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TranslationInputPayload {
    text: String,
    auto_translate: bool,
}

#[tauri::command]
pub fn open_result_window(text: String, app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;

        // Emit event to frontend with text
        window
            .emit("input-translation", text)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn emit_screenshot_error(app: tauri::AppHandle, message: String) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
        let _ = window.emit("screenshot-error", message);
    }
}

#[tauri::command]
pub fn open_translation_result_window(text: String, app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;

        window
            .emit(
                "input-translation",
                TranslationInputPayload {
                    text,
                    auto_translate: true,
                },
            )
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn show_translation_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        window
            .emit("show-translation-window", ())
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub async fn open_selection_translation_window(app: tauri::AppHandle) -> Result<(), String> {
    let text = tauri::async_runtime::spawn_blocking(copy_selected_text)
        .await
        .map_err(|e| e.to_string())??;

    open_translation_result_window(text, app)
}

fn copy_selected_text() -> Result<String, String> {
    use arboard::Clipboard;
    use enigo::{Enigo, Key, KeyboardControllable};
    use std::time::Duration;

    let mut enigo = Enigo::new();
    #[cfg(target_os = "macos")]
    let modifier = Key::Meta;
    #[cfg(not(target_os = "macos"))]
    let modifier = Key::Control;

    enigo.key_down(modifier);
    enigo.key_click(Key::Layout('c'));
    enigo.key_up(modifier);

    std::thread::sleep(Duration::from_millis(120));

    let mut clipboard =
        Clipboard::new().map_err(|e| format!("Failed to open clipboard: {}", e))?;
    let text = clipboard
        .get_text()
        .map_err(|e| format!("Failed to read selected text from clipboard: {}", e))?;
    if text.trim().is_empty() {
        return Err("Selected text is empty".to_string());
    }

    Ok(text)
}

#[tauri::command]
pub fn configure_hotkey(
    category: String,
    action: String,
    hotkey: String,
    app: tauri::AppHandle,
) -> Result<Option<String>, String> {
    crate::startup_shortcuts::configure_hotkey(&app, &category, &action, &hotkey)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn configure_translation_hotkey(
    action: String,
    hotkey: String,
    app: tauri::AppHandle,
) -> Result<Option<String>, String> {
    crate::startup_shortcuts::configure_translation_shortcut(&app, &action, &hotkey)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn trigger_screenshot(
    app: tauri::AppHandle,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    open_capture_window_for_mode(&app, &state, "screenshot").await
}

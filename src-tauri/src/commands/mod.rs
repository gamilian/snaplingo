mod capture_commands;
mod capture_session_commands;
mod history_commands;
mod ocr_commands;
mod pinned_image_commands;
mod provider_commands;
mod screenshot_window_commands;
mod translation_commands;

pub use capture_commands::*;
pub use capture_session_commands::*;
pub use history_commands::*;
pub use ocr_commands::*;
pub use pinned_image_commands::*;
pub use provider_commands::*;
pub use screenshot_window_commands::*;
pub use translation_commands::*;

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

#[tauri::command]
pub fn open_ocr_result_window(text: String, app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        window.emit("input-ocr", text).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn copy_text_to_clipboard(text: String) -> Result<(), String> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| format!("Failed to open clipboard: {}", e))?;
    clipboard
        .set_text(text)
        .map_err(|e| format!("Failed to write text to clipboard: {}", e))
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

pub fn show_ocr_window(app: tauri::AppHandle) -> Result<(), String> {
    emit_main_window_event(app, "show-ocr-window")
}

pub fn start_file_ocr(app: tauri::AppHandle) -> Result<(), String> {
    emit_main_window_event(app, "start-file-ocr")
}

fn emit_main_window_event(app: tauri::AppHandle, event: &str) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        window.emit(event, ()).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub async fn open_selection_translation_window_for_state(
    app: tauri::AppHandle,
    state: &crate::AppState,
) -> Result<(), String> {
    let snapshot = state
        .selected_text_acquirer
        .acquire()
        .await
        .map_err(|e| e.to_string())?;
    open_translation_result_window(snapshot.text, app)
}

#[tauri::command]
pub async fn open_selection_translation_window(
    app: tauri::AppHandle,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    open_selection_translation_window_for_state(app, state.inner()).await
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

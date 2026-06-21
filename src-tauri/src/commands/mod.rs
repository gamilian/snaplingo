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

#[tauri::command]
pub async fn trigger_screenshot(
    app: tauri::AppHandle,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    open_capture_window_for_mode(&app, &state, "screenshot").await
}

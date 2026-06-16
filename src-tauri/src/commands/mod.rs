mod translation_commands;
mod provider_commands;
mod ocr_commands;
mod capture_commands;
mod history_commands;
mod workflow_commands;
mod screenshot_window_commands;

pub use translation_commands::*;
pub use provider_commands::*;
pub use ocr_commands::*;
pub use capture_commands::*;
pub use history_commands::*;
pub use workflow_commands::*;
pub use screenshot_window_commands::*;

use tauri::{Emitter, Manager};

#[tauri::command]
pub fn open_result_window(
    text: String,
    app: tauri::AppHandle,
) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;

        // Emit event to frontend with text
        window.emit("input-translation", text)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn trigger_screenshot(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window
            .emit("hotkey-triggered", "screenshot")
            .map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Main window not found".to_string())
    }
}

mod translation_commands;
mod provider_commands;
mod ocr_commands;
mod capture_commands;

pub use translation_commands::*;
pub use provider_commands::*;
pub use ocr_commands::*;
pub use capture_commands::*;

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

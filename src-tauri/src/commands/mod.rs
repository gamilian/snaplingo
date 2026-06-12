mod translate;
mod config;

pub use translate::*;
pub use config::*;

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

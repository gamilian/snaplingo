mod capture_session_commands;
mod clipboard_commands;
mod history_commands;
mod hotkey_commands;
mod library_index_commands;
mod ocr_commands;
mod permission_commands;
mod pinned_image_commands;
mod provider_commands;
mod result_window_commands;
mod screenshot_favorite_commands;
mod settings_commands;
mod translation_commands;
mod tts_commands;

pub use capture_session_commands::*;
pub use clipboard_commands::*;
pub use history_commands::*;
pub use hotkey_commands::*;
pub use library_index_commands::*;
pub use ocr_commands::*;
pub use permission_commands::*;
pub use pinned_image_commands::*;
pub use provider_commands::*;
pub use result_window_commands::*;
pub use screenshot_favorite_commands::*;
pub use settings_commands::*;
pub use translation_commands::*;
pub use tts_commands::*;

use tauri::State;

#[tauri::command]
pub async fn trigger_screenshot(
    app: tauri::AppHandle,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    let _ = app;
    open_capture_window_for_mode(state.inner(), "screenshot").await
}

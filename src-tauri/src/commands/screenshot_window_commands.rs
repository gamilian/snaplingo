use tauri::{AppHandle, State};

use crate::AppState;

#[tauri::command]
pub fn create_screenshot_window(
    _app: AppHandle,
    _screenshot_base64: String,
    _scale_factor: f64,
) -> Result<(), String> {
    Err("Legacy screenshot window is replaced by capture sessions".to_string())
}

#[tauri::command]
pub fn create_screenshot_window_simple(_app: AppHandle) -> Result<(), String> {
    Err("Legacy screenshot window is replaced by capture sessions".to_string())
}

#[tauri::command]
pub fn close_screenshot_window(_app: AppHandle) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub fn crop_screenshot(
    _x: f64,
    _y: f64,
    _width: f64,
    _height: f64,
    _state: State<'_, AppState>,
) -> Result<String, String> {
    Err("Legacy screenshot crop is replaced by capture sessions".to_string())
}

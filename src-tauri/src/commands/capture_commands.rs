use crate::infrastructure::system::screenshot::ScreenRegion;
use base64::Engine;
use std::path::PathBuf;
use tauri::State;

#[tauri::command]
pub async fn capture_full_screen(state: State<'_, crate::AppState>) -> Result<String, String> {
    let png_data = state
        .capture_service
        .capture_full_screen()
        .await
        .map_err(|e| e.to_string())?;
    Ok(base64::engine::general_purpose::STANDARD.encode(&png_data))
}

#[tauri::command]
pub async fn capture_region(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    state: State<'_, crate::AppState>,
) -> Result<String, String> {
    let region = ScreenRegion {
        x,
        y,
        width,
        height,
    };
    let png_data = state
        .capture_service
        .capture_region(region)
        .await
        .map_err(|e| e.to_string())?;
    Ok(base64::engine::general_purpose::STANDARD.encode(&png_data))
}

#[tauri::command]
pub async fn save_screenshot(
    data: String,
    path: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    // Decode base64 to bytes
    let png_data = base64::engine::general_purpose::STANDARD
        .decode(data)
        .map_err(|e| format!("Failed to decode base64: {}", e))?;

    let path_buf = PathBuf::from(path);
    state
        .capture_service
        .save_screenshot(&png_data, &path_buf)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

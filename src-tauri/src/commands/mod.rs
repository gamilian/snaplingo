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
use crate::ScreenshotState;
use base64::Engine;
use parking_lot::Mutex as ParkingLotMutex;
use std::sync::Arc;

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

pub fn emit_screenshot_error(app: tauri::AppHandle, message: String) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
        let _ = window.emit("screenshot-error", message);
    }
}

pub async fn start_screenshot_overlay(
    app: tauri::AppHandle,
    capture_service: Arc<crate::CaptureService>,
    screenshot_state: Arc<ParkingLotMutex<ScreenshotState>>,
) -> Result<(), String> {
    let Some(main_window) = app.get_webview_window("main") else {
        log::error!("Main window not found");
        return Err("Main window not found".to_string());
    };

    let screenshot_bytes = capture_service.capture_full_screen().await
        .map_err(|e| e.to_string())?;
    let scale_factor = main_window.scale_factor().map_err(|e| e.to_string())?;

    if let Ok(img) = image::load_from_memory(&screenshot_bytes) {
        let mut state = screenshot_state.lock();
        state.data = Some(screenshot_bytes.clone());
        state.width = img.width();
        state.height = img.height();
        state.scale_factor = scale_factor;
    } else {
        let mut state = screenshot_state.lock();
        *state = ScreenshotState {
            data: Some(screenshot_bytes.clone()),
            width: 0,
            height: 0,
            scale_factor,
        };
    }

    let screenshot_base64 = base64::engine::general_purpose::STANDARD.encode(&screenshot_bytes);
    crate::commands::create_screenshot_window(app, screenshot_base64, scale_factor)?;

    Ok(())
}

#[tauri::command]
pub async fn trigger_screenshot(
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::AppState>,
) -> Result<(), String> {
    log::info!("Manual screenshot trigger from frontend");
    start_screenshot_overlay(
        app,
        state.capture_service.clone(),
        state.screenshot_state.clone(),
    ).await
}

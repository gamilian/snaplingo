use crate::{settings_window, AppState};
use base64::Engine;
use std::io::Cursor;
use std::process::Command;
use tauri::{Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};

/// Simple approach: Use macOS native screencapture tool
/// This is how many screenshot apps work on macOS - they wrap the native tool
#[tauri::command]
pub fn create_screenshot_window_simple(_app: tauri::AppHandle) -> Result<(), String> {
    log::info!("Starting screenshot using macOS native screencapture");

    // Use macOS native screencapture command in interactive mode
    // -i = interactive selection (user drags to select region)
    // -c = copy result to clipboard
    // This gives the user the familiar macOS screenshot crosshair

    let result = Command::new("screencapture").arg("-i").arg("-c").spawn();

    match result {
        Ok(mut child) => {
            log::info!("screencapture launched successfully");

            // Spawn a thread to wait for completion
            std::thread::spawn(move || {
                if let Ok(status) = child.wait() {
                    if status.success() {
                        log::info!("Screenshot captured to clipboard");
                    } else {
                        log::warn!("screencapture exited with status: {}", status);
                    }
                }
            });

            Ok(())
        }
        Err(e) => {
            log::error!("Failed to launch screencapture: {}", e);
            Err(format!("Failed to start screenshot: {}", e))
        }
    }
}

#[tauri::command]
#[allow(dead_code)]
pub fn create_screenshot_window_simple_custom(app: tauri::AppHandle) -> Result<(), String> {
    log::info!("Creating screenshot window (simple mode - plugin handles capture)");

    // CRITICAL: Hide ALL existing windows first
    // This allows the screenshot window to be truly independent at OS level
    for (label, window) in app.webview_windows().iter() {
        if label != "screenshot" {
            log::info!("Hiding window: {}", label);
            if let Err(e) = window.hide() {
                log::warn!("Failed to hide window {}: {}", label, e);
            }
        }
    }

    // Close existing screenshot window if any
    if let Some(existing) = app.get_webview_window("screenshot") {
        let _ = existing.close();
    }

    // Get primary monitor position and size
    let monitor = app
        .primary_monitor()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "No monitor found".to_string())?;

    let position = monitor.position();
    let size = monitor.size();
    let scale = monitor.scale_factor();

    log::info!(
        "Monitor: pos=({}, {}), size={}x{}, scale={}",
        position.x,
        position.y,
        size.width,
        size.height,
        scale
    );

    // Create an independent overlay window that covers the entire screen
    let window = WebviewWindowBuilder::new(
        &app,
        "screenshot",
        WebviewUrl::App("screenshot.html".into()),
    )
    .title("Screenshot")
    .position(position.x as f64, position.y as f64)
    .inner_size(size.width as f64, size.height as f64)
    .decorations(false)
    .resizable(false)
    .closable(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .focused(legacy_screenshot_window_should_start_focused())
    .accept_first_mouse(true)
    .visible(false)
    .build()
    .map_err(|e| format!("Failed to create screenshot window: {}", e))?;

    // Store app handle for restoring windows later
    let app_clone = app.clone();

    // Show window and set up close handler
    tauri::async_runtime::spawn(async move {
        // Brief delay to ensure window is ready
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        if let Err(e) = window.show() {
            log::error!("Failed to show screenshot window: {}", e);
        }
        if should_focus_legacy_screenshot_window_after_show() {
            if let Err(e) = window.set_focus() {
                log::error!("Failed to focus screenshot window: {}", e);
            }
        }

        // Listen for window close to restore other windows
        window.on_window_event(move |event| {
            if let tauri::WindowEvent::Destroyed = event {
                let visible_window_labels = app_clone
                    .webview_windows()
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>();
                for label in legacy_screenshot_window_labels_to_restore(&visible_window_labels) {
                    if let Some(win) = app_clone.get_webview_window(&label) {
                        log::info!("Showing window: {}", label);
                        let _ = win.show();
                    }
                }
            }
        });
    });

    Ok(())
}

#[tauri::command]
pub fn create_screenshot_window(
    app: tauri::AppHandle,
    _screenshot_base64: String,
    _scale_factor: f64,
) -> Result<(), String> {
    log::info!("Creating screenshot window");

    // Get primary monitor position and size
    let (x, y, width, height) =
        if let Some(monitor) = app.primary_monitor().map_err(|e| e.to_string())? {
            let position = monitor.position();
            let size = monitor.size();
            let scale_factor = monitor.scale_factor();
            let logical_position = position.to_logical::<f64>(scale_factor);
            let logical_size = size.to_logical::<f64>(scale_factor);
            (
                logical_position.x,
                logical_position.y,
                logical_size.width,
                logical_size.height,
            )
        } else {
            return Err("No monitor found".to_string());
        };

    // Create a fullscreen overlay window (not fullscreen mode, but screen-sized)
    WebviewWindowBuilder::new(
        &app,
        "screenshot",
        WebviewUrl::App("screenshot.html".into()),
    )
    .title("Screenshot")
    .inner_size(width, height) // ← Use screen size, not fullscreen mode
    .position(x, y)
    .decorations(false)
    .resizable(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .focused(legacy_screenshot_window_should_start_focused())
    .accept_first_mouse(true)
    .build()
    .map_err(|e| format!("Failed to create screenshot window: {}", e))?;

    log::info!("Screenshot window created; waiting for overlay page readiness");

    Ok(())
}

#[tauri::command]
pub fn screenshot_overlay_ready(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    log::info!("Screenshot overlay page is ready");

    let window = app
        .get_webview_window("screenshot")
        .ok_or_else(|| "Screenshot window not found".to_string())?;

    #[derive(serde::Serialize, Clone)]
    struct ScreenshotData {
        base64: String,
        scale_factor: f64,
    }

    let data = {
        let screenshot_state = state.screenshot_state.lock();
        let screenshot_bytes = screenshot_state
            .data
            .as_ref()
            .ok_or_else(|| "No screenshot data available".to_string())?;

        ScreenshotData {
            base64: base64::engine::general_purpose::STANDARD.encode(screenshot_bytes),
            scale_factor: screenshot_state.scale_factor,
        }
    };

    if should_focus_legacy_screenshot_window_when_overlay_ready() {
        window
            .set_focus()
            .map_err(|e| format!("Failed to focus screenshot window: {}", e))?;
    }
    window
        .emit("screenshot-data", data)
        .map_err(|e| format!("Failed to emit screenshot data: {}", e))?;

    Ok(())
}

#[tauri::command]
pub fn close_screenshot_window(app: tauri::AppHandle) -> Result<(), String> {
    log::info!("Closing screenshot window");

    if let Some(window) = app.get_webview_window("screenshot") {
        window
            .close()
            .map_err(|e| format!("Failed to close screenshot window: {}", e))?;
    }

    let visible_window_labels = app.webview_windows().keys().cloned().collect::<Vec<_>>();
    for label in legacy_screenshot_window_labels_to_restore(&visible_window_labels) {
        if let Some(window) = app.get_webview_window(&label) {
            log::info!("Restoring window: {}", label);
            if let Err(e) = window.show() {
                log::warn!("Failed to show window {}: {}", label, e);
            }
        }
    }

    Ok(())
}

#[tauri::command]
pub fn crop_screenshot(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    log::info!(
        "Cropping screenshot: x={}, y={}, width={}, height={}",
        x,
        y,
        width,
        height
    );

    let screenshot_state = state.screenshot_state.lock();

    // Check if we have screenshot data
    let data = screenshot_state
        .data
        .as_ref()
        .ok_or_else(|| "No screenshot data available".to_string())?;

    let scale_factor = screenshot_state.scale_factor;

    // Convert logical pixels to physical pixels
    let px = (x * scale_factor) as u32;
    let py = (y * scale_factor) as u32;
    let pw = (width * scale_factor) as u32;
    let ph = (height * scale_factor) as u32;

    log::info!(
        "Physical coordinates: x={}, y={}, width={}, height={} (scale={})",
        px,
        py,
        pw,
        ph,
        scale_factor
    );

    // Load and crop image
    let img = image::load_from_memory(data).map_err(|e| format!("Failed to load image: {}", e))?;

    let cropped = img.crop_imm(px, py, pw, ph);

    // Encode to PNG
    let mut buf = Vec::new();
    cropped
        .write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)
        .map_err(|e| format!("Failed to encode PNG: {}", e))?;

    // Convert to base64
    use base64::{engine::general_purpose, Engine as _};
    let base64 = general_purpose::STANDARD.encode(&buf);

    if should_focus_main_after_legacy_screenshot_crop() {
        if let Some(settings_window) =
            app.get_webview_window(settings_window::SETTINGS_WINDOW_LABEL)
        {
            settings_window
                .show()
                .map_err(|e| format!("Failed to show settings window: {}", e))?;
            settings_window
                .set_focus()
                .map_err(|e| format!("Failed to focus settings window: {}", e))?;
            settings_window
                .emit("screenshot-captured", base64.clone())
                .map_err(|e| format!("Failed to emit screenshot-captured: {}", e))?;
        }
    } else if let Some(settings_window) =
        app.get_webview_window(settings_window::SETTINGS_WINDOW_LABEL)
    {
        settings_window
            .emit("screenshot-captured", base64.clone())
            .map_err(|e| format!("Failed to emit screenshot-captured: {}", e))?;
    }

    Ok(base64)
}

fn legacy_screenshot_window_labels_to_restore(_visible_window_labels: &[String]) -> Vec<String> {
    Vec::new()
}

fn should_focus_main_after_legacy_screenshot_crop() -> bool {
    false
}

fn legacy_screenshot_window_should_start_focused() -> bool {
    false
}

#[allow(dead_code)]
fn should_focus_legacy_screenshot_window_after_show() -> bool {
    false
}

fn should_focus_legacy_screenshot_window_when_overlay_ready() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_screenshot_close_does_not_restore_main_window() {
        let labels = vec![
            "settings".to_string(),
            "capture".to_string(),
            "screenshot".to_string(),
        ];

        assert!(legacy_screenshot_window_labels_to_restore(&labels).is_empty());
    }

    #[test]
    fn legacy_screenshot_crop_does_not_focus_main_window() {
        assert!(!should_focus_main_after_legacy_screenshot_crop());
    }

    #[test]
    fn legacy_screenshot_window_does_not_start_focused() {
        assert!(!legacy_screenshot_window_should_start_focused());
    }

    #[test]
    fn legacy_screenshot_window_does_not_focus_after_show() {
        assert!(!should_focus_legacy_screenshot_window_after_show());
    }

    #[test]
    fn legacy_screenshot_overlay_ready_does_not_focus_window() {
        assert!(!should_focus_legacy_screenshot_window_when_overlay_ready());
    }
}

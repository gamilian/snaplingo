use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, WebviewUrl, WebviewWindowBuilder,
};

use crate::domain::capture::LogicalRect;

use super::backend::{
    capture_snapshot_window_labels_to_hide, capture_snapshot_window_labels_to_restore,
    capture_window_url_with_session, normalized_capture_mode, CAPTURE_WINDOW_LABEL,
};

pub fn hide_capture_snapshot_windows(app: &AppHandle) -> Result<Vec<String>, String> {
    let visible_window_labels = app
        .webview_windows()
        .into_iter()
        .filter_map(|(label, window)| match window.is_visible() {
            Ok(true) => Some(label),
            _ => None,
        })
        .collect::<Vec<_>>();
    let labels_to_hide = capture_snapshot_window_labels_to_hide(&visible_window_labels);

    for label in &labels_to_hide {
        if let Some(window) = app.get_webview_window(label) {
            window.hide().map_err(|e| e.to_string())?;
        }
    }

    Ok(labels_to_hide)
}

pub fn restore_capture_snapshot_windows(
    app: &AppHandle,
    hidden_window_labels: &[String],
) -> Result<(), String> {
    for label in capture_snapshot_window_labels_to_restore(hidden_window_labels) {
        if let Some(window) = app.get_webview_window(&label) {
            window.show().map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

pub fn open_capture_window_for_session(
    app: &AppHandle,
    mode: &str,
    session_id: &str,
    bounds: &LogicalRect,
) -> Result<(), String> {
    let mode = normalized_capture_mode(mode);

    if let Some(window) = app.get_webview_window(CAPTURE_WINDOW_LABEL) {
        window.set_fullscreen(false).map_err(|e| e.to_string())?;
        window
            .set_position(LogicalPosition::new(bounds.x, bounds.y))
            .map_err(|e| e.to_string())?;
        window
            .set_size(LogicalSize::new(bounds.width, bounds.height))
            .map_err(|e| e.to_string())?;
        configure_capture_window_for_current_space(&window)?;
        window
            .emit(
                "hotkey-triggered",
                serde_json::json!({
                    "mode": mode,
                    "sessionId": session_id,
                }),
            )
            .map_err(|e| e.to_string())?;
        return Ok(());
    }

    let window = WebviewWindowBuilder::new(
        app,
        CAPTURE_WINDOW_LABEL,
        WebviewUrl::App(capture_window_url_with_session(mode, session_id)),
    )
    .title("SnapLingo Capture")
    .position(bounds.x, bounds.y)
    .inner_size(bounds.width, bounds.height)
    .decorations(false)
    .always_on_top(true)
    .visible_on_all_workspaces(true)
    .transparent(true)
    .visible(false)
    .skip_taskbar(true)
    .focused(true)
    .shadow(false)
    .build()
    .map_err(|e| e.to_string())?;
    configure_capture_window_for_current_space(&window)?;

    Ok(())
}

fn configure_capture_window_for_current_space(window: &tauri::WebviewWindow) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        super::macos::configure_capture_window_for_current_space(window)?;
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = window;
    }

    Ok(())
}

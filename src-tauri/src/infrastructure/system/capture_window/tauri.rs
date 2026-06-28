use std::{
    thread,
    time::{Duration, Instant},
};

use tauri::{
    utils::config::Color, AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, WebviewUrl,
    WebviewWindowBuilder,
};

use crate::domain::capture::LogicalRect;

use super::backend::{
    capture_snapshot_window_labels_to_hide, capture_snapshot_window_labels_to_restore,
    capture_window_prewarm_url, capture_window_url_with_session, normalized_capture_mode,
    CAPTURE_WINDOW_LABEL,
};

pub fn begin_capture_presentation(app: &AppHandle) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        super::macos::begin_capture_presentation(app)?;
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
    }

    Ok(())
}

pub fn end_capture_presentation(app: &AppHandle) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        super::macos::end_capture_presentation(app)?;
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
    }

    Ok(())
}

pub fn is_capture_presentation_active() -> bool {
    #[cfg(target_os = "macos")]
    {
        return super::macos::is_capture_presentation_active();
    }

    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

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

pub fn reveal_capture_window(app: &AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window(CAPTURE_WINDOW_LABEL)
        .ok_or_else(|| "Capture window is not open".to_string())?;

    #[cfg(target_os = "macos")]
    {
        suppress_capture_window_activation(app)?;
        reveal_capture_window_for_current_space(&window)?;
        restore_suppressed_capture_window_activation();
    }

    #[cfg(not(target_os = "macos"))]
    {
        configure_capture_window_for_current_space(&window)?;
        window.show().map_err(|e| e.to_string())?;
        focus_capture_window_for_current_space(&window)?;
        reveal_capture_window_for_current_space(&window)?;
    }

    Ok(())
}

pub fn prepare_capture_window_for_reveal(app: &AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window(CAPTURE_WINDOW_LABEL)
        .ok_or_else(|| "Capture window is not open".to_string())?;

    #[cfg(target_os = "macos")]
    {
        suppress_capture_window_activation(app)?;
        super::macos::prepare_capture_window_for_reveal(&window)?;
        restore_suppressed_capture_window_activation();
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = window;
    }

    Ok(())
}

pub fn hide_capture_window(app: &AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window(CAPTURE_WINDOW_LABEL)
        .ok_or_else(|| "Capture window is not open".to_string())?;

    #[cfg(target_os = "macos")]
    {
        super::macos::hide_capture_window_for_current_space(&window)?;
    }

    #[cfg(not(target_os = "macos"))]
    {
        window.hide().map_err(|e| e.to_string())?;
    }

    Ok(())
}

pub fn destroy_inactive_capture_window(app: &AppHandle) -> Result<(), String> {
    if !should_destroy_capture_window_when_inactive() {
        return Ok(());
    }

    let Some(window) = app.get_webview_window(CAPTURE_WINDOW_LABEL) else {
        return Ok(());
    };

    window.destroy().map_err(|e| e.to_string())
}

pub fn prewarm_capture_window(app: &AppHandle) -> Result<(), String> {
    if !should_prewarm_capture_window() {
        return Ok(());
    }

    if app.get_webview_window(CAPTURE_WINDOW_LABEL).is_some() {
        return Ok(());
    }

    let window = WebviewWindowBuilder::new(
        app,
        CAPTURE_WINDOW_LABEL,
        WebviewUrl::App(capture_window_prewarm_url()),
    )
    .title("SnapLingo Capture")
    .position(0.0, 0.0)
    .inner_size(1.0, 1.0)
    .decorations(false)
    .always_on_top(true)
    .visible_on_all_workspaces(capture_window_visible_on_all_workspaces())
    .transparent(capture_window_is_transparent())
    .background_color(capture_window_background_color())
    .visible(false)
    .skip_taskbar(true)
    .focused(false)
    .accept_first_mouse(capture_window_accepts_first_mouse())
    .shadow(false)
    .build()
    .map_err(|e| e.to_string())?;
    configure_capture_window_for_current_space(&window)?;
    restore_suppressed_capture_window_activation();

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
        if !should_reuse_capture_window_for_session() {
            discard_capture_window_before_new_session(app, &window)?;
        } else {
            if should_reset_capture_window_fullscreen_before_reuse() {
                window.set_fullscreen(false).map_err(|e| e.to_string())?;
            }
            if capture_window_visible_on_all_workspaces() {
                window
                    .set_visible_on_all_workspaces(true)
                    .map_err(|e| e.to_string())?;
            }
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
    }

    suppress_capture_window_activation(app)?;
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
    .visible_on_all_workspaces(capture_window_visible_on_all_workspaces())
    .transparent(capture_window_is_transparent())
    .background_color(capture_window_background_color())
    .visible(false)
    .skip_taskbar(true)
    .focused(false)
    .accept_first_mouse(capture_window_accepts_first_mouse())
    .shadow(false)
    .build()
    .map_err(|e| e.to_string())?;
    configure_capture_window_for_current_space(&window)?;
    restore_suppressed_capture_window_activation();

    Ok(())
}

fn suppress_capture_window_activation(app: &AppHandle) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        super::macos::suppress_capture_window_activation(app)?;
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
    }

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

fn restore_suppressed_capture_window_activation() {
    #[cfg(target_os = "macos")]
    {
        super::macos::restore_suppressed_capture_window_activation();
    }
}

#[cfg(not(target_os = "macos"))]
fn focus_capture_window_for_current_space(window: &tauri::WebviewWindow) -> Result<(), String> {
    window.set_focus().map_err(|e| e.to_string())?;

    Ok(())
}

fn capture_window_accepts_first_mouse() -> bool {
    true
}

fn capture_window_is_transparent() -> bool {
    true
}

fn capture_window_visible_on_all_workspaces() -> bool {
    true
}

fn capture_window_background_color() -> Color {
    Color(0, 0, 0, 0)
}

fn should_reset_capture_window_fullscreen_before_reuse() -> bool {
    !cfg!(target_os = "macos")
}

fn should_prewarm_capture_window() -> bool {
    !cfg!(target_os = "macos")
}

fn should_reuse_capture_window_for_session() -> bool {
    !cfg!(target_os = "macos")
}

fn should_destroy_capture_window_when_inactive() -> bool {
    !should_reuse_capture_window_for_session()
}

fn discard_capture_window_before_new_session(
    app: &AppHandle,
    window: &tauri::WebviewWindow,
) -> Result<(), String> {
    window.destroy().map_err(|e| e.to_string())?;

    let start = Instant::now();
    while start.elapsed() < capture_window_destroy_timeout() {
        if app.get_webview_window(CAPTURE_WINDOW_LABEL).is_none() {
            return Ok(());
        }

        thread::sleep(capture_window_destroy_poll_interval());
    }

    if app.get_webview_window(CAPTURE_WINDOW_LABEL).is_none() {
        return Ok(());
    }

    Err("Timed out waiting for previous capture window to close".to_string())
}

fn capture_window_destroy_timeout() -> Duration {
    Duration::from_millis(2000)
}

fn capture_window_destroy_poll_interval() -> Duration {
    Duration::from_millis(10)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_window_accepts_first_mouse_on_reveal() {
        assert!(capture_window_accepts_first_mouse());
    }

    #[test]
    fn capture_window_is_transparent_for_lazy_canvas_overlay() {
        assert!(capture_window_is_transparent());
        assert_eq!(capture_window_background_color(), Color(0, 0, 0, 0));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_capture_window_joins_all_spaces_for_fullscreen_apps() {
        assert!(capture_window_visible_on_all_workspaces());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_capture_window_is_not_prewarmed() {
        assert!(!should_prewarm_capture_window());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_capture_window_is_recreated_between_sessions_to_retarget_fullscreen_space() {
        assert!(!should_reuse_capture_window_for_session());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_capture_window_is_destroyed_when_inactive() {
        assert!(should_destroy_capture_window_when_inactive());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_waits_for_slow_capture_window_label_release() {
        assert!(capture_window_destroy_timeout() >= Duration::from_millis(1500));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_capture_window_reuse_does_not_force_fullscreen_transition() {
        assert!(!should_reset_capture_window_fullscreen_before_reuse());
    }

    #[test]
    fn waits_briefly_for_destroyed_capture_window_label_to_be_released() {
        assert!(capture_window_destroy_timeout() >= capture_window_destroy_poll_interval());
    }
}

fn reveal_capture_window_for_current_space(window: &tauri::WebviewWindow) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        super::macos::reveal_capture_window_for_current_space(window)?;
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = window;
    }

    Ok(())
}

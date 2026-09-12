use std::{
    thread,
    time::{Duration, Instant},
};

use tauri::{utils::config::Color, AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
use tauri::{LogicalPosition, LogicalSize};
#[cfg(target_os = "windows")]
use tauri::{PhysicalPosition, PhysicalSize};

use crate::application::capture::CaptureWindowGeometry;
#[cfg(target_os = "windows")]
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

pub fn reveal_capture_window(
    app: &AppHandle,
    _geometry: Option<&CaptureWindowGeometry>,
) -> Result<(), String> {
    let window = app
        .get_webview_window(CAPTURE_WINDOW_LABEL)
        .ok_or_else(|| "Capture window is not open".to_string())?;

    #[cfg(target_os = "macos")]
    {
        reveal_capture_window_for_current_space(&window)?;
    }

    #[cfg(not(target_os = "macos"))]
    {
        configure_capture_window_for_current_space(
            &window,
            _geometry.and_then(|geometry| geometry.desktop_scale),
        )?;
        window.show().map_err(|e| e.to_string())?;
        focus_capture_window_for_current_space(&window)?;
        reveal_capture_window_for_current_space(&window)?;
    }

    Ok(())
}

pub fn prepare_capture_window_for_reveal(
    app: &AppHandle,
    geometry: Option<&CaptureWindowGeometry>,
) -> Result<(), String> {
    let window = app
        .get_webview_window(CAPTURE_WINDOW_LABEL)
        .ok_or_else(|| "Capture window is not open".to_string())?;
    if let Some(geometry) = geometry {
        set_capture_window_frame(&window, geometry)?;
    }

    #[cfg(target_os = "macos")]
    {
        suppress_capture_window_activation(app)?;
        super::macos::prepare_capture_window_for_reveal(&window)?;
        restore_capture_window_activation();
    }

    #[cfg(target_os = "windows")]
    configure_capture_window_for_current_space(
        &window,
        geometry.and_then(|geometry| geometry.desktop_scale),
    )?;

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = window;
    }

    Ok(())
}

pub fn hide_capture_window(app: &AppHandle) -> Result<(), String> {
    let Some(window) = app.get_webview_window(CAPTURE_WINDOW_LABEL) else {
        return Ok(());
    };

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

pub fn set_capture_window_cursor_passthrough(app: &AppHandle, enabled: bool) -> Result<(), String> {
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app, enabled);
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        let window = app
            .get_webview_window(CAPTURE_WINDOW_LABEL)
            .ok_or_else(|| "Capture window is not open".to_string())?;

        window
            .set_ignore_cursor_events(enabled)
            .map_err(|error| error.to_string())
    }
}

pub fn destroy_inactive_capture_window(app: &AppHandle) -> Result<(), String> {
    if !should_destroy_capture_window_when_inactive(is_capture_presentation_active()) {
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
    .focusable(capture_window_is_focusable())
    .accept_first_mouse(capture_window_accepts_first_mouse())
    .shadow(false)
    .build()
    .map_err(|e| e.to_string())?;
    configure_capture_window_for_current_space(&window, None)?;
    restore_capture_window_activation();

    Ok(())
}

pub fn open_capture_window_for_session(
    app: &AppHandle,
    mode: &str,
    session_id: &str,
    geometry: &CaptureWindowGeometry,
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
                .set_focusable(capture_window_is_focusable())
                .map_err(|e| e.to_string())?;
            set_capture_window_frame(&window, geometry)?;
            configure_capture_window_for_current_space(&window, geometry.desktop_scale)?;
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
    #[cfg(target_os = "windows")]
    let initial_bounds = LogicalRect {
        x: 0.0,
        y: 0.0,
        width: 1.0,
        height: 1.0,
    };
    // Tauri queues the builder position on macOS. It must agree with the
    // native frame so that a late queued move cannot restore the main origin.
    #[cfg(not(target_os = "windows"))]
    let initial_bounds = geometry.bounds.clone();
    let window = WebviewWindowBuilder::new(
        app,
        CAPTURE_WINDOW_LABEL,
        WebviewUrl::App(capture_window_url_with_session(mode, session_id)),
    )
    .title("SnapLingo Capture")
    .position(initial_bounds.x, initial_bounds.y)
    .inner_size(initial_bounds.width, initial_bounds.height)
    .decorations(false)
    .always_on_top(true)
    .visible_on_all_workspaces(capture_window_visible_on_all_workspaces())
    .transparent(capture_window_is_transparent())
    .background_color(capture_window_background_color())
    .visible(false)
    .skip_taskbar(true)
    .focused(false)
    .focusable(capture_window_is_focusable())
    .accept_first_mouse(capture_window_accepts_first_mouse())
    .shadow(false)
    .build()
    .map_err(|e| e.to_string())?;
    bind_capture_window_session(app, session_id);
    set_capture_window_frame(&window, geometry)?;
    configure_capture_window_for_current_space(&window, geometry.desktop_scale)?;
    restore_capture_window_activation();

    Ok(())
}

pub(super) fn bind_capture_window_session(app: &AppHandle, session_id: &str) {
    let Some(window) = app.get_webview_window(CAPTURE_WINDOW_LABEL) else {
        return;
    };
    let Some(state) = app.try_state::<crate::AppState>() else {
        return;
    };
    let runtime = std::sync::Arc::downgrade(&state.capture.runtime);
    let session_id = crate::domain::capture::CaptureSessionId(session_id.to_owned());
    // Bind the ID to this window instance: looking up the label at destruction
    // time could accidentally cancel a newly opened capture window.
    window.on_window_event(move |event| {
        if !matches!(event, tauri::WindowEvent::Destroyed) {
            return;
        }
        if let Some(runtime) = runtime.upgrade() {
            let session_id = session_id.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = runtime.cancel_capture_session(&session_id).await {
                    log::warn!("Failed to clean up destroyed capture session: {}", error);
                }
            });
        }
    });
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

fn configure_capture_window_for_current_space(
    window: &tauri::WebviewWindow,
    _desktop_scale: Option<f64>,
) -> Result<(), String> {
    window
        .set_focusable(capture_window_is_focusable())
        .map_err(|e| e.to_string())?;

    #[cfg(target_os = "macos")]
    {
        super::macos::configure_capture_window_for_current_space(window)?;
    }

    #[cfg(target_os = "windows")]
    {
        // WebView2 follows the window's monitor DPI. Keep CSS coordinates in
        // the same primary-display scale as the frozen capture session.
        if let Some(desktop_scale) = _desktop_scale {
            let window_scale = window.scale_factor().map_err(|e| e.to_string())?;
            window
                .set_zoom(desktop_scale / window_scale)
                .map_err(|e| e.to_string())?;
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = window;
    }

    Ok(())
}

fn set_capture_window_frame(
    window: &tauri::WebviewWindow,
    geometry: &CaptureWindowGeometry,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    super::macos::set_capture_window_frame(window, &geometry.bounds)?;
    #[cfg(target_os = "windows")]
    set_windows_capture_window_frame(window, geometry)?;
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let bounds = &geometry.bounds;
        window
            .set_position(LogicalPosition::new(bounds.x, bounds.y))
            .map_err(|e| e.to_string())?;
        window
            .set_size(LogicalSize::new(bounds.width, bounds.height))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn set_windows_capture_window_frame(
    window: &tauri::WebviewWindow,
    geometry: &CaptureWindowGeometry,
) -> Result<(), String> {
    let bounds = &geometry.bounds;
    let scale = geometry
        .desktop_scale
        .ok_or_else(|| "Windows capture session has no desktop scale".to_string())?;
    window
        .set_position(PhysicalPosition::new(
            (bounds.x * scale).round() as i32,
            (bounds.y * scale).round() as i32,
        ))
        .map_err(|e| e.to_string())?;
    window
        .set_size(PhysicalSize::new(
            (bounds.width * scale).round() as u32,
            (bounds.height * scale).round() as u32,
        ))
        .map_err(|e| e.to_string())
}

fn restore_capture_window_activation() {
    #[cfg(target_os = "macos")]
    {
        super::macos::restore_capture_window_activation();
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

fn capture_window_is_focusable() -> bool {
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

fn should_destroy_capture_window_when_inactive(has_active_presentation: bool) -> bool {
    !has_active_presentation && !should_reuse_capture_window_for_session()
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
    fn capture_window_is_focusable_on_reveal() {
        assert!(capture_window_is_focusable());
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
        assert!(should_destroy_capture_window_when_inactive(false));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_keeps_the_capture_window_when_another_session_is_still_active() {
        assert!(!should_destroy_capture_window_when_inactive(true));
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

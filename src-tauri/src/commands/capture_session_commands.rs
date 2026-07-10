use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use tauri::{AppHandle, Manager, State};

use crate::application::services::CaptureSessionOutput;
use crate::domain::capture::{
    AnnotationCommand, CaptureOutputAction, CaptureSessionId, CaptureSessionView, LogicalPoint,
    LogicalRect,
};
use crate::domain::ocr::OcrResult;
use crate::infrastructure::system::capture_window::{
    hide_capture_window as hide_capture_window_for_app,
    prepare_capture_window_for_reveal as prepare_capture_window_for_reveal_for_app,
    reveal_capture_window as reveal_capture_window_for_app,
};

static CAPTURE_SHORTCUT_OPEN_IN_FLIGHT: AtomicBool = AtomicBool::new(false);

struct CaptureShortcutOpenGuard;

impl Drop for CaptureShortcutOpenGuard {
    fn drop(&mut self) {
        CAPTURE_SHORTCUT_OPEN_IN_FLIGHT.store(false, Ordering::SeqCst);
    }
}

#[tauri::command]
pub async fn open_capture_window(
    mode: String,
    _app: AppHandle,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    open_capture_window_for_mode(state.inner(), &mode).await
}

pub async fn open_capture_window_for_mode(
    state: &crate::AppState,
    mode: &str,
) -> Result<(), String> {
    state
        .capture
        .session_runtime
        .open_capture_window_for_mode(mode)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_capture_session(
    state: State<'_, crate::AppState>,
) -> Result<CaptureSessionView, String> {
    state
        .capture
        .session_runtime
        .create_session_from_visible_desktop()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn reveal_capture_window(app: AppHandle) -> Result<(), String> {
    reveal_capture_window_on_main_thread(&app).await
}

#[tauri::command]
pub async fn prepare_capture_window_for_reveal(app: AppHandle) -> Result<(), String> {
    prepare_capture_window_for_reveal_on_main_thread(&app).await
}

#[tauri::command]
pub async fn hide_capture_window(app: AppHandle) -> Result<(), String> {
    hide_capture_window_on_main_thread(&app).await
}

fn capture_session_view_base64_bytes(session: &CaptureSessionView) -> usize {
    session
        .monitors
        .iter()
        .map(|monitor| monitor.image_base64.len())
        .sum::<usize>()
        + session
            .captured_cursor
            .as_ref()
            .map(|cursor| cursor.image_base64.len())
            .unwrap_or_default()
}

fn elapsed_ms(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}

async fn run_on_main_thread<T, F>(
    app: &AppHandle,
    operation_name: &'static str,
    operation: F,
) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(AppHandle) -> Result<T, String> + Send + 'static,
{
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let app_for_operation = app.clone();
    app.run_on_main_thread(move || {
        let _ = sender.send(operation(app_for_operation));
    })
    .map_err(|e| format!("Failed to dispatch {operation_name}: {e}"))?;

    receiver
        .await
        .map_err(|e| format!("Failed to receive {operation_name} result: {e}"))?
}

async fn reveal_capture_window_on_main_thread(app: &AppHandle) -> Result<(), String> {
    run_on_main_thread(app, "reveal capture window", |app| {
        reveal_capture_window_for_app(&app)
    })
    .await
}

async fn prepare_capture_window_for_reveal_on_main_thread(app: &AppHandle) -> Result<(), String> {
    run_on_main_thread(app, "prepare capture window for reveal", |app| {
        prepare_capture_window_for_reveal_for_app(&app)
    })
    .await
}

async fn hide_capture_window_on_main_thread(app: &AppHandle) -> Result<(), String> {
    run_on_main_thread(app, "hide capture window", |app| {
        hide_capture_window_for_app(&app)
    })
    .await
}

#[tauri::command]
pub fn get_capture_session(
    session_id: String,
    state: State<'_, crate::AppState>,
) -> Result<CaptureSessionView, String> {
    let start = Instant::now();
    let view = state
        .capture
        .sessions
        .get_session_view_without_monitor_images(&CaptureSessionId(session_id))
        .map_err(|e| e.to_string())?;
    let view_base64_bytes = capture_session_view_base64_bytes(&view);

    log::info!(
        "[capture-perf] get_capture_session session_id={} monitors={} candidates={} view_base64_bytes={} total_ms={:.1}",
        view.id.0,
        view.monitors.len(),
        view.candidates.len(),
        view_base64_bytes,
        elapsed_ms(start),
    );

    Ok(view)
}

#[tauri::command]
pub async fn hydrate_capture_session_snapshots(
    session_id: String,
    state: State<'_, crate::AppState>,
) -> Result<CaptureSessionView, String> {
    state
        .capture
        .sessions
        .hydrate_session_snapshots(&CaptureSessionId(session_id))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn log_capture_frontend_perf(
    event: String,
    mode: String,
    session_id: Option<String>,
    elapsed_ms: f64,
) {
    log::info!(
        "[capture-perf] frontend event={} mode={} session_id={} elapsed_ms={:.1}",
        event,
        mode,
        session_id.unwrap_or_else(|| "none".to_string()),
        elapsed_ms,
    );
}

#[tauri::command]
pub fn current_capture_cursor_position(
    session_id: String,
    state: State<'_, crate::AppState>,
) -> Result<Option<LogicalPoint>, String> {
    state
        .capture
        .sessions
        .current_cursor_position(&CaptureSessionId(session_id))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cancel_capture_session(
    session_id: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state
        .capture
        .session_runtime
        .cancel_capture_session(&CaptureSessionId(session_id))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn restore_capture_snapshot_windows_for_session(
    session_id: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state
        .capture
        .session_runtime
        .restore_capture_snapshot_windows_for_session(&CaptureSessionId(session_id))
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn render_capture_output(
    session_id: String,
    rect: LogicalRect,
    annotations: Vec<AnnotationCommand>,
    include_cursor: Option<bool>,
    state: State<'_, crate::AppState>,
) -> Result<String, String> {
    let session_id = CaptureSessionId(session_id);

    state
        .capture
        .session_runtime
        .render_png_base64(
            &session_id,
            &rect,
            &annotations,
            include_cursor.unwrap_or(false),
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn default_capture_save_path(state: State<'_, crate::AppState>) -> Result<String, String> {
    Ok(state
        .capture
        .output
        .default_capture_save_path()
        .to_string_lossy()
        .to_string())
}

#[tauri::command]
pub fn quick_capture_save_path(
    directory: Option<String>,
    state: State<'_, crate::AppState>,
) -> Result<String, String> {
    Ok(state
        .capture
        .output
        .quick_capture_save_path(directory.as_deref())
        .to_string_lossy()
        .to_string())
}

#[tauri::command]
pub async fn output_capture(
    session_id: String,
    rect: LogicalRect,
    annotations: Vec<AnnotationCommand>,
    include_cursor: Option<bool>,
    action: CaptureOutputAction,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    let session_id = CaptureSessionId(session_id);

    let output = state
        .capture
        .session_runtime
        .output_selection(
            &session_id,
            &rect,
            &annotations,
            include_cursor.unwrap_or(false),
            action,
        )
        .await
        .map_err(|e| e.to_string())?;

    match output {
        CaptureSessionOutput::Completed => Ok(()),
        CaptureSessionOutput::Pin(png_data) => state
            .capture
            .pinned_images
            .pin_png_and_open(png_data)
            .await
            .map_err(|error| error.to_string()),
    }
}

#[tauri::command]
pub async fn run_capture_ocr(
    session_id: String,
    rect: LogicalRect,
    state: State<'_, crate::AppState>,
) -> Result<OcrResult, String> {
    let capture_session_id = CaptureSessionId(session_id.clone());

    let result = state
        .capture
        .session_runtime
        .recognize_selection_text(&capture_session_id, &rect)
        .await
        .map_err(|e| e.to_string())?;

    log::info!(
        "Capture OCR completed: session_id={} text_chars={} rect={}x{}",
        session_id,
        result.text.chars().count(),
        rect.width,
        rect.height
    );

    Ok(result)
}

pub async fn open_capture_window_from_shortcut(app: AppHandle, mode: &'static str) {
    let Some(_guard) = try_begin_capture_shortcut_open() else {
        log::info!("Ignoring capture shortcut while a capture window is already opening");
        return;
    };

    let result = async {
        let state = app.state::<crate::AppState>();
        open_capture_window_for_mode(state.inner(), mode)
            .await
            .map_err(crate::AppError::from)
    }
    .await;

    if let Err(err) = result {
        log::error!("Failed to open capture window: {}", err);
    }
}

fn try_begin_capture_shortcut_open() -> Option<CaptureShortcutOpenGuard> {
    CAPTURE_SHORTCUT_OPEN_IN_FLIGHT
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .ok()
        .map(|_| CaptureShortcutOpenGuard)
}

#[cfg(test)]
mod tests {
    use super::try_begin_capture_shortcut_open;

    #[test]
    fn capture_shortcut_open_guard_blocks_reentrant_open_until_dropped() {
        let guard = try_begin_capture_shortcut_open().expect("first open should start");

        assert!(try_begin_capture_shortcut_open().is_none());

        drop(guard);
        assert!(try_begin_capture_shortcut_open().is_some());
    }
}

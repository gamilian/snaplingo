use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use tauri::{AppHandle, Manager, State};

use crate::application::services::CaptureSessionOutput;
use crate::domain::capture::{
    AnnotationCommand, CaptureOutputAction, CaptureSessionId, CaptureSessionView, LogicalPoint,
    LogicalRect, PinnedImageView,
};
use crate::domain::ocr::OcrResult;
use crate::infrastructure::system::capture_window::{
    begin_capture_presentation, capture_window_bounds, destroy_inactive_capture_window,
    end_capture_presentation, hide_capture_window as hide_capture_window_for_app,
    open_capture_window_for_session,
    prepare_capture_window_for_reveal as prepare_capture_window_for_reveal_for_app,
    restore_capture_snapshot_windows, reveal_capture_window as reveal_capture_window_for_app,
};
use crate::infrastructure::system::pinned_window::open_pinned_image_window;

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
    app: AppHandle,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    open_capture_window_for_mode(&app, &state, &mode).await
}

pub async fn open_capture_window_for_mode(
    app: &AppHandle,
    state: &crate::AppState,
    mode: &str,
) -> Result<(), String> {
    let total_start = Instant::now();
    let session_start = Instant::now();
    let session = create_triggered_capture_session_from_visible_desktop(app, state).await?;
    let session_ms = elapsed_ms(session_start);
    let monitor_count = session.monitors.len();
    let candidate_count = session.candidates.len();
    let view_base64_bytes = capture_session_view_base64_bytes(&session);

    let open_start = Instant::now();
    let open_result = match capture_window_bounds(&session.monitors) {
        Some(bounds) => {
            open_capture_window_for_session_on_main_thread(app, mode, &session.id.0, &bounds).await
        }
        None => Err("Cannot open capture window without monitor bounds".to_string()),
    };
    let open_ms = elapsed_ms(open_start);

    log::info!(
        "[capture-perf] open_capture_window mode={} monitors={} candidates={} view_base64_bytes={} create_session_ms={:.1} open_overlay_ms={:.1} total_ms={:.1} success={}",
        mode,
        monitor_count,
        candidate_count,
        view_base64_bytes,
        session_ms,
        open_ms,
        elapsed_ms(total_start),
        open_result.is_ok(),
    );

    if let Err(open_err) = open_result {
        let restore_result =
            restore_capture_snapshot_windows_for_session_id(app, state, &session.id).await;
        let _ = state.capture_session_service.cancel_session(&session.id);
        let presentation_result = end_capture_presentation_on_main_thread(app).await;

        if let Err(restore_err) = restore_result {
            return Err(format!(
                "{}; also failed to restore hidden windows: {}",
                open_err, restore_err
            ));
        }

        if let Err(presentation_err) = presentation_result {
            return Err(format!(
                "{}; also failed to end capture presentation: {}",
                open_err, presentation_err
            ));
        }

        return Err(open_err);
    }

    Ok(())
}

#[tauri::command]
pub async fn create_capture_session(
    app: AppHandle,
    state: State<'_, crate::AppState>,
) -> Result<CaptureSessionView, String> {
    create_capture_session_from_visible_desktop(&app, state.inner()).await
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

async fn create_triggered_capture_session_from_visible_desktop(
    app: &AppHandle,
    state: &crate::AppState,
) -> Result<CaptureSessionView, String> {
    create_capture_session_from_visible_desktop(app, state).await
}

async fn create_capture_session_from_visible_desktop(
    app: &AppHandle,
    state: &crate::AppState,
) -> Result<CaptureSessionView, String> {
    let total_start = Instant::now();
    let begin_start = Instant::now();
    begin_capture_presentation_on_main_thread(app).await?;
    let begin_ms = elapsed_ms(begin_start);

    let hide_overlay_start = Instant::now();
    if let Err(err) = hide_capture_window_on_main_thread(app).await {
        if err != "Capture window is not open" {
            let presentation_result = end_capture_presentation_on_main_thread(app).await;
            return match presentation_result {
                Ok(()) => Err(err),
                Err(presentation_err) => Err(format!(
                    "{}; also failed to end capture presentation: {}",
                    err, presentation_err
                )),
            };
        }
    }
    let hide_overlay_ms = elapsed_ms(hide_overlay_start);

    let session_start = Instant::now();
    let session_result = state
        .capture_session_service
        .create_session_without_monitor_images()
        .await
        .map_err(|e| e.to_string());
    let session_ms = elapsed_ms(session_start);

    log::info!(
        "[capture-perf] create_visible_desktop_frozen_session begin_ms={:.1} hide_overlay_ms={:.1} capture_session_ms={:.1} total_ms={:.1} success={}",
        begin_ms,
        hide_overlay_ms,
        session_ms,
        elapsed_ms(total_start),
        session_result.is_ok(),
    );

    match session_result {
        Ok(session) => Ok(session),
        Err(session_err) => {
            let presentation_result = end_capture_presentation_on_main_thread(app).await;

            match presentation_result {
                Ok(()) => Err(session_err),
                Err(presentation_err) => Err(format!(
                    "{}; also failed to end capture presentation: {}",
                    session_err, presentation_err
                )),
            }
        }
    }
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

async fn begin_capture_presentation_on_main_thread(app: &AppHandle) -> Result<(), String> {
    run_on_main_thread(app, "begin capture presentation", |app| {
        begin_capture_presentation(&app)
    })
    .await
}

async fn end_capture_presentation_on_main_thread(app: &AppHandle) -> Result<(), String> {
    run_on_main_thread(app, "end capture presentation", |app| {
        end_capture_presentation(&app)
    })
    .await
}

async fn open_capture_window_for_session_on_main_thread(
    app: &AppHandle,
    mode: &str,
    session_id: &str,
    bounds: &LogicalRect,
) -> Result<(), String> {
    let mode = mode.to_string();
    let session_id = session_id.to_string();
    let bounds = bounds.clone();

    run_on_main_thread(app, "open capture window", move |app| {
        open_capture_window_for_session(&app, &mode, &session_id, &bounds)
    })
    .await
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

async fn destroy_inactive_capture_window_on_main_thread(app: &AppHandle) -> Result<(), String> {
    run_on_main_thread(app, "destroy inactive capture window", |app| {
        destroy_inactive_capture_window(&app)
    })
    .await
}

async fn restore_capture_snapshot_windows_on_main_thread(
    app: &AppHandle,
    hidden_window_labels: Vec<String>,
) -> Result<(), String> {
    run_on_main_thread(app, "restore capture snapshot windows", move |app| {
        restore_capture_snapshot_windows(&app, &hidden_window_labels)
    })
    .await
}

async fn open_pinned_image_window_on_main_thread(
    app: &AppHandle,
    image: PinnedImageView,
) -> Result<(), String> {
    run_on_main_thread(app, "open pinned image window", move |app| {
        open_pinned_image_window(&app, &image)
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
        .capture_session_service
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
        .capture_session_service
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
        .capture_session_service
        .current_cursor_position(&CaptureSessionId(session_id))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cancel_capture_session(
    session_id: String,
    app: AppHandle,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    let session_id = CaptureSessionId(session_id);

    let restore_result =
        restore_capture_snapshot_windows_for_session_id(&app, state.inner(), &session_id).await;
    let cancel_result = state
        .capture_session_service
        .cancel_session(&session_id)
        .map_err(|e| e.to_string());
    let presentation_result = end_capture_presentation_on_main_thread(&app).await;
    let destroy_window_result = destroy_inactive_capture_window_on_main_thread(&app).await;

    match (
        restore_result,
        cancel_result,
        presentation_result,
        destroy_window_result,
    ) {
        (Ok(()), Ok(()), Ok(()), Ok(())) => Ok(()),
        (Err(err), _, _, _) => Err(err),
        (_, Err(err), _, _) => Err(err),
        (_, _, Err(err), _) => Err(err),
        (_, _, _, Err(err)) => Err(err),
    }
}

#[tauri::command]
pub async fn restore_capture_snapshot_windows_for_session(
    session_id: String,
    app: AppHandle,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    let restore_result = restore_capture_snapshot_windows_for_session_id(
        &app,
        state.inner(),
        &CaptureSessionId(session_id),
    )
    .await;
    let presentation_result = end_capture_presentation_on_main_thread(&app).await;
    let destroy_window_result = destroy_inactive_capture_window_on_main_thread(&app).await;

    match (restore_result, presentation_result, destroy_window_result) {
        (Ok(()), Ok(()), Ok(())) => Ok(()),
        (Err(err), Ok(()), Ok(())) => Err(err),
        (Ok(()), Err(err), Ok(())) => Err(err),
        (Ok(()), Ok(()), Err(err)) => Err(err),
        (Err(restore_err), Err(presentation_err), _) => Err(format!(
            "{}; also failed to end capture presentation: {}",
            restore_err, presentation_err
        )),
        (Err(restore_err), _, Err(destroy_err)) => Err(format!(
            "{}; also failed to destroy inactive capture window: {}",
            restore_err, destroy_err
        )),
        (_, Err(presentation_err), Err(destroy_err)) => Err(format!(
            "{}; also failed to destroy inactive capture window: {}",
            presentation_err, destroy_err
        )),
    }
}

async fn restore_capture_snapshot_windows_for_session_id(
    app: &AppHandle,
    state: &crate::AppState,
    session_id: &CaptureSessionId,
) -> Result<(), String> {
    let hidden_window_labels = state
        .capture_session_service
        .take_hidden_window_labels(session_id)
        .map_err(|e| e.to_string())?;

    restore_capture_snapshot_windows_on_main_thread(app, hidden_window_labels).await
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
    ensure_capture_session_cached_for_selection(state.inner(), &session_id, &rect)?;

    state
        .capture_session_runtime
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
        .capture_output_service
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
        .capture_output_service
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
    app: AppHandle,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    let session_id = CaptureSessionId(session_id);
    ensure_capture_session_cached_for_selection(state.inner(), &session_id, &rect)?;

    let output = state
        .capture_session_runtime
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
        CaptureSessionOutput::Pin(png_data) => {
            let image = state
                .inner()
                .pinned_image_service
                .pin_png_view(png_data)
                .map_err(|e| e.to_string())?;

            open_pinned_image_window_on_main_thread(&app, image).await
        }
    }
}

#[tauri::command]
pub async fn run_capture_ocr(
    session_id: String,
    rect: LogicalRect,
    state: State<'_, crate::AppState>,
) -> Result<OcrResult, String> {
    let session_id = CaptureSessionId(session_id);
    ensure_capture_session_cached_for_selection(state.inner(), &session_id, &rect)?;

    state
        .capture_session_runtime
        .recognize_selection_text(&session_id, &rect)
        .await
        .map_err(|e| e.to_string())
}

fn ensure_capture_session_cached_for_selection(
    state: &crate::AppState,
    session_id: &CaptureSessionId,
    rect: &LogicalRect,
) -> Result<(), String> {
    if state
        .capture_session_service
        .session_selection_needs_freeze(session_id, rect)
        .map_err(|e| e.to_string())?
    {
        return Err("Capture session snapshots are not ready for the selected area".to_string());
    }

    Ok(())
}

pub async fn open_capture_window_from_shortcut(app: AppHandle, mode: &'static str) {
    let Some(_guard) = try_begin_capture_shortcut_open() else {
        log::info!("Ignoring capture shortcut while a capture window is already opening");
        return;
    };

    let result = async {
        let state = app.state::<crate::AppState>();
        open_capture_window_for_mode(&app, state.inner(), mode)
            .await
            .map_err(crate::AppError::from)
    }
    .await;

    if let Err(err) = result {
        log::error!("Failed to open capture window: {}", err);
        super::emit_capture_screenshot_error(app, err.to_string());
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

use std::time::Instant;

use tauri::{AppHandle, Manager, State};

use crate::application::services::CaptureSessionOutput;
use crate::domain::capture::{
    AnnotationCommand, CaptureOutputAction, CaptureSessionId, CaptureSessionView, LogicalPoint,
    LogicalRect,
};
use crate::domain::ocr::OcrResult;
use crate::infrastructure::system::capture_window::{
    begin_capture_presentation, capture_snapshot_hide_settle_delay_ms, capture_window_bounds,
    end_capture_presentation, hide_capture_snapshot_windows,
    hide_capture_window as hide_capture_window_for_app, open_capture_window_for_session,
    restore_capture_snapshot_windows, reveal_capture_window as reveal_capture_window_for_app,
};
use crate::infrastructure::system::pinned_window::open_pinned_image_window;

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
    let session = create_capture_session_from_visible_desktop(app, state).await?;
    let session_ms = elapsed_ms(session_start);
    let monitor_count = session.monitors.len();
    let candidate_count = session.candidates.len();
    let view_base64_bytes = capture_session_view_base64_bytes(&session);

    let open_start = Instant::now();
    let open_result = capture_window_bounds(&session.monitors)
        .ok_or_else(|| "Cannot open capture window without monitor bounds".to_string())
        .and_then(|bounds| open_capture_window_for_session(app, mode, &session.id.0, &bounds));
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
            restore_capture_snapshot_windows_for_session_id(app, state, &session.id);
        let _ = state.capture_session_service.cancel_session(&session.id);
        let presentation_result = end_capture_presentation(app);

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
pub fn reveal_capture_window(app: AppHandle) -> Result<(), String> {
    reveal_capture_window_for_app(&app)
}

#[tauri::command]
pub fn hide_capture_window(app: AppHandle) -> Result<(), String> {
    hide_capture_window_for_app(&app)
}

async fn create_capture_session_from_visible_desktop(
    app: &AppHandle,
    state: &crate::AppState,
) -> Result<CaptureSessionView, String> {
    let total_start = Instant::now();
    let begin_start = Instant::now();
    begin_capture_presentation(app)?;
    let begin_ms = elapsed_ms(begin_start);

    let hide_start = Instant::now();
    let hidden_window_labels = match hide_capture_snapshot_windows(app) {
        Ok(labels) => labels,
        Err(err) => match end_capture_presentation(app) {
            Ok(()) => return Err(err),
            Err(presentation_err) => {
                return Err(format!(
                    "{}; also failed to end capture presentation: {}",
                    err, presentation_err
                ))
            }
        },
    };
    let hide_ms = elapsed_ms(hide_start);

    let settle_delay_ms = capture_snapshot_hide_settle_delay_ms(&hidden_window_labels);
    let settle_start = Instant::now();
    if settle_delay_ms > 0 {
        tokio::time::sleep(tokio::time::Duration::from_millis(settle_delay_ms)).await;
    }
    let settle_ms = elapsed_ms(settle_start);

    let session_start = Instant::now();
    let session_result = state
        .capture_session_service
        .create_session_with_hidden_window_labels(hidden_window_labels.clone())
        .await
        .map_err(|e| e.to_string());
    let session_ms = elapsed_ms(session_start);

    log::info!(
        "[capture-perf] create_visible_desktop_session hidden_windows={} begin_ms={:.1} hide_ms={:.1} settle_requested_ms={} settle_ms={:.1} capture_session_ms={:.1} total_ms={:.1} success={}",
        hidden_window_labels.len(),
        begin_ms,
        hide_ms,
        settle_delay_ms,
        settle_ms,
        session_ms,
        elapsed_ms(total_start),
        session_result.is_ok(),
    );

    match session_result {
        Ok(session) => Ok(session),
        Err(session_err) => {
            let restore_result = restore_capture_snapshot_windows(app, &hidden_window_labels);
            let presentation_result = end_capture_presentation(app);

            match (restore_result, presentation_result) {
                (Ok(()), Ok(())) => Err(session_err),
                (Err(restore_err), Ok(())) => Err(format!(
                    "{}; also failed to restore hidden windows: {}",
                    session_err, restore_err
                )),
                (Ok(()), Err(presentation_err)) => Err(format!(
                    "{}; also failed to end capture presentation: {}",
                    session_err, presentation_err
                )),
                (Err(restore_err), Err(presentation_err)) => Err(format!(
                    "{}; also failed to restore hidden windows: {}; also failed to end capture presentation: {}",
                    session_err, restore_err, presentation_err
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

#[tauri::command]
pub fn get_capture_session(
    session_id: String,
    state: State<'_, crate::AppState>,
) -> Result<CaptureSessionView, String> {
    state
        .capture_session_service
        .get_session_view(&CaptureSessionId(session_id))
        .map_err(|e| e.to_string())
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
        restore_capture_snapshot_windows_for_session_id(&app, state.inner(), &session_id);
    let cancel_result = state
        .capture_session_service
        .cancel_session(&session_id)
        .map_err(|e| e.to_string());
    let presentation_result = end_capture_presentation(&app);

    match (restore_result, cancel_result, presentation_result) {
        (Ok(()), Ok(()), Ok(())) => Ok(()),
        (Err(err), _, _) => Err(err),
        (_, Err(err), _) => Err(err),
        (_, _, Err(err)) => Err(err),
    }
}

#[tauri::command]
pub fn restore_capture_snapshot_windows_for_session(
    session_id: String,
    app: AppHandle,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    let restore_result = restore_capture_snapshot_windows_for_session_id(
        &app,
        state.inner(),
        &CaptureSessionId(session_id),
    );
    let presentation_result = end_capture_presentation(&app);

    match (restore_result, presentation_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(err), Ok(())) => Err(err),
        (Ok(()), Err(err)) => Err(err),
        (Err(restore_err), Err(presentation_err)) => Err(format!(
            "{}; also failed to end capture presentation: {}",
            restore_err, presentation_err
        )),
    }
}

fn restore_capture_snapshot_windows_for_session_id(
    app: &AppHandle,
    state: &crate::AppState,
    session_id: &CaptureSessionId,
) -> Result<(), String> {
    let hidden_window_labels = state
        .capture_session_service
        .take_hidden_window_labels(session_id)
        .map_err(|e| e.to_string())?;

    restore_capture_snapshot_windows(app, &hidden_window_labels)
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

            open_pinned_image_window(&app, &image)
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
    state
        .capture_session_runtime
        .recognize_selection_text(&session_id, &rect)
        .await
        .map_err(|e| e.to_string())
}

pub async fn open_capture_window_from_shortcut(app: AppHandle, mode: &'static str) {
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

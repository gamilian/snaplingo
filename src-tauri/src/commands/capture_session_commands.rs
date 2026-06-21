use tauri::{AppHandle, Manager, State};

use crate::application::services::CaptureSessionOutput;
use crate::domain::capture::{
    AnnotationCommand, CaptureOutputAction, CaptureSessionId, CaptureSessionView, LogicalRect,
};
use crate::domain::ocr::OcrResult;
use crate::infrastructure::system::capture_window::{
    capture_snapshot_hide_settle_delay_ms, capture_window_bounds, hide_capture_snapshot_windows,
    open_capture_window_for_session, restore_capture_snapshot_windows,
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
    let session = create_capture_session_from_visible_desktop(app, state).await?;
    let open_result = capture_window_bounds(&session.monitors)
        .ok_or_else(|| "Cannot open capture window without monitor bounds".to_string())
        .and_then(|bounds| open_capture_window_for_session(app, mode, &session.id.0, &bounds));

    if let Err(open_err) = open_result {
        let restore_result =
            restore_capture_snapshot_windows_for_session_id(app, state, &session.id);
        let _ = state.capture_session_service.cancel_session(&session.id);

        if let Err(restore_err) = restore_result {
            return Err(format!(
                "{}; also failed to restore hidden windows: {}",
                open_err, restore_err
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

async fn create_capture_session_from_visible_desktop(
    app: &AppHandle,
    state: &crate::AppState,
) -> Result<CaptureSessionView, String> {
    let hidden_window_labels = hide_capture_snapshot_windows(app)?;
    let settle_delay_ms = capture_snapshot_hide_settle_delay_ms(&hidden_window_labels);
    if settle_delay_ms > 0 {
        tokio::time::sleep(tokio::time::Duration::from_millis(settle_delay_ms)).await;
    }

    let session_result = state
        .capture_session_service
        .create_session_with_hidden_window_labels(hidden_window_labels.clone())
        .await
        .map_err(|e| e.to_string());

    match session_result {
        Ok(session) => Ok(session),
        Err(session_err) => match restore_capture_snapshot_windows(app, &hidden_window_labels) {
            Ok(()) => Err(session_err),
            Err(restore_err) => Err(format!(
                "{}; also failed to restore hidden windows: {}",
                session_err, restore_err
            )),
        },
    }
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
pub async fn cancel_capture_session(
    session_id: String,
    app: AppHandle,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    let session_id = CaptureSessionId(session_id);

    restore_capture_snapshot_windows_for_session_id(&app, state.inner(), &session_id)?;

    state
        .capture_session_service
        .cancel_session(&session_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn restore_capture_snapshot_windows_for_session(
    session_id: String,
    app: AppHandle,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    restore_capture_snapshot_windows_for_session_id(
        &app,
        state.inner(),
        &CaptureSessionId(session_id),
    )
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
        .capture_session_service
        .render_png_base64(
            &state.image_composition_service,
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
        .capture_session_service
        .output_selection(
            &state.image_composition_service,
            &state.capture_output_service,
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
        .capture_session_service
        .recognize_selection_text(
            &state.image_composition_service,
            &state.ocr_coordinator,
            &session_id,
            &rect,
        )
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
        super::emit_screenshot_error(app, err.to_string());
    }
}

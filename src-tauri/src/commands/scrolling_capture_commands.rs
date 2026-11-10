use crate::domain::capture::{
    CaptureSessionId, LogicalRect, ScrollingCaptureOutputAction, ScrollingCaptureView,
};
use tauri::State;

#[tauri::command]
pub async fn begin_scrolling_capture(
    session_id: String,
    rect: LogicalRect,
    state: State<'_, crate::AppState>,
) -> Result<ScrollingCaptureView, String> {
    state
        .capture
        .runtime
        .begin_scrolling_capture(CaptureSessionId(session_id), rect)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_scrolling_capture(
    capture_id: String,
    state: State<'_, crate::AppState>,
) -> Result<ScrollingCaptureView, String> {
    state
        .capture
        .runtime
        .scrolling_capture_view(&capture_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn capture_scrolling_frame(
    capture_id: String,
    state: State<'_, crate::AppState>,
) -> Result<ScrollingCaptureView, String> {
    state
        .capture
        .runtime
        .capture_scrolling_frame(&capture_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn undo_scrolling_capture(
    capture_id: String,
    state: State<'_, crate::AppState>,
) -> Result<ScrollingCaptureView, String> {
    state
        .capture
        .runtime
        .undo_scrolling_capture(&capture_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn finish_scrolling_capture(
    capture_id: String,
    action: ScrollingCaptureOutputAction,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state
        .capture
        .runtime
        .finish_scrolling_capture(&capture_id, action)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn cancel_scrolling_capture(
    capture_id: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state
        .capture
        .runtime
        .cancel_scrolling_capture(&capture_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn set_scrolling_capture_preview(
    capture_id: String,
    visible: bool,
    state: State<'_, crate::AppState>,
) -> Result<bool, String> {
    state
        .capture
        .runtime
        .set_scrolling_preview_visible(&capture_id, visible)
        .await
        .map_err(|error| error.to_string())
}

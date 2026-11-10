use tauri::State;

use crate::application::screenshot_favorites::{
    ScreenshotFavoritePage, ScreenshotFavoriteQuery, ScreenshotFavoriteRecord,
};
use crate::domain::capture::{AnnotationCommand, CaptureSessionId, LogicalRect};

#[tauri::command]
pub async fn favorite_capture_selection(
    session_id: String,
    rect: LogicalRect,
    annotations: Vec<AnnotationCommand>,
    include_cursor: Option<bool>,
    state: State<'_, crate::AppState>,
) -> Result<ScreenshotFavoriteRecord, String> {
    state
        .screenshot_favorites
        .capture
        .add_selection(
            &CaptureSessionId(session_id),
            &rect,
            &annotations,
            include_cursor.unwrap_or(false),
        )
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn query_screenshot_favorites(
    query: ScreenshotFavoriteQuery,
    state: State<'_, crate::AppState>,
) -> Result<ScreenshotFavoritePage, String> {
    state
        .screenshot_favorites
        .favorites
        .query(query)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn update_screenshot_favorite_metadata(
    id: i64,
    note: Option<String>,
    tags: Vec<String>,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state
        .screenshot_favorites
        .favorites
        .update_metadata(id, note, tags)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn delete_screenshot_favorite(
    id: i64,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state
        .screenshot_favorites
        .favorites
        .delete(id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn copy_screenshot_favorite(
    id: i64,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state
        .screenshot_favorites
        .favorites
        .copy(id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn reveal_screenshot_favorite(
    id: i64,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state
        .screenshot_favorites
        .favorites
        .reveal(id)
        .await
        .map_err(|error| error.to_string())
}

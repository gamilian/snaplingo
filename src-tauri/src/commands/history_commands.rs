use crate::application::favorites::{FavoriteKind, FavoritePage, FavoriteQuery};
use crate::application::history::{
    HistoryEntry, HistoryKind, HistoryPage, HistoryQuery, OcrHistoryEntry, TranslationHistoryEntry,
};
use crate::domain::ocr::OcrRequest;
use crate::domain::ocr::OcrResult;
use crate::domain::translation::{TranslationRequest, TranslationResult};
use crate::AppState;
use tauri::State;

/// Get translation history with pagination
#[tauri::command]
pub async fn get_translation_history(
    limit: usize,
    offset: usize,
    state: State<'_, AppState>,
) -> Result<Vec<TranslationHistoryEntry>, String> {
    let result: crate::Result<Vec<TranslationHistoryEntry>> = state
        .history
        .history
        .get_translation_history(limit, offset)
        .await;

    result.map_err(|e| e.to_string())
}

/// Get OCR history with pagination
#[tauri::command]
pub async fn get_ocr_history(
    limit: usize,
    offset: usize,
    state: State<'_, AppState>,
) -> Result<Vec<OcrHistoryEntry>, String> {
    let result: crate::Result<Vec<OcrHistoryEntry>> =
        state.history.history.get_ocr_history(limit, offset).await;

    result.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn query_translation_history(
    query: HistoryQuery,
    state: State<'_, AppState>,
) -> Result<HistoryPage<TranslationHistoryEntry>, String> {
    state
        .history
        .history
        .query_translation_history(query)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn query_ocr_history(
    query: HistoryQuery,
    state: State<'_, AppState>,
) -> Result<HistoryPage<OcrHistoryEntry>, String> {
    state
        .history
        .history
        .query_ocr_history(query)
        .await
        .map_err(|error| error.to_string())
}

/// Search history by query string
#[tauri::command]
pub async fn search_history(
    query: String,
    state: State<'_, AppState>,
) -> Result<Vec<HistoryEntry>, String> {
    let result: crate::Result<Vec<HistoryEntry>> =
        state.history.history.search_history(&query).await;

    result.map_err(|e| e.to_string())
}

/// Delete a history entry by ID
#[tauri::command]
pub async fn delete_history(id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let result: crate::Result<()> = state.history.history.delete_history(id).await;
    result.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_history_favorite(
    id: i64,
    favorite: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .history
        .history
        .set_history_favorite(id, favorite)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn favorite_translation_result(
    source_history_id: Option<i64>,
    request: TranslationRequest,
    result: TranslationResult,
    state: State<'_, AppState>,
) -> Result<i64, String> {
    state
        .favorites
        .favorites
        .add_translation(source_history_id, request, result)
        .await
        .map(|record| record.id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn favorite_ocr_result(
    source_history_id: Option<i64>,
    request: OcrRequest,
    result: OcrResult,
    provider_used: Option<String>,
    state: State<'_, AppState>,
) -> Result<i64, String> {
    let provider_used = provider_used.unwrap_or_else(|| {
        state
            .providers
            .ocr
            .get_active()
            .map(|provider| provider.read().id().to_string())
            .unwrap_or_else(|| "manual".to_string())
    });
    let image_data = if request.image_data.is_empty() {
        match source_history_id {
            Some(id) => state
                .history
                .history
                .read_ocr_source(id)
                .await
                .map_err(|error| error.to_string())?,
            None => Vec::new(),
        }
    } else {
        request.image_data
    };
    state
        .favorites
        .favorites
        .add_ocr(
            source_history_id,
            image_data,
            request.language,
            provider_used,
            result,
        )
        .await
        .map(|record| record.id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn query_favorites(
    query: FavoriteQuery,
    state: State<'_, AppState>,
) -> Result<FavoritePage, String> {
    state
        .favorites
        .favorites
        .query(query)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn update_favorite_metadata(
    id: i64,
    note: Option<String>,
    tags: Vec<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .favorites
        .favorites
        .update_metadata(id, note, tags)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn delete_favorite(id: i64, state: State<'_, AppState>) -> Result<(), String> {
    state
        .favorites
        .favorites
        .delete(id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn rerun_ocr_favorite(id: i64, state: State<'_, AppState>) -> Result<OcrResult, String> {
    let image = state
        .favorites
        .favorites
        .read_ocr_source(id)
        .await
        .map_err(|error| error.to_string())?;
    state
        .providers
        .ocr
        .recognize_image(image)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn list_favorite_tags(
    kind: FavoriteKind,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    state
        .favorites
        .favorites
        .list_tags(kind)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn update_history_note(
    id: i64,
    note: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .history
        .history
        .update_history_note(id, note)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn replace_history_tags(
    id: i64,
    tags: Vec<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .history
        .history
        .replace_history_tags(id, tags)
        .await
        .map_err(|error| error.to_string())
}

/// Clear all history
#[tauri::command]
pub async fn clear_all_history(state: State<'_, AppState>) -> Result<(), String> {
    let result: crate::Result<()> = state.history.history.clear_all_history().await;
    result.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clear_history(kind: HistoryKind, state: State<'_, AppState>) -> Result<(), String> {
    state
        .history
        .history
        .clear_history(kind)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn rerun_ocr_history(id: i64, state: State<'_, AppState>) -> Result<OcrResult, String> {
    state
        .history
        .ocr_replay
        .run(id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn export_translation_favorites(
    path: String,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    state
        .history
        .history
        .export_translation_favorites(&path)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn list_history_tags(
    kind: HistoryKind,
    favorite_only: bool,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    state
        .history
        .history
        .list_tags(kind, favorite_only)
        .await
        .map_err(|error| error.to_string())
}

use crate::infrastructure::storage::{HistoryEntry, OcrHistoryEntry, TranslationHistoryEntry};
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
        .service
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
        state.history.service.get_ocr_history(limit, offset).await;

    result.map_err(|e| e.to_string())
}

/// Search history by query string
#[tauri::command]
pub async fn search_history(
    query: String,
    state: State<'_, AppState>,
) -> Result<Vec<HistoryEntry>, String> {
    let result: crate::Result<Vec<HistoryEntry>> =
        state.history.service.search_history(&query).await;

    result.map_err(|e| e.to_string())
}

/// Delete a history entry by ID
#[tauri::command]
pub async fn delete_history(id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let result: crate::Result<()> = state.history.service.delete_history(id).await;

    result.map_err(|e| e.to_string())
}

/// Clear all history
#[tauri::command]
pub async fn clear_all_history(state: State<'_, AppState>) -> Result<(), String> {
    let result: crate::Result<()> = state.history.service.clear_all_history().await;

    result.map_err(|e| e.to_string())
}

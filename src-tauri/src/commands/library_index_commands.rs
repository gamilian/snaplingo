use tauri::State;

use crate::application::library_index::{LibraryIndexPage, LibraryIndexQuery};
use crate::AppState;

#[tauri::command]
pub async fn query_library_history_index(
    query: LibraryIndexQuery,
    state: State<'_, AppState>,
) -> Result<LibraryIndexPage, String> {
    state
        .library_index
        .query_history(query)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn query_library_favorite_index(
    query: LibraryIndexQuery,
    state: State<'_, AppState>,
) -> Result<LibraryIndexPage, String> {
    state
        .library_index
        .query_favorites(query)
        .await
        .map_err(|error| error.to_string())
}

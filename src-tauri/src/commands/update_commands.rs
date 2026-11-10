use crate::application::updates::AppUpdateStatus;
use tauri::State;

#[tauri::command]
pub async fn check_app_update(
    state: State<'_, crate::AppState>,
) -> Result<AppUpdateStatus, String> {
    state
        .updates
        .check()
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn download_app_update(state: State<'_, crate::AppState>) -> Result<String, String> {
    state
        .updates
        .download_and_open()
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn open_app_release_page(state: State<'_, crate::AppState>) -> Result<(), String> {
    state
        .updates
        .open_release_page()
        .map_err(|error| error.to_string())
}

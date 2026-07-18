use crate::application::RequiredPermissionsStatus;
use tauri::State;

#[tauri::command]
pub fn get_required_permissions_status(
    state: State<'_, crate::AppState>,
) -> RequiredPermissionsStatus {
    state.permissions.status()
}

#[tauri::command]
pub fn request_required_permissions(
    state: State<'_, crate::AppState>,
) -> RequiredPermissionsStatus {
    state.permissions.request_next_missing()
}

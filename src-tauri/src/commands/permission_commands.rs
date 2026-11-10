use crate::application::{RequiredPermissionsContext, RequiredPermissionsStatus, SystemPermission};
use tauri::State;

#[tauri::command]
pub fn get_required_permissions_status(
    state: State<'_, crate::AppState>,
) -> RequiredPermissionsStatus {
    state.permissions.status()
}

#[tauri::command]
pub fn get_required_permissions_context(
    state: State<'_, crate::AppState>,
) -> RequiredPermissionsContext {
    state.permissions.context()
}

#[tauri::command]
pub async fn request_required_permission(
    state: State<'_, crate::AppState>,
    permission: SystemPermission,
) -> Result<RequiredPermissionsStatus, String> {
    let permissions = state.permissions.clone();
    tauri::async_runtime::spawn_blocking(move || permissions.request(permission))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn reset_required_permission(
    state: State<'_, crate::AppState>,
    permission: SystemPermission,
) -> Result<RequiredPermissionsStatus, String> {
    let permissions = state.permissions.clone();
    tauri::async_runtime::spawn_blocking(move || permissions.reset(permission))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn restart_after_permission_change(state: State<'_, crate::AppState>) -> Result<(), String> {
    state
        .permissions
        .restart()
        .map_err(|error| error.to_string())
}

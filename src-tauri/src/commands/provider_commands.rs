use tauri::State;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub is_configured: bool,
    pub requires_api_key: bool,
    pub is_active: bool,
}

#[tauri::command]
pub async fn list_translation_providers(
    state: State<'_, crate::AppState>,
) -> Result<Vec<ProviderInfo>, String> {
    let all_providers = state.translation_coordinator.list_all();
    let active = state.translation_coordinator.get_active();
    let active_ids: Vec<_> = active.iter().map(|p| p.id().to_string()).collect();

    let info: Vec<_> = all_providers.iter().map(|p| ProviderInfo {
        id: p.id().to_string(),
        name: p.name().to_string(),
        is_configured: p.is_configured(),
        requires_api_key: p.requires_api_key(),
        is_active: active_ids.contains(&p.id().to_string()),
    }).collect();

    Ok(info)
}

#[tauri::command]
pub async fn activate_translation_provider(
    provider_id: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state.translation_coordinator
        .activate(&provider_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn deactivate_translation_provider(
    provider_id: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state.translation_coordinator
        .deactivate(&provider_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn configure_translation_provider(
    provider_id: String,
    api_key: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    // Save to keychain
    state.keychain
        .save_provider_credential(&provider_id, &api_key)
        .map_err(|e| e.to_string())?;

    // Update provider configuration
    // Note: This requires provider to support set_api_key, which would need
    // interior mutability or a different pattern. For now, providers are
    // recreated on startup with credentials from keychain.

    Ok(())
}

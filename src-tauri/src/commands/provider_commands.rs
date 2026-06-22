use crate::application::providers::common::CredentialField;
use crate::application::providers::{
    add_custom_translation_provider as add_custom_translation_provider_to_config,
    validate_required_credentials, AddCustomTranslationProviderInput, CustomTranslationProviderDef,
    CustomTranslationProviderView,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::State;

#[derive(Serialize, Deserialize)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub is_configured: bool,
    pub requires_api_key: bool,
    pub is_active: bool,
    // Custom provider 额外字段
    pub is_builtin: bool,
    pub protocol: Option<String>,
    pub endpoint: Option<String>,
    pub model: Option<String>,
    pub reasoning_level: Option<String>,
}

impl From<CustomTranslationProviderView> for ProviderInfo {
    fn from(view: CustomTranslationProviderView) -> Self {
        Self {
            id: view.id,
            name: view.name,
            is_configured: true,
            requires_api_key: true,
            is_active: true,
            is_builtin: false,
            protocol: Some(view.protocol),
            endpoint: Some(view.endpoint),
            model: Some(view.model),
            reasoning_level: view.reasoning_level,
        }
    }
}

#[tauri::command]
pub async fn list_translation_providers(
    state: State<'_, crate::AppState>,
) -> Result<Vec<ProviderInfo>, String> {
    let all_providers = state.translation_coordinator.list_all();
    let active = state.translation_coordinator.get_active();
    let active_ids: Vec<_> = active.iter().map(|p| p.read().id().to_string()).collect();

    // Load custom provider definitions for extra metadata
    let custom_defs = state
        .config_file
        .load::<Vec<CustomTranslationProviderDef>>("custom_translation_providers")
        .unwrap_or_default();

    let info: Vec<_> = all_providers
        .iter()
        .map(|p| {
            let provider = p.read();
            let id = provider.id().to_string();
            let is_builtin = matches!(
                id.as_str(),
                "google-translate" | "deepl" | "baidu-translate"
            );

            // Find matching custom def
            let custom_def = custom_defs.iter().find(|def| def.id == id);

            ProviderInfo {
                id: id.clone(),
                name: provider.name().to_string(),
                is_configured: provider.is_configured(),
                requires_api_key: provider.requires_api_key(),
                is_active: active_ids.contains(&id),
                is_builtin,
                protocol: custom_def.map(|def| format!("{:?}", def.protocol).to_lowercase()),
                endpoint: custom_def.map(|def| def.endpoint.clone()),
                model: custom_def.map(|def| def.model.clone()),
                reasoning_level: custom_def.and_then(|def| {
                    def.reasoning_level
                        .map(|level| format!("{:?}", level).to_lowercase())
                }),
            }
        })
        .collect();

    Ok(info)
}

#[tauri::command]
pub async fn activate_translation_provider(
    provider_id: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state
        .translation_coordinator
        .activate(&provider_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn deactivate_translation_provider(
    provider_id: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state
        .translation_coordinator
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
    state
        .keychain
        .save_provider_credential(&provider_id, &api_key)
        .map_err(|e| e.to_string())?;

    // Update provider configuration
    // Note: This requires provider to support set_api_key, which would need
    // interior mutability or a different pattern. For now, providers are
    // recreated on startup with credentials from keychain.

    Ok(())
}

#[tauri::command]
pub async fn reorder_active_translation_providers(
    provider_ids: Vec<String>,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state
        .translation_coordinator
        .reorder_active(provider_ids)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_provider_credential_schema(
    provider_id: String,
    state: State<'_, crate::AppState>,
) -> Result<Vec<CredentialField>, String> {
    let providers = state.translation_coordinator.list_all();
    let provider_lock = providers
        .iter()
        .find(|p| p.read().id() == provider_id)
        .ok_or_else(|| format!("Provider not found: {}", provider_id))?;

    let fields = provider_lock.read().credential_fields();
    Ok(fields)
}

#[tauri::command]
pub async fn configure_translation_provider_credentials(
    provider_id: String,
    credentials: HashMap<String, String>,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    // Validate provider exists
    let providers = state.translation_coordinator.list_all();
    let provider_lock = providers
        .iter()
        .find(|p| p.read().id() == provider_id)
        .ok_or_else(|| format!("Provider not found: {}", provider_id))?;

    // Get expected fields
    let expected_fields = provider_lock.read().credential_fields();

    validate_required_credentials(&expected_fields, &credentials).map_err(|e| e.to_string())?;

    // Save to keychain
    state
        .keychain
        .save_provider_credentials(&provider_id, &credentials)
        .map_err(|e| e.to_string())?;

    // Reconfigure provider at runtime (hot-reload)
    state
        .translation_coordinator
        .reconfigure_provider(&provider_id, &credentials)
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[derive(Serialize, Deserialize)]
pub struct AddCustomTranslationProviderRequest {
    pub name: String,
    pub protocol: String, // "openai" | "anthropic" | "gemini"
    pub endpoint: String,
    pub model: String,
    pub api_key: String,
    pub reasoning_level: Option<String>, // "minimal" | "low" | "medium" | "high" | "xhigh"
}

#[tauri::command]
pub async fn add_custom_translation_provider(
    request: AddCustomTranslationProviderRequest,
    state: State<'_, crate::AppState>,
) -> Result<ProviderInfo, String> {
    let input = AddCustomTranslationProviderInput {
        name: request.name,
        protocol: request.protocol,
        endpoint: request.endpoint,
        model: request.model,
        api_key: request.api_key,
        reasoning_level: request.reasoning_level,
    };

    let view = add_custom_translation_provider_to_config(
        input,
        &state.config_file,
        &state.keychain,
        state.http_client.clone(),
        &state.translation_coordinator,
    )
    .map_err(|e| e.to_string())?;

    Ok(ProviderInfo::from(view))
}

#[tauri::command]
pub async fn remove_custom_translation_provider(
    provider_id: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    // Step 1: Load existing custom providers
    let mut custom_defs = state
        .config_file
        .load::<Vec<CustomTranslationProviderDef>>("custom_translation_providers")
        .unwrap_or_default();

    // Step 2: Check if provider exists and is not builtin
    let builtin_ids = ["google-translate", "deepl", "baidu-translate"];
    if builtin_ids.contains(&provider_id.as_str()) {
        return Err("Cannot remove builtin provider".into());
    }

    let index = custom_defs
        .iter()
        .position(|def| def.id == provider_id)
        .ok_or_else(|| format!("Provider not found: {}", provider_id))?;

    // Step 3: Deactivate and unregister
    state
        .translation_coordinator
        .unregister(&provider_id)
        .map_err(|e| format!("Failed to unregister: {}", e))?;

    // Step 4: Remove from config
    custom_defs.remove(index);
    state
        .config_file
        .save("custom_translation_providers", &custom_defs)
        .map_err(|e| format!("Failed to save config: {}", e))?;

    // Step 5: Delete keychain entry
    state
        .keychain
        .delete_provider_credential(&provider_id)
        .map_err(|e| format!("Failed to delete credential: {}", e))?;

    Ok(())
}

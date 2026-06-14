use tauri::State;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use crate::{CustomTranslationProviderDef, create_llm_translation_provider};
use crate::application::providers::common::CredentialField;

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

#[tauri::command]
pub async fn list_translation_providers(
    state: State<'_, crate::AppState>,
) -> Result<Vec<ProviderInfo>, String> {
    let all_providers = state.translation_coordinator.list_all();
    let active = state.translation_coordinator.get_active();
    let active_ids: Vec<_> = active.iter().map(|p| p.id().to_string()).collect();

    // Load custom provider definitions for extra metadata
    let custom_defs = state.config_file
        .load::<Vec<CustomTranslationProviderDef>>("custom_translation_providers")
        .unwrap_or_default();

    let info: Vec<_> = all_providers.iter().map(|p| {
        let id = p.id().to_string();
        let is_builtin = matches!(id.as_str(), "google-translate" | "deepl" | "baidu-translate");

        // Find matching custom def
        let custom_def = custom_defs.iter().find(|def| def.id == id);

        ProviderInfo {
            id: id.clone(),
            name: p.name().to_string(),
            is_configured: p.is_configured(),
            requires_api_key: p.requires_api_key(),
            is_active: active_ids.contains(&id),
            is_builtin,
            protocol: custom_def.map(|def| format!("{:?}", def.protocol).to_lowercase()),
            endpoint: custom_def.map(|def| def.endpoint.clone()),
            model: custom_def.map(|def| def.model.clone()),
            reasoning_level: custom_def.and_then(|def|
                def.reasoning_level.map(|level| format!("{:?}", level).to_lowercase())
            ),
        }
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

#[tauri::command]
pub async fn reorder_active_translation_providers(
    provider_ids: Vec<String>,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state.translation_coordinator
        .reorder_active(provider_ids)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_provider_credential_schema(
    provider_id: String,
    state: State<'_, crate::AppState>,
) -> Result<Vec<CredentialField>, String> {
    let providers = state.translation_coordinator.list_all();
    let provider = providers
        .iter()
        .find(|p| p.id() == provider_id)
        .ok_or_else(|| format!("Provider not found: {}", provider_id))?;

    Ok(provider.credential_fields())
}

#[tauri::command]
pub async fn configure_translation_provider_credentials(
    provider_id: String,
    credentials: HashMap<String, String>,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    // Validate provider exists
    let providers = state.translation_coordinator.list_all();
    let provider = providers
        .iter()
        .find(|p| p.id() == provider_id)
        .ok_or_else(|| format!("Provider not found: {}", provider_id))?;

    // Get expected fields
    let expected_fields = provider.credential_fields();

    // Validate all required fields are present
    for field in &expected_fields {
        if !credentials.contains_key(&field.name) {
            return Err(format!("Missing required field: {}", field.label));
        }
        if credentials.get(&field.name).unwrap().trim().is_empty() {
            return Err(format!("Field cannot be empty: {}", field.label));
        }
    }

    // Save to keychain
    state.keychain
        .save_provider_credentials(&provider_id, &credentials)
        .map_err(|e| e.to_string())?;

    // Note: Providers are currently recreated on startup with credentials from keychain.
    // Runtime reconfiguration would require providers to support interior mutability.

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
    use crate::infrastructure::llm::{LLMProtocol, ReasoningLevel};

    // Validate fields
    if request.name.trim().is_empty() {
        return Err("Name cannot be empty".into());
    }
    if request.endpoint.trim().is_empty() {
        return Err("Endpoint cannot be empty".into());
    }
    if request.model.trim().is_empty() {
        return Err("Model cannot be empty".into());
    }
    if request.api_key.trim().is_empty() {
        return Err("API key cannot be empty".into());
    }

    // Parse protocol
    let protocol = match request.protocol.as_str() {
        "openai" => LLMProtocol::OpenAI,
        "anthropic" => LLMProtocol::Anthropic,
        "gemini" => LLMProtocol::Gemini,
        _ => return Err(format!("Invalid protocol: {}", request.protocol)),
    };

    // Parse reasoning level
    let reasoning_level = match request.reasoning_level.as_deref() {
        Some("minimal") => Some(ReasoningLevel::Minimal),
        Some("low") => Some(ReasoningLevel::Low),
        Some("medium") => Some(ReasoningLevel::Medium),
        Some("high") => Some(ReasoningLevel::High),
        Some("xhigh") => Some(ReasoningLevel::XHigh),
        Some(other) => return Err(format!("Invalid reasoning level: {}", other)),
        None => None,
    };

    // Generate unique ID
    let id = format!(
        "custom-llm-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );

    // Step 1: Save API key to keychain
    state.keychain
        .save_provider_credential(&id, &request.api_key)
        .map_err(|e| format!("Failed to save API key: {}", e))?;

    // Step 2: Create provider definition
    let def = CustomTranslationProviderDef {
        id: id.clone(),
        name: request.name.clone(),
        protocol,
        endpoint: request.endpoint.clone(),
        model: request.model.clone(),
        reasoning_level,
    };

    // Step 3: Load existing custom providers and append
    let mut custom_defs = state.config_file
        .load::<Vec<CustomTranslationProviderDef>>("custom_translation_providers")
        .unwrap_or_default();
    custom_defs.push(def.clone());

    // Step 4: Save to ConfigFile
    state.config_file
        .save("custom_translation_providers", &custom_defs)
        .map_err(|e| {
            // Rollback: delete keychain entry
            let _ = state.keychain.delete_provider_credential(&id);
            format!("Failed to save config: {}", e)
        })?;

    // Step 5: Create and register provider at runtime
    let provider = create_llm_translation_provider(&def, state.http_client.clone(), request.api_key.clone());
    state.translation_coordinator
        .register(provider)
        .map_err(|e| {
            // Rollback: remove from config and keychain
            custom_defs.pop();
            let _ = state.config_file.save("custom_translation_providers", &custom_defs);
            let _ = state.keychain.delete_provider_credential(&id);
            format!("Failed to register provider: {}", e)
        })?;

    // Step 6: Activate immediately
    state.translation_coordinator
        .activate(&id)
        .map_err(|e| format!("Failed to activate provider: {}", e))?;

    // Step 7: Return ProviderInfo
    Ok(ProviderInfo {
        id: id.clone(),
        name: request.name,
        is_configured: true,
        requires_api_key: true,
        is_active: true,
        is_builtin: false,
        protocol: Some(request.protocol),
        endpoint: Some(request.endpoint),
        model: Some(request.model),
        reasoning_level: reasoning_level.map(|level| format!("{:?}", level).to_lowercase()),
    })
}

#[tauri::command]
pub async fn remove_custom_translation_provider(
    provider_id: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    // Step 1: Load existing custom providers
    let mut custom_defs = state.config_file
        .load::<Vec<CustomTranslationProviderDef>>("custom_translation_providers")
        .unwrap_or_default();

    // Step 2: Check if provider exists and is not builtin
    let builtin_ids = ["google-translate", "deepl", "baidu-translate"];
    if builtin_ids.contains(&provider_id.as_str()) {
        return Err("Cannot remove builtin provider".into());
    }

    let index = custom_defs.iter().position(|def| def.id == provider_id);
    if index.is_none() {
        return Err(format!("Provider not found: {}", provider_id));
    }

    // Step 3: Deactivate and unregister
    state.translation_coordinator
        .unregister(&provider_id)
        .map_err(|e| format!("Failed to unregister: {}", e))?;

    // Step 4: Remove from config
    custom_defs.remove(index.unwrap());
    state.config_file
        .save("custom_translation_providers", &custom_defs)
        .map_err(|e| format!("Failed to save config: {}", e))?;

    // Step 5: Delete keychain entry
    state.keychain
        .delete_provider_credential(&provider_id)
        .map_err(|e| format!("Failed to delete credential: {}", e))?;

    Ok(())
}

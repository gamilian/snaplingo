use crate::application::providers::common::CredentialField;
use crate::application::providers::{
    AddCustomTranslationProviderInput, ProviderConnectionTestInput, ProviderInfo,
    ProviderModelListInput, TranslationPromptStrategyConfig, UpdateCustomTranslationProviderInput,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::State;

#[tauri::command]
pub async fn list_translation_providers(
    state: State<'_, crate::AppState>,
) -> Result<Vec<ProviderInfo>, String> {
    Ok(state.providers.administration.list_translation_providers())
}

#[tauri::command]
pub async fn activate_translation_provider(
    provider_id: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state
        .providers
        .administration
        .activate_translation_provider(provider_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn deactivate_translation_provider(
    provider_id: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state
        .providers
        .administration
        .deactivate_translation_provider(provider_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn reorder_active_translation_providers(
    provider_ids: Vec<String>,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state
        .providers
        .administration
        .reorder_active_translation_providers(provider_ids)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_provider_credential_schema(
    provider_id: String,
    state: State<'_, crate::AppState>,
) -> Result<Vec<CredentialField>, String> {
    state
        .providers
        .administration
        .translation_credential_schema(provider_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn configure_translation_provider_credentials(
    provider_id: String,
    credentials: HashMap<String, String>,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state
        .providers
        .administration
        .configure_translation_provider_credentials(provider_id, credentials)
        .map_err(|e| e.to_string())
}

#[derive(Serialize, Deserialize)]
pub struct AddCustomTranslationProviderRequest {
    pub name: String,
    pub protocol: String, // "openai" | "anthropic" | "gemini"
    pub endpoint: String,
    pub model: String,
    pub api_key: String,
    pub reasoning_level: Option<String>, // "minimal" | "low" | "medium" | "high" | "xhigh"
    pub prompt_strategy_id: Option<String>,
    pub prompt_fallback_strategy_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct UpdateCustomTranslationProviderRequest {
    pub name: String,
    pub protocol: String, // "openai" | "anthropic" | "gemini"
    pub endpoint: String,
    pub model: String,
    pub api_key: Option<String>,
    pub reasoning_level: Option<String>, // "minimal" | "low" | "medium" | "high" | "xhigh"
    pub prompt_strategy_id: Option<String>,
    pub prompt_fallback_strategy_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct OpenAICompatibleModelsRequest {
    pub endpoint: String,
    pub api_key: String,
}

#[derive(Serialize, Deserialize)]
pub struct TestOpenAICompatibleProviderRequest {
    pub endpoint: String,
    pub api_key: String,
    pub model: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenAICompatibleModelInfo {
    pub id: String,
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
        prompt_strategy_id: request.prompt_strategy_id,
        prompt_fallback_strategy_id: request.prompt_fallback_strategy_id,
    };

    state
        .providers
        .administration
        .add_custom_translation_provider(input)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_custom_translation_provider(
    provider_id: String,
    request: UpdateCustomTranslationProviderRequest,
    state: State<'_, crate::AppState>,
) -> Result<ProviderInfo, String> {
    let input = UpdateCustomTranslationProviderInput {
        name: request.name,
        protocol: request.protocol,
        endpoint: request.endpoint,
        model: request.model,
        api_key: request.api_key,
        reasoning_level: request.reasoning_level,
        prompt_strategy_id: request.prompt_strategy_id,
        prompt_fallback_strategy_id: request.prompt_fallback_strategy_id,
    };

    state
        .providers
        .administration
        .update_custom_translation_provider(provider_id, input)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_translation_prompt_strategies(
    state: State<'_, crate::AppState>,
) -> Result<TranslationPromptStrategyConfig, String> {
    Ok(state
        .providers
        .administration
        .list_translation_prompt_strategies())
}

#[tauri::command]
pub async fn save_translation_prompt_strategies(
    config: TranslationPromptStrategyConfig,
    state: State<'_, crate::AppState>,
) -> Result<TranslationPromptStrategyConfig, String> {
    state
        .providers
        .administration
        .save_translation_prompt_strategies(config)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_openai_compatible_models(
    request: OpenAICompatibleModelsRequest,
    state: State<'_, crate::AppState>,
) -> Result<Vec<OpenAICompatibleModelInfo>, String> {
    state
        .providers
        .administration
        .list_openai_compatible_models(model_list_input(request))
        .await
        .map(|models| {
            models
                .into_iter()
                .map(|m| OpenAICompatibleModelInfo { id: m.id })
                .collect()
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn test_openai_compatible_provider(
    request: TestOpenAICompatibleProviderRequest,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state
        .providers
        .administration
        .test_openai_compatible_provider(connection_test_input(request))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn test_openai_responses_provider(
    request: TestOpenAICompatibleProviderRequest,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state
        .providers
        .administration
        .test_openai_responses_provider(connection_test_input(request))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_anthropic_models(
    request: OpenAICompatibleModelsRequest,
    state: State<'_, crate::AppState>,
) -> Result<Vec<OpenAICompatibleModelInfo>, String> {
    state
        .providers
        .administration
        .list_anthropic_models(model_list_input(request))
        .await
        .map(|models| {
            models
                .into_iter()
                .map(|m| OpenAICompatibleModelInfo { id: m.id })
                .collect()
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn test_anthropic_provider(
    request: TestOpenAICompatibleProviderRequest,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state
        .providers
        .administration
        .test_anthropic_provider(connection_test_input(request))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_gemini_models(
    request: OpenAICompatibleModelsRequest,
    state: State<'_, crate::AppState>,
) -> Result<Vec<OpenAICompatibleModelInfo>, String> {
    state
        .providers
        .administration
        .list_gemini_models(model_list_input(request))
        .await
        .map(|models| {
            models
                .into_iter()
                .map(|m| OpenAICompatibleModelInfo { id: m.id })
                .collect()
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn test_gemini_provider(
    request: TestOpenAICompatibleProviderRequest,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state
        .providers
        .administration
        .test_gemini_provider(connection_test_input(request))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn test_custom_translation_provider(
    provider_id: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state
        .providers
        .administration
        .test_custom_translation_provider(provider_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_custom_translation_provider(
    provider_id: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state
        .providers
        .administration
        .remove_custom_translation_provider(provider_id)
        .map_err(|e| e.to_string())
}

fn model_list_input(request: OpenAICompatibleModelsRequest) -> ProviderModelListInput {
    ProviderModelListInput {
        endpoint: request.endpoint,
        api_key: request.api_key,
    }
}

fn connection_test_input(
    request: TestOpenAICompatibleProviderRequest,
) -> ProviderConnectionTestInput {
    ProviderConnectionTestInput {
        endpoint: request.endpoint,
        api_key: request.api_key,
        model: request.model,
    }
}

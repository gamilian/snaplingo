use crate::application::providers::common::CredentialField;
use crate::application::providers::configuration::{
    CredentialValue, ProviderInfo,
};
use crate::application::providers::{
    merge_prompt_strategy_config, sanitize_prompt_strategy_config,
    validate_prompt_strategy_config,
    AddCustomTranslationProviderInput,
    CustomTranslationProviderView, TranslationPromptStrategyConfig,
    UpdateCustomTranslationProviderInput,
};
use crate::infrastructure::llm::LLMProtocol;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::State;

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
            prompt_strategy_id: Some(view.prompt_strategy_id),
            prompt_fallback_strategy_id: Some(view.prompt_fallback_strategy_id),
        }
    }
}

#[tauri::command]
pub async fn list_translation_providers(
    state: State<'_, crate::AppState>,
) -> Result<Vec<ProviderInfo>, String> {
    let info = state.provider_configuration.list_provider_infos();
    let active = state.translation_coordinator.get_active();
    let active_ids: Vec<_> = active.iter().map(|p| p.read().id().to_string()).collect();

    Ok(order_provider_infos_for_display(info, &active_ids))
}

fn order_provider_infos_for_display(
    providers: Vec<ProviderInfo>,
    active_ids: &[String],
) -> Vec<ProviderInfo> {
    let mut by_id: HashMap<_, _> = providers
        .into_iter()
        .map(|provider| (provider.id.clone(), provider))
        .collect();

    let mut ordered = Vec::new();
    for id in active_ids {
        if let Some(provider) = by_id.remove(id) {
            ordered.push(provider);
        }
    }

    let mut inactive: Vec<_> = by_id.into_values().collect();
    inactive.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));
    ordered.extend(inactive);

    ordered
}

#[tauri::command]
pub async fn activate_translation_provider(
    provider_id: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state
        .provider_configuration
        .activate_provider(provider_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn deactivate_translation_provider(
    provider_id: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state
        .provider_configuration
        .deactivate_provider(provider_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn configure_translation_provider(
    provider_id: String,
    api_key: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    let mut credentials = HashMap::new();
    if provider_id == "deeplx" {
        credentials.insert("mode".to_string(), "deepl".to_string());
        credentials.insert("api_key".to_string(), api_key);
    } else {
        credentials.insert("api_key".to_string(), api_key);
    }

    let cred_values: Vec<CredentialValue> = credentials
        .into_iter()
        .map(|(key, value)| CredentialValue { key, value })
        .collect();

    state.provider_configuration
        .save_credentials(provider_id, cred_values)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn reorder_active_translation_providers(
    provider_ids: Vec<String>,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state
        .provider_configuration
        .reorder_active_providers(provider_ids)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_provider_credential_schema(
    provider_id: String,
    state: State<'_, crate::AppState>,
) -> Result<Vec<CredentialField>, String> {
    state.provider_configuration
        .credential_schema(provider_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn configure_translation_provider_credentials(
    provider_id: String,
    credentials: HashMap<String, String>,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    let cred_values: Vec<CredentialValue> = credentials
        .into_iter()
        .map(|(key, value)| CredentialValue { key, value })
        .collect();

    state.provider_configuration
        .save_credentials(provider_id, cred_values)
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

    let view = state
        .provider_configuration
        .add(input)
        .map_err(|e| e.to_string())?;

    Ok(ProviderInfo::from(view))
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

    let view = state
        .provider_configuration
        .update(provider_id.clone(), input)
        .map_err(|e| e.to_string())?;

    let mut info = ProviderInfo::from(view);
    info.is_active = state
        .translation_coordinator
        .get_active()
        .iter()
        .any(|provider| provider.read().id() == provider_id);
    Ok(info)
}

#[tauri::command]
pub async fn list_translation_prompt_strategies(
    state: State<'_, crate::AppState>,
) -> Result<TranslationPromptStrategyConfig, String> {
    let stored = state
        .config_file
        .load::<TranslationPromptStrategyConfig>("translation_prompt_strategies")
        .ok();
    Ok(merge_prompt_strategy_config(stored))
}

#[tauri::command]
pub async fn save_translation_prompt_strategies(
    config: TranslationPromptStrategyConfig,
    state: State<'_, crate::AppState>,
) -> Result<TranslationPromptStrategyConfig, String> {
    validate_prompt_strategy_config(&config).map_err(|e| e.to_string())?;
    let config = sanitize_prompt_strategy_config(config);
    state
        .config_file
        .save("translation_prompt_strategies", &config)
        .map_err(|e| format!("Failed to save prompt strategies: {}", e))?;
    Ok(config)
}

#[tauri::command]
pub async fn list_openai_compatible_models(
    request: OpenAICompatibleModelsRequest,
    state: State<'_, crate::AppState>,
) -> Result<Vec<OpenAICompatibleModelInfo>, String> {
    let endpoint = validate_non_blank(&request.endpoint, "API address")?;
    let api_key = validate_non_blank(&request.api_key, "API key")?;

    state
        .llm_introspection
        .list_models(LLMProtocol::OpenAI, endpoint, api_key)
        .await
        .map(|models| {
            models
                .into_iter()
                .map(|m| OpenAICompatibleModelInfo { id: m.id })
                .collect()
        })
        .map_err(|e| format!("Failed to fetch model list: {}", e))
}

#[tauri::command]
pub async fn test_openai_compatible_provider(
    request: TestOpenAICompatibleProviderRequest,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    let endpoint = validate_non_blank(&request.endpoint, "API address")?;
    let api_key = validate_non_blank(&request.api_key, "API key")?;
    let model = validate_non_blank(&request.model, "Model")?;

    state
        .llm_introspection
        .test(LLMProtocol::OpenAI, endpoint, model, api_key)
        .await
        .map_err(|e| format!("Provider test failed: {}", e))
}

#[tauri::command]
pub async fn test_openai_responses_provider(
    request: TestOpenAICompatibleProviderRequest,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    let endpoint = validate_non_blank(&request.endpoint, "API address")?;
    let api_key = validate_non_blank(&request.api_key, "API key")?;
    let model = validate_non_blank(&request.model, "Model")?;

    state
        .llm_introspection
        .test(LLMProtocol::OpenAIResponses, endpoint, model, api_key)
        .await
        .map_err(|e| format!("Provider test failed: {}", e))
}

#[tauri::command]
pub async fn list_anthropic_models(
    request: OpenAICompatibleModelsRequest,
    state: State<'_, crate::AppState>,
) -> Result<Vec<OpenAICompatibleModelInfo>, String> {
    let endpoint = validate_non_blank(&request.endpoint, "API address")?;
    let api_key = validate_non_blank(&request.api_key, "API key")?;

    state
        .llm_introspection
        .list_models(LLMProtocol::Anthropic, endpoint, api_key)
        .await
        .map(|models| {
            models
                .into_iter()
                .map(|m| OpenAICompatibleModelInfo { id: m.id })
                .collect()
        })
        .map_err(|e| format!("Failed to fetch model list: {}", e))
}

#[tauri::command]
pub async fn test_anthropic_provider(
    request: TestOpenAICompatibleProviderRequest,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    let endpoint = validate_non_blank(&request.endpoint, "API address")?;
    let api_key = validate_non_blank(&request.api_key, "API key")?;
    let model = validate_non_blank(&request.model, "Model")?;

    state
        .llm_introspection
        .test(LLMProtocol::Anthropic, endpoint, model, api_key)
        .await
        .map_err(|e| format!("Provider test failed: {}", e))
}

#[tauri::command]
pub async fn list_gemini_models(
    request: OpenAICompatibleModelsRequest,
    state: State<'_, crate::AppState>,
) -> Result<Vec<OpenAICompatibleModelInfo>, String> {
    let endpoint = validate_non_blank(&request.endpoint, "API address")?;
    let api_key = validate_non_blank(&request.api_key, "API key")?;

    state
        .llm_introspection
        .list_models(LLMProtocol::Gemini, endpoint, api_key)
        .await
        .map(|models| {
            models
                .into_iter()
                .map(|m| OpenAICompatibleModelInfo { id: m.id })
                .collect()
        })
        .map_err(|e| format!("Failed to fetch model list: {}", e))
}

#[tauri::command]
pub async fn test_gemini_provider(
    request: TestOpenAICompatibleProviderRequest,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    let endpoint = validate_non_blank(&request.endpoint, "API address")?;
    let api_key = validate_non_blank(&request.api_key, "API key")?;
    let model = validate_non_blank(&request.model, "Model")?;

    state
        .llm_introspection
        .test(LLMProtocol::Gemini, endpoint, model, api_key)
        .await
        .map_err(|e| format!("Provider test failed: {}", e))
}

#[tauri::command]
pub async fn test_custom_translation_provider(
    provider_id: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state
        .provider_configuration
        .test_custom_provider(provider_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_custom_translation_provider(
    provider_id: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state
        .provider_configuration
        .remove(provider_id)
        .map_err(|e| e.to_string())
}

fn validate_non_blank<'a>(value: &'a str, label: &str) -> Result<&'a str, String> {
    let value = value.trim();
    if value.is_empty() {
        Err(format!("{} cannot be empty", label))
    } else {
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider_info(id: &str, name: &str, is_active: bool) -> ProviderInfo {
        ProviderInfo {
            id: id.to_string(),
            name: name.to_string(),
            is_configured: true,
            requires_api_key: false,
            is_active,
            is_builtin: false,
            protocol: None,
            endpoint: None,
            model: None,
            reasoning_level: None,
            prompt_strategy_id: None,
            prompt_fallback_strategy_id: None,
        }
    }

    #[test]
    fn provider_list_keeps_active_provider_order_before_inactive_providers() {
        let providers = vec![
            provider_info("google-translate", "Google Translate", true),
            provider_info("custom-gpt", "gpt-5-mini", true),
            provider_info("deeplx", "DeepLX", false),
        ];
        let active_ids = vec!["custom-gpt".to_string(), "google-translate".to_string()];

        let ordered = order_provider_infos_for_display(providers, &active_ids);
        let ordered_ids: Vec<_> = ordered
            .iter()
            .map(|provider| provider.id.as_str())
            .collect();

        assert_eq!(
            ordered_ids,
            vec!["custom-gpt", "google-translate", "deeplx"]
        );
    }
}

use crate::application::providers::common::CredentialField;
use crate::application::providers::{
    add_custom_translation_provider as add_custom_translation_provider_to_config,
    build_updated_custom_translation_provider_def, create_llm_translation_provider,
    custom_translation_provider_view, merge_prompt_strategy_config,
    sanitize_prompt_strategy_config, validate_prompt_strategy_config,
    validate_required_credentials, AddCustomTranslationProviderInput, CustomTranslationProviderDef,
    CustomTranslationProviderView, TranslationPromptStrategyConfig,
    UpdateCustomTranslationProviderInput,
};
use crate::infrastructure::llm::{
    anthropic_models_url, gemini_models_url, openai_compatible_models_url, AnthropicLLMClient,
    GeminiLLMClient, LLMClient, LLMOptions, LLMProtocol, LLMRequest, OpenAILLMClient,
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
    pub prompt_strategy_id: Option<String>,
    pub prompt_fallback_strategy_id: Option<String>,
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
            prompt_strategy_id: Some(view.prompt_strategy_id),
            prompt_fallback_strategy_id: Some(view.prompt_fallback_strategy_id),
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
                "google-translate" | "deeplx" | "baidu-translate"
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
                protocol: custom_def.map(|def| def.protocol.as_str().to_string()),
                endpoint: custom_def.map(|def| def.endpoint.clone()),
                model: custom_def.map(|def| def.model.clone()),
                reasoning_level: custom_def.and_then(|def| {
                    def.reasoning_level
                        .map(|level| format!("{:?}", level).to_lowercase())
                }),
                prompt_strategy_id: custom_def.map(|def| def.prompt_strategy_id.clone()),
                prompt_fallback_strategy_id: custom_def
                    .map(|def| def.prompt_fallback_strategy_id.clone()),
            }
        })
        .collect();

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
    let mut credentials = HashMap::new();
    if provider_id == "deeplx" {
        credentials.insert("mode".to_string(), "deepl".to_string());
        credentials.insert("api_key".to_string(), api_key);
    } else {
        credentials.insert("api_key".to_string(), api_key);
    }
    configure_translation_provider_credentials_inner(&provider_id, &credentials, state.inner())
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
    configure_translation_provider_credentials_inner(&provider_id, &credentials, state.inner())
}

fn configure_translation_provider_credentials_inner(
    provider_id: &str,
    credentials: &HashMap<String, String>,
    state: &crate::AppState,
) -> Result<(), String> {
    let providers = state.translation_coordinator.list_all();
    let provider_lock = providers
        .iter()
        .find(|p| p.read().id() == provider_id)
        .ok_or_else(|| format!("Provider not found: {provider_id}"))?;

    let expected_fields = provider_lock.read().credential_fields();

    if provider_id == "deeplx" {
        validate_deeplx_credentials(credentials)?;
    } else {
        validate_required_credentials(&expected_fields, credentials).map_err(|e| e.to_string())?;
    }

    state
        .keychain
        .save_provider_credentials(provider_id, credentials)
        .map_err(|e| e.to_string())?;

    if let Some(api_key) = credentials.get("api_key") {
        state
            .keychain
            .save_provider_credential(provider_id, api_key)
            .map_err(|e| e.to_string())?;
    }

    state
        .translation_coordinator
        .reconfigure_provider(provider_id, credentials)
        .map_err(|e| e.to_string())?;

    Ok(())
}

fn validate_deeplx_credentials(credentials: &HashMap<String, String>) -> Result<(), String> {
    let mode = credentials
        .get("mode")
        .map(String::as_str)
        .unwrap_or("deeplx");

    match mode {
        "deepl" => validate_non_blank(
            credentials.get("api_key").map(String::as_str).unwrap_or(""),
            "DeepL API Key",
        )
        .map(|_| ()),
        "deeplx" => validate_non_blank(
            credentials
                .get("endpoint")
                .map(String::as_str)
                .unwrap_or(""),
            "DeepLX API address",
        )
        .map(|_| ()),
        other => Err(format!("Invalid DeepLX mode: {}", other)),
    }
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

    let view = add_custom_translation_provider_to_config(
        input,
        state.config_file.clone(),
        &state.keychain,
        state.http_client.clone(),
        &state.translation_coordinator,
    )
    .map_err(|e| e.to_string())?;

    Ok(ProviderInfo::from(view))
}

#[tauri::command]
pub async fn update_custom_translation_provider(
    provider_id: String,
    request: UpdateCustomTranslationProviderRequest,
    state: State<'_, crate::AppState>,
) -> Result<ProviderInfo, String> {
    let mut custom_defs = state
        .config_file
        .load::<Vec<CustomTranslationProviderDef>>("custom_translation_providers")
        .unwrap_or_default();

    let index = custom_defs
        .iter()
        .position(|def| def.id == provider_id)
        .ok_or_else(|| format!("Provider not found: {}", provider_id))?;

    if state.translation_coordinator.get(&provider_id).is_none() {
        return Err(format!("Provider not found: {}", provider_id));
    }

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
    let def = build_updated_custom_translation_provider_def(provider_id.clone(), &input)
        .map_err(|e| e.to_string())?;

    let api_key = if let Some(api_key) = input
        .api_key
        .as_deref()
        .map(str::trim)
        .filter(|key| !key.is_empty())
    {
        state
            .keychain
            .save_provider_credential(&provider_id, api_key)
            .map_err(|e| format!("Failed to save API key: {}", e))?;
        api_key.to_string()
    } else {
        state
            .keychain
            .load_provider_credential(&provider_id)
            .map_err(|e| format!("Failed to load existing API key: {}", e))?
    };

    custom_defs[index] = def.clone();
    state
        .config_file
        .save("custom_translation_providers", &custom_defs)
        .map_err(|e| format!("Failed to save config: {}", e))?;

    let provider = create_llm_translation_provider(
        &def,
        state.http_client.clone(),
        api_key,
        state.config_file.clone(),
    );
    state
        .translation_coordinator
        .replace(provider)
        .map_err(|e| format!("Failed to update provider: {}", e))?;

    let mut info = ProviderInfo::from(custom_translation_provider_view(&def));
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
    let url = openai_compatible_models_url(endpoint);

    let response = state
        .http_client
        .get(&url, openai_authorization_headers(api_key))
        .await
        .map_err(|e| format!("Failed to fetch model list: {}", e))?;

    ensure_openai_compatible_success_status(response.status, &response.body)?;
    parse_openai_compatible_models_response(&response.body)
}

#[tauri::command]
pub async fn test_openai_compatible_provider(
    request: TestOpenAICompatibleProviderRequest,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    let endpoint = validate_non_blank(&request.endpoint, "API address")?;
    let api_key = validate_non_blank(&request.api_key, "API key")?;
    let model = validate_non_blank(&request.model, "Model")?;

    let client = OpenAILLMClient::new_chat_completions(
        state.http_client.clone(),
        endpoint.to_string(),
        model.to_string(),
        api_key.to_string(),
    );
    let request = LLMRequest {
        system_prompt: Some("You are a translation engine. Return only OK.".to_string()),
        user_prompt: "OK".to_string(),
        options: LLMOptions {
            reasoning: None,
            temperature: Some(0.0),
            max_tokens: Some(8),
        },
    };

    client
        .generate(&request)
        .await
        .map(|_| ())
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

    let client = OpenAILLMClient::new_responses(
        state.http_client.clone(),
        endpoint.to_string(),
        model.to_string(),
        api_key.to_string(),
    );

    test_llm_client(client).await
}

#[tauri::command]
pub async fn list_anthropic_models(
    request: OpenAICompatibleModelsRequest,
    state: State<'_, crate::AppState>,
) -> Result<Vec<OpenAICompatibleModelInfo>, String> {
    let endpoint = validate_non_blank(&request.endpoint, "API address")?;
    let api_key = validate_non_blank(&request.api_key, "API key")?;
    let url = anthropic_models_url(endpoint);

    let response = state
        .http_client
        .get(&url, anthropic_headers(api_key))
        .await
        .map_err(|e| format!("Failed to fetch model list: {}", e))?;

    ensure_openai_compatible_success_status(response.status, &response.body)?;
    parse_anthropic_models_response(&response.body)
}

#[tauri::command]
pub async fn test_anthropic_provider(
    request: TestOpenAICompatibleProviderRequest,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    let endpoint = validate_non_blank(&request.endpoint, "API address")?;
    let api_key = validate_non_blank(&request.api_key, "API key")?;
    let model = validate_non_blank(&request.model, "Model")?;

    let client = AnthropicLLMClient::new(
        state.http_client.clone(),
        endpoint.to_string(),
        model.to_string(),
        api_key.to_string(),
    );

    test_llm_client(client).await
}

#[tauri::command]
pub async fn list_gemini_models(
    request: OpenAICompatibleModelsRequest,
    state: State<'_, crate::AppState>,
) -> Result<Vec<OpenAICompatibleModelInfo>, String> {
    let endpoint = validate_non_blank(&request.endpoint, "API address")?;
    let api_key = validate_non_blank(&request.api_key, "API key")?;
    let url = gemini_models_url(endpoint, api_key);

    let response = state
        .http_client
        .get(&url, HashMap::new())
        .await
        .map_err(|e| format!("Failed to fetch model list: {}", e))?;

    ensure_openai_compatible_success_status(response.status, &response.body)?;
    parse_gemini_models_response(&response.body)
}

#[tauri::command]
pub async fn test_gemini_provider(
    request: TestOpenAICompatibleProviderRequest,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    let endpoint = validate_non_blank(&request.endpoint, "API address")?;
    let api_key = validate_non_blank(&request.api_key, "API key")?;
    let model = validate_non_blank(&request.model, "Model")?;

    let client = GeminiLLMClient::new(
        state.http_client.clone(),
        endpoint.to_string(),
        model.to_string(),
        api_key.to_string(),
    );

    test_llm_client(client).await
}

#[tauri::command]
pub async fn test_custom_translation_provider(
    provider_id: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    let custom_defs = state
        .config_file
        .load::<Vec<CustomTranslationProviderDef>>("custom_translation_providers")
        .unwrap_or_default();

    let def = custom_defs
        .iter()
        .find(|def| def.id == provider_id)
        .ok_or_else(|| format!("Provider not found: {}", provider_id))?;

    let api_key = state
        .keychain
        .load_provider_credential(&provider_id)
        .map_err(|e| format!("Failed to load provider credential: {}", e))?;

    test_custom_translation_provider_def(def, &api_key, state.inner()).await
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
    let builtin_ids = ["google-translate", "deeplx", "baidu-translate"];
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

fn validate_non_blank<'a>(value: &'a str, label: &str) -> Result<&'a str, String> {
    let value = value.trim();
    if value.is_empty() {
        Err(format!("{} cannot be empty", label))
    } else {
        Ok(value)
    }
}

fn openai_authorization_headers(api_key: &str) -> HashMap<String, String> {
    HashMap::from([
        ("Authorization".to_string(), format!("Bearer {}", api_key)),
        ("Content-Type".to_string(), "application/json".to_string()),
    ])
}

fn anthropic_headers(api_key: &str) -> HashMap<String, String> {
    HashMap::from([
        ("x-api-key".to_string(), api_key.to_string()),
        ("anthropic-version".to_string(), "2023-06-01".to_string()),
        ("Content-Type".to_string(), "application/json".to_string()),
    ])
}

async fn test_llm_client(client: impl LLMClient) -> Result<(), String> {
    let request = LLMRequest {
        system_prompt: Some("You are a translation engine. Return only OK.".to_string()),
        user_prompt: "OK".to_string(),
        options: LLMOptions {
            reasoning: None,
            temperature: Some(0.0),
            max_tokens: Some(8),
        },
    };

    client
        .generate(&request)
        .await
        .map(|_| ())
        .map_err(|e| format!("Provider test failed: {}", e))
}

async fn test_custom_translation_provider_def(
    def: &CustomTranslationProviderDef,
    api_key: &str,
    state: &crate::AppState,
) -> Result<(), String> {
    let endpoint = validate_non_blank(&def.endpoint, "API address")?;
    let api_key = validate_non_blank(api_key, "API key")?;
    let model = validate_non_blank(&def.model, "Model")?;

    match def.protocol {
        LLMProtocol::OpenAI => {
            let client = OpenAILLMClient::new_chat_completions(
                state.http_client.clone(),
                endpoint.to_string(),
                model.to_string(),
                api_key.to_string(),
            );
            test_llm_client(client).await
        }
        LLMProtocol::OpenAIResponses => {
            let client = OpenAILLMClient::new_responses(
                state.http_client.clone(),
                endpoint.to_string(),
                model.to_string(),
                api_key.to_string(),
            );
            test_llm_client(client).await
        }
        LLMProtocol::Anthropic => {
            let client = AnthropicLLMClient::new(
                state.http_client.clone(),
                endpoint.to_string(),
                model.to_string(),
                api_key.to_string(),
            );
            test_llm_client(client).await
        }
        LLMProtocol::Gemini => {
            let client = GeminiLLMClient::new(
                state.http_client.clone(),
                endpoint.to_string(),
                model.to_string(),
                api_key.to_string(),
            );
            test_llm_client(client).await
        }
    }
}

fn ensure_openai_compatible_success_status(status: u16, body: &str) -> Result<(), String> {
    match status {
        200 => Ok(()),
        401 | 403 => Err("Invalid API key or insufficient permission".to_string()),
        404 => Err("API endpoint not found".to_string()),
        429 => Err("Rate limit exceeded".to_string()),
        _ => Err(format!("API returned status {}: {}", status, body)),
    }
}

fn parse_openai_compatible_models_response(
    body: &str,
) -> Result<Vec<OpenAICompatibleModelInfo>, String> {
    let json: serde_json::Value =
        serde_json::from_str(body).map_err(|e| format!("Model list JSON parse failed: {}", e))?;
    let data = json["data"]
        .as_array()
        .ok_or_else(|| "Model list response is missing data array".to_string())?;

    let models: Vec<_> = data
        .iter()
        .filter_map(|item| {
            item["id"]
                .as_str()
                .or_else(|| item.as_str())
                .map(|id| OpenAICompatibleModelInfo { id: id.to_string() })
        })
        .collect();

    if models.is_empty() {
        Err("Model list response did not contain model ids".to_string())
    } else {
        Ok(models)
    }
}

fn parse_anthropic_models_response(body: &str) -> Result<Vec<OpenAICompatibleModelInfo>, String> {
    let json: serde_json::Value =
        serde_json::from_str(body).map_err(|e| format!("Model list JSON parse failed: {}", e))?;
    let data = json["data"]
        .as_array()
        .ok_or_else(|| "Model list response is missing data array".to_string())?;
    models_from_array(data, "id")
}

fn parse_gemini_models_response(body: &str) -> Result<Vec<OpenAICompatibleModelInfo>, String> {
    let json: serde_json::Value =
        serde_json::from_str(body).map_err(|e| format!("Model list JSON parse failed: {}", e))?;
    let data = json["models"]
        .as_array()
        .ok_or_else(|| "Model list response is missing models array".to_string())?;
    let models: Vec<_> = data
        .iter()
        .filter_map(|item| item["name"].as_str())
        .map(|name| OpenAICompatibleModelInfo {
            id: name.strip_prefix("models/").unwrap_or(name).to_string(),
        })
        .collect();

    if models.is_empty() {
        Err("Model list response did not contain model ids".to_string())
    } else {
        Ok(models)
    }
}

fn models_from_array(
    data: &[serde_json::Value],
    field: &str,
) -> Result<Vec<OpenAICompatibleModelInfo>, String> {
    let models: Vec<_> = data
        .iter()
        .filter_map(|item| {
            item[field]
                .as_str()
                .or_else(|| item.as_str())
                .map(|id| OpenAICompatibleModelInfo { id: id.to_string() })
        })
        .collect();

    if models.is_empty() {
        Err("Model list response did not contain model ids".to_string())
    } else {
        Ok(models)
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

    #[test]
    fn parses_openai_compatible_model_list_response() {
        let models = parse_openai_compatible_models_response(
            r#"{"data":[{"id":"DeepSeek-V4-Pro"},{"id":"GLM-5.1"}]}"#,
        )
        .unwrap();
        let ids: Vec<_> = models.iter().map(|model| model.id.as_str()).collect();

        assert_eq!(ids, vec!["DeepSeek-V4-Pro", "GLM-5.1"]);
    }

    #[test]
    fn rejects_openai_compatible_model_list_without_data_array() {
        let error = parse_openai_compatible_models_response(r#"{"object":"list"}"#).unwrap_err();

        assert_eq!(error, "Model list response is missing data array");
    }

    #[test]
    fn parses_anthropic_model_list_response() {
        let models = parse_anthropic_models_response(
            r#"{"data":[{"id":"claude-sonnet-4-5"},{"id":"claude-haiku-4-5"}]}"#,
        )
        .unwrap();
        let ids: Vec<_> = models.iter().map(|model| model.id.as_str()).collect();

        assert_eq!(ids, vec!["claude-sonnet-4-5", "claude-haiku-4-5"]);
    }

    #[test]
    fn parses_gemini_model_list_response() {
        let models = parse_gemini_models_response(
            r#"{"models":[{"name":"models/gemini-2.5-pro"},{"name":"models/gemini-2.5-flash"}]}"#,
        )
        .unwrap();
        let ids: Vec<_> = models.iter().map(|model| model.id.as_str()).collect();

        assert_eq!(ids, vec!["gemini-2.5-pro", "gemini-2.5-flash"]);
    }
}

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::application::providers::common::CredentialField;
use crate::application::providers::translation::{LLMTranslationProvider, TranslationCoordinator};
use crate::application::providers::{
    ProviderPromptStrategy, DEFAULT_PROMPT_STRATEGY_ID, SMART_PROMPT_STRATEGY_ID,
};
use crate::infrastructure::http::HttpClient;
use crate::infrastructure::llm::{
    AnthropicLLMClient, GeminiLLMClient, LLMClient, LLMProtocol, OpenAILLMClient, ReasoningLevel,
};
use crate::infrastructure::storage::{ConfigFile, Keychain};
use crate::AppError;

/// Custom translation provider definition persisted in the user config.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CustomTranslationProviderDef {
    pub id: String,
    pub name: String,
    pub protocol: LLMProtocol,
    pub endpoint: String,
    pub model: String,
    pub reasoning_level: Option<ReasoningLevel>,
    #[serde(default = "default_prompt_strategy_id")]
    pub prompt_strategy_id: String,
    #[serde(default = "default_prompt_fallback_strategy_id")]
    pub prompt_fallback_strategy_id: String,
}

pub struct AddCustomTranslationProviderInput {
    pub name: String,
    pub protocol: String,
    pub endpoint: String,
    pub model: String,
    pub api_key: String,
    pub reasoning_level: Option<String>,
    pub prompt_strategy_id: Option<String>,
    pub prompt_fallback_strategy_id: Option<String>,
}

pub struct UpdateCustomTranslationProviderInput {
    pub name: String,
    pub protocol: String,
    pub endpoint: String,
    pub model: String,
    pub api_key: Option<String>,
    pub reasoning_level: Option<String>,
    pub prompt_strategy_id: Option<String>,
    pub prompt_fallback_strategy_id: Option<String>,
}

/// Information about a translation provider for display purposes.
#[derive(Serialize)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub is_configured: bool,
    pub requires_api_key: bool,
    pub is_active: bool,
    pub is_builtin: bool,
    pub protocol: Option<String>,
    pub endpoint: Option<String>,
    pub model: Option<String>,
    pub reasoning_level: Option<String>,
    pub prompt_strategy_id: Option<String>,
    pub prompt_fallback_strategy_id: Option<String>,
}

/// A credential value to save.
#[derive(Deserialize)]
pub struct CredentialValue {
    pub key: String,
    pub value: String,
}

pub struct CustomTranslationProviderView {
    pub id: String,
    pub name: String,
    pub protocol: String,
    pub endpoint: String,
    pub model: String,
    pub reasoning_level: Option<String>,
    pub prompt_strategy_id: String,
    pub prompt_fallback_strategy_id: String,
}

pub fn add_custom_translation_provider(
    input: AddCustomTranslationProviderInput,
    config_file: Arc<ConfigFile>,
    keychain: &Keychain,
    http_client: Arc<dyn HttpClient>,
    translation_coordinator: &TranslationCoordinator,
) -> crate::Result<CustomTranslationProviderView> {
    let id = create_custom_translation_provider_id();
    let def = build_custom_translation_provider_def(id.clone(), &input)?;

    keychain
        .save_provider_credential(&id, &input.api_key)
        .map_err(|e| AppError::Other(format!("Failed to save API key: {}", e)))?;

    let mut custom_defs = config_file
        .load::<Vec<CustomTranslationProviderDef>>("custom_translation_providers")
        .unwrap_or_default();
    custom_defs.push(def.clone());

    config_file
        .save("custom_translation_providers", &custom_defs)
        .map_err(|e| {
            let _ = keychain.delete_provider_credential(&id);
            AppError::Other(format!("Failed to save config: {}", e))
        })?;

    let provider =
        create_llm_translation_provider(&def, http_client, input.api_key, config_file.clone());
    translation_coordinator.register(provider).map_err(|e| {
        custom_defs.pop();
        let _ = config_file.save("custom_translation_providers", &custom_defs);
        let _ = keychain.delete_provider_credential(&id);
        AppError::Other(format!("Failed to register provider: {}", e))
    })?;

    translation_coordinator.activate(&id).map_err(|e| {
        let _ = translation_coordinator.unregister(&id);
        custom_defs.pop();
        let _ = config_file.save("custom_translation_providers", &custom_defs);
        let _ = keychain.delete_provider_credential(&id);
        AppError::Other(format!("Failed to activate provider: {}", e))
    })?;

    Ok(custom_translation_provider_view(&def))
}

fn build_custom_translation_provider_def(
    id: String,
    input: &AddCustomTranslationProviderInput,
) -> crate::Result<CustomTranslationProviderDef> {
    if input.api_key.trim().is_empty() {
        return Err(AppError::Other("API key cannot be empty".into()));
    }

    build_custom_translation_provider_def_from_parts(
        id,
        &input.name,
        &input.protocol,
        &input.endpoint,
        &input.model,
        input.reasoning_level.as_deref(),
        input.prompt_strategy_id.as_deref(),
        input.prompt_fallback_strategy_id.as_deref(),
    )
}

pub fn build_updated_custom_translation_provider_def(
    id: String,
    input: &UpdateCustomTranslationProviderInput,
) -> crate::Result<CustomTranslationProviderDef> {
    build_custom_translation_provider_def_from_parts(
        id,
        &input.name,
        &input.protocol,
        &input.endpoint,
        &input.model,
        input.reasoning_level.as_deref(),
        input.prompt_strategy_id.as_deref(),
        input.prompt_fallback_strategy_id.as_deref(),
    )
}

fn build_custom_translation_provider_def_from_parts(
    id: String,
    name: &str,
    protocol: &str,
    endpoint: &str,
    model: &str,
    reasoning_level: Option<&str>,
    prompt_strategy_id: Option<&str>,
    prompt_fallback_strategy_id: Option<&str>,
) -> crate::Result<CustomTranslationProviderDef> {
    if name.trim().is_empty() {
        return Err(AppError::Other("Name cannot be empty".into()));
    }
    if endpoint.trim().is_empty() {
        return Err(AppError::Other("Endpoint cannot be empty".into()));
    }
    if model.trim().is_empty() {
        return Err(AppError::Other("Model cannot be empty".into()));
    }

    Ok(CustomTranslationProviderDef {
        id,
        name: name.to_string(),
        protocol: parse_llm_protocol(protocol)?,
        endpoint: endpoint.to_string(),
        model: model.to_string(),
        reasoning_level: parse_reasoning_level(reasoning_level)?,
        prompt_strategy_id: parse_prompt_strategy_id(prompt_strategy_id, SMART_PROMPT_STRATEGY_ID)?,
        prompt_fallback_strategy_id: parse_prompt_strategy_id(
            prompt_fallback_strategy_id,
            DEFAULT_PROMPT_STRATEGY_ID,
        )?,
    })
}

fn parse_prompt_strategy_id(value: Option<&str>, default: &str) -> crate::Result<String> {
    match value.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) => Ok(value.to_string()),
        None => Ok(default.to_string()),
    }
}

fn parse_llm_protocol(protocol: &str) -> crate::Result<LLMProtocol> {
    match protocol {
        "openai" => Ok(LLMProtocol::OpenAI),
        "openai-responses" | "openai_responses" => Ok(LLMProtocol::OpenAIResponses),
        "anthropic" => Ok(LLMProtocol::Anthropic),
        "gemini" => Ok(LLMProtocol::Gemini),
        _ => Err(AppError::Other(format!("Invalid protocol: {}", protocol))),
    }
}

fn parse_reasoning_level(reasoning_level: Option<&str>) -> crate::Result<Option<ReasoningLevel>> {
    match reasoning_level {
        Some("minimal") => Ok(Some(ReasoningLevel::Minimal)),
        Some("low") => Ok(Some(ReasoningLevel::Low)),
        Some("medium") => Ok(Some(ReasoningLevel::Medium)),
        Some("high") => Ok(Some(ReasoningLevel::High)),
        Some("xhigh") => Ok(Some(ReasoningLevel::XHigh)),
        Some(other) => Err(AppError::Other(format!(
            "Invalid reasoning level: {}",
            other
        ))),
        None => Ok(None),
    }
}

fn create_custom_translation_provider_id() -> String {
    format!(
        "custom-llm-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    )
}

pub fn custom_translation_provider_view(
    def: &CustomTranslationProviderDef,
) -> CustomTranslationProviderView {
    CustomTranslationProviderView {
        id: def.id.clone(),
        name: def.name.clone(),
        protocol: def.protocol.as_str().to_string(),
        endpoint: def.endpoint.clone(),
        model: def.model.clone(),
        reasoning_level: def
            .reasoning_level
            .map(|level| format!("{:?}", level).to_lowercase()),
        prompt_strategy_id: def.prompt_strategy_id.clone(),
        prompt_fallback_strategy_id: def.prompt_fallback_strategy_id.clone(),
    }
}

pub fn create_llm_translation_provider(
    def: &CustomTranslationProviderDef,
    http_client: Arc<dyn HttpClient>,
    api_key: String,
    config_file: Arc<ConfigFile>,
) -> LLMTranslationProvider {
    let llm_client: Arc<dyn LLMClient> = match def.protocol {
        LLMProtocol::OpenAI => Arc::new(OpenAILLMClient::new_chat_completions(
            http_client,
            def.endpoint.clone(),
            def.model.clone(),
            api_key,
        )),
        LLMProtocol::OpenAIResponses => Arc::new(OpenAILLMClient::new_responses(
            http_client,
            def.endpoint.clone(),
            def.model.clone(),
            api_key,
        )),
        LLMProtocol::Anthropic => Arc::new(AnthropicLLMClient::new(
            http_client,
            def.endpoint.clone(),
            def.model.clone(),
            api_key,
        )),
        LLMProtocol::Gemini => Arc::new(GeminiLLMClient::new(
            http_client,
            def.endpoint.clone(),
            def.model.clone(),
            api_key,
        )),
    };

    LLMTranslationProvider::new(
        llm_client,
        def.id.clone(),
        def.name.clone(),
        def.reasoning_level,
        ProviderPromptStrategy {
            strategy_id: def.prompt_strategy_id.clone(),
            fallback_strategy_id: def.prompt_fallback_strategy_id.clone(),
        },
        config_file,
    )
}

fn default_prompt_strategy_id() -> String {
    SMART_PROMPT_STRATEGY_ID.to_string()
}

fn default_prompt_fallback_strategy_id() -> String {
    DEFAULT_PROMPT_STRATEGY_ID.to_string()
}

pub fn validate_required_credentials(
    fields: &[CredentialField],
    credentials: &HashMap<String, String>,
) -> crate::Result<()> {
    for field in fields {
        let value = credentials.get(&field.name).ok_or_else(|| {
            crate::AppError::Other(format!("Missing required field: {}", field.label))
        })?;

        if value.trim().is_empty() {
            return Err(crate::AppError::Other(format!(
                "Field cannot be empty: {}",
                field.label
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use crate::application::providers::common::{CredentialField, Provider};
    use crate::application::providers::translation::TranslationProvider;
    use crate::application::providers::{DEFAULT_PROMPT_STRATEGY_ID, SMART_PROMPT_STRATEGY_ID};
    use crate::domain::translation::TranslationRequest;
    use crate::infrastructure::http::{HttpClient, HttpResponse};
    use crate::infrastructure::llm::{LLMProtocol, ReasoningLevel};
    use crate::infrastructure::storage::ConfigFile;
    use anyhow::Result;
    use async_trait::async_trait;

    use super::{
        build_custom_translation_provider_def, build_updated_custom_translation_provider_def,
        create_llm_translation_provider, validate_required_credentials,
        AddCustomTranslationProviderInput, CustomTranslationProviderDef,
        UpdateCustomTranslationProviderInput,
    };

    struct MockHttpClient {
        response: HttpResponse,
        post_bodies: Arc<Mutex<Vec<String>>>,
    }

    #[async_trait]
    impl HttpClient for MockHttpClient {
        async fn post(
            &self,
            _url: &str,
            _headers: HashMap<String, String>,
            body: String,
        ) -> Result<HttpResponse> {
            self.post_bodies.lock().unwrap().push(body);
            Ok(self.response.clone())
        }

        async fn get(&self, _url: &str, _headers: HashMap<String, String>) -> Result<HttpResponse> {
            unimplemented!()
        }
    }

    fn custom_provider_def(protocol: LLMProtocol) -> CustomTranslationProviderDef {
        CustomTranslationProviderDef {
            id: "custom-test".to_string(),
            name: "Custom Test".to_string(),
            protocol,
            endpoint: "https://llm.example.test".to_string(),
            model: "test-model".to_string(),
            reasoning_level: None,
            prompt_strategy_id: SMART_PROMPT_STRATEGY_ID.to_string(),
            prompt_fallback_strategy_id: DEFAULT_PROMPT_STRATEGY_ID.to_string(),
        }
    }

    fn translation_request() -> TranslationRequest {
        TranslationRequest {
            text: "Hello".to_string(),
            source_lang: "en".to_string(),
            target_lang: "fr".to_string(),
        }
    }

    fn mock_http_client(response_body: &str) -> Arc<dyn HttpClient> {
        Arc::new(MockHttpClient {
            response: HttpResponse {
                status: 200,
                body: response_body.to_string(),
                headers: HashMap::new(),
            },
            post_bodies: Arc::new(Mutex::new(Vec::new())),
        })
    }

    fn valid_add_input() -> AddCustomTranslationProviderInput {
        AddCustomTranslationProviderInput {
            name: "Custom LLM".to_string(),
            protocol: "openai".to_string(),
            endpoint: "https://llm.example.test".to_string(),
            model: "gpt-test".to_string(),
            api_key: "test-key".to_string(),
            reasoning_level: Some("high".to_string()),
            prompt_strategy_id: None,
            prompt_fallback_strategy_id: None,
        }
    }

    #[test]
    fn custom_translation_provider_input_rejects_blank_name() {
        let input = AddCustomTranslationProviderInput {
            name: "  ".to_string(),
            ..valid_add_input()
        };

        let result = build_custom_translation_provider_def("custom-id".to_string(), &input);

        assert_eq!(result.unwrap_err().to_string(), "Name cannot be empty");
    }

    #[test]
    fn custom_translation_provider_input_rejects_blank_endpoint() {
        let input = AddCustomTranslationProviderInput {
            endpoint: "  ".to_string(),
            ..valid_add_input()
        };

        let result = build_custom_translation_provider_def("custom-id".to_string(), &input);

        assert_eq!(result.unwrap_err().to_string(), "Endpoint cannot be empty");
    }

    #[test]
    fn custom_translation_provider_input_rejects_blank_model() {
        let input = AddCustomTranslationProviderInput {
            model: "  ".to_string(),
            ..valid_add_input()
        };

        let result = build_custom_translation_provider_def("custom-id".to_string(), &input);

        assert_eq!(result.unwrap_err().to_string(), "Model cannot be empty");
    }

    #[test]
    fn custom_translation_provider_input_rejects_blank_api_key() {
        let input = AddCustomTranslationProviderInput {
            api_key: "  ".to_string(),
            ..valid_add_input()
        };

        let result = build_custom_translation_provider_def("custom-id".to_string(), &input);

        assert_eq!(result.unwrap_err().to_string(), "API key cannot be empty");
    }

    #[test]
    fn custom_translation_provider_input_rejects_invalid_protocol() {
        let input = AddCustomTranslationProviderInput {
            protocol: "ollama".to_string(),
            ..valid_add_input()
        };

        let result = build_custom_translation_provider_def("custom-id".to_string(), &input);

        assert_eq!(result.unwrap_err().to_string(), "Invalid protocol: ollama");
    }

    #[test]
    fn custom_translation_provider_input_accepts_openai_responses_protocol() {
        let input = AddCustomTranslationProviderInput {
            protocol: "openai-responses".to_string(),
            ..valid_add_input()
        };

        let def = build_custom_translation_provider_def("custom-id".to_string(), &input).unwrap();

        assert_eq!(def.protocol, LLMProtocol::OpenAIResponses);
    }

    #[test]
    fn custom_translation_provider_input_rejects_invalid_reasoning_level() {
        let input = AddCustomTranslationProviderInput {
            reasoning_level: Some("huge".to_string()),
            ..valid_add_input()
        };

        let result = build_custom_translation_provider_def("custom-id".to_string(), &input);

        assert_eq!(
            result.unwrap_err().to_string(),
            "Invalid reasoning level: huge"
        );
    }

    #[test]
    fn update_custom_translation_provider_def_preserves_id_without_requiring_api_key() {
        let input = UpdateCustomTranslationProviderInput {
            name: "gpt-5-mini".to_string(),
            protocol: "openai".to_string(),
            endpoint: "https://api.openai.com".to_string(),
            model: "gpt-5-mini".to_string(),
            api_key: None,
            reasoning_level: Some("minimal".to_string()),
            prompt_strategy_id: Some("technical".to_string()),
            prompt_fallback_strategy_id: Some(DEFAULT_PROMPT_STRATEGY_ID.to_string()),
        };

        let def = build_updated_custom_translation_provider_def("custom-gpt".to_string(), &input)
            .unwrap();

        assert_eq!(def.id, "custom-gpt");
        assert_eq!(def.name, "gpt-5-mini");
        assert_eq!(def.model, "gpt-5-mini");
        assert_eq!(def.reasoning_level, Some(ReasoningLevel::Minimal));
        assert_eq!(def.prompt_strategy_id, "technical");
        assert_eq!(def.prompt_fallback_strategy_id, DEFAULT_PROMPT_STRATEGY_ID);
    }

    #[test]
    fn custom_translation_provider_defaults_to_smart_prompt_strategy() {
        let def =
            build_custom_translation_provider_def("custom-id".to_string(), &valid_add_input())
                .unwrap();

        assert_eq!(def.prompt_strategy_id, SMART_PROMPT_STRATEGY_ID);
        assert_eq!(def.prompt_fallback_strategy_id, DEFAULT_PROMPT_STRATEGY_ID);
    }

    #[test]
    fn create_llm_translation_provider_uses_custom_id_and_name() {
        let def = custom_provider_def(LLMProtocol::OpenAI);

        let provider = create_llm_translation_provider(
            &def,
            mock_http_client(r#"{"choices":[]}"#),
            "key".into(),
            Arc::new(ConfigFile::new_temp()),
        );

        assert_eq!(provider.id(), "custom-test");
        assert_eq!(provider.name(), "Custom Test");
    }

    #[tokio::test]
    async fn create_llm_translation_provider_accepts_supported_protocols() {
        let cases = [
            (
                LLMProtocol::OpenAI,
                r#"{"choices":[{"message":{"content":"Bonjour"}}]}"#,
            ),
            (
                LLMProtocol::OpenAIResponses,
                r#"{"output":[{"type":"message","content":[{"type":"output_text","text":"Bonjour"}]}]}"#,
            ),
            (
                LLMProtocol::Anthropic,
                r#"{"content":[{"type":"text","text":"Bonjour"}]}"#,
            ),
            (
                LLMProtocol::Gemini,
                r#"{"candidates":[{"content":{"parts":[{"text":"Bonjour"}]}}]}"#,
            ),
        ];

        for (protocol, response_body) in cases {
            let def = custom_provider_def(protocol);
            let provider = create_llm_translation_provider(
                &def,
                mock_http_client(response_body),
                "key".into(),
                Arc::new(ConfigFile::new_temp()),
            );

            let result = provider.translate(&translation_request()).await.unwrap();

            assert_eq!(result.provider_id, "custom-test");
            assert_eq!(result.translated_text, "Bonjour");
        }
    }

    #[tokio::test]
    async fn create_llm_translation_provider_preserves_reasoning_level() {
        let post_bodies = Arc::new(Mutex::new(Vec::new()));
        let http_client = Arc::new(MockHttpClient {
            response: HttpResponse {
                status: 200,
                body: r#"{"choices":[{"message":{"content":"Bonjour"}}]}"#.to_string(),
                headers: HashMap::new(),
            },
            post_bodies: post_bodies.clone(),
        });
        let def = CustomTranslationProviderDef {
            model: "o3-mini".to_string(),
            reasoning_level: Some(ReasoningLevel::High),
            ..custom_provider_def(LLMProtocol::OpenAI)
        };

        let provider = create_llm_translation_provider(
            &def,
            http_client,
            "key".into(),
            Arc::new(ConfigFile::new_temp()),
        );
        provider.translate(&translation_request()).await.unwrap();

        let body: serde_json::Value =
            serde_json::from_str(&post_bodies.lock().unwrap()[0]).unwrap();
        assert_eq!(body["reasoning_effort"], "high");
    }

    #[test]
    fn validate_required_credentials_rejects_blank_required_field() {
        let fields = vec![CredentialField::new("api_key", "API Key", true)];
        let credentials = HashMap::from([("api_key".to_string(), "  ".to_string())]);

        let result = validate_required_credentials(&fields, &credentials);

        assert_eq!(
            result.unwrap_err().to_string(),
            "Field cannot be empty: API Key"
        );
    }
}

/// Owns the full custom LLM provider lifecycle: add/update/remove, credentials, listing, testing.
pub struct ProviderConfiguration {
    config_file: Arc<ConfigFile>,
    keychain: Arc<Keychain>,
    http_client: Arc<dyn HttpClient>,
    translation_coordinator: Arc<TranslationCoordinator>,
    llm_introspection: Arc<crate::application::providers::LlmIntrospection>,
}

impl ProviderConfiguration {
    pub fn new(
        config_file: Arc<ConfigFile>,
        keychain: Arc<Keychain>,
        http_client: Arc<dyn HttpClient>,
        translation_coordinator: Arc<TranslationCoordinator>,
        llm_introspection: Arc<crate::application::providers::LlmIntrospection>,
    ) -> Self {
        Self {
            config_file,
            keychain,
            http_client,
            translation_coordinator,
            llm_introspection,
        }
    }

    /// Add a new custom translation provider.
    pub fn add(
        &self,
        input: AddCustomTranslationProviderInput,
    ) -> crate::Result<CustomTranslationProviderView> {
        add_custom_translation_provider(
            input,
            self.config_file.clone(),
            &self.keychain,
            self.http_client.clone(),
            &self.translation_coordinator,
        )
    }

    /// Update an existing custom translation provider.
    pub fn update(
        &self,
        provider_id: String,
        input: UpdateCustomTranslationProviderInput,
    ) -> crate::Result<CustomTranslationProviderView> {
        // Load current state for rollback
        let mut custom_defs = self
            .config_file
            .load::<Vec<CustomTranslationProviderDef>>("custom_translation_providers")
            .unwrap_or_default();

        let index = custom_defs
            .iter()
            .position(|def| def.id == provider_id)
            .ok_or_else(|| AppError::Other(format!("Provider not found: {}", provider_id)))?;

        let old_def = custom_defs[index].clone();

        // Load old API key for rollback if we're changing it
        let old_api_key = if input.api_key.as_ref().map(|k| !k.trim().is_empty()).unwrap_or(false) {
            // We're changing the key, load old for rollback - must succeed
            Some(
                self.keychain
                    .load_provider_credential(&provider_id)
                    .map_err(|e| AppError::Other(format!("Cannot load existing key for rollback: {}", e)))?
            )
        } else {
            None
        };

        // Determine API key: use new if provided, otherwise load existing
        let api_key = if let Some(ref new_key) = input.api_key {
            let trimmed = new_key.trim();
            if trimmed.is_empty() {
                // If explicitly empty, load existing
                self.keychain
                    .load_provider_credential(&provider_id)
                    .map_err(|e| AppError::Other(format!("Failed to load existing API key: {}", e)))?
            } else {
                trimmed.to_string()
            }
        } else {
            self.keychain
                .load_provider_credential(&provider_id)
                .map_err(|e| AppError::Other(format!("Failed to load existing API key: {}", e)))?
        };

        // Build updated definition
        let updated_def = build_updated_custom_translation_provider_def(provider_id.clone(), &input)?;

        // Step 1: Save config first (no side effects if this fails)
        custom_defs[index] = updated_def.clone();
        self.config_file
            .save("custom_translation_providers", &custom_defs)
            .map_err(|e| {
                // Restore in-memory state
                custom_defs[index] = old_def.clone();
                AppError::Other(format!("Failed to save config: {}", e))
            })?;

        // Step 2: Save new API key if provided and non-empty
        if let Some(ref new_key) = input.api_key {
            if !new_key.trim().is_empty() {
                if let Err(e) = self.keychain.save_provider_credential(&provider_id, new_key.trim()) {
                    // Rollback config
                    custom_defs[index] = old_def.clone();
                    let _ = self.config_file.save("custom_translation_providers", &custom_defs);
                    return Err(AppError::Other(format!("Failed to save API key: {}", e)));
                }
            }
        }

        // Step 3: Create and register the updated provider
        let provider = create_llm_translation_provider(
            &updated_def,
            self.http_client.clone(),
            api_key,
            self.config_file.clone(),
        );

        // Step 4: Attempt to replace in coordinator - rollback on failure
        if let Err(e) = self.translation_coordinator.replace(provider) {
            // Rollback config
            custom_defs[index] = old_def;
            let _ = self
                .config_file
                .save("custom_translation_providers", &custom_defs);

            // Rollback keychain if we saved a new key
            if let Some(ref old_key) = old_api_key {
                let _ = self.keychain.save_provider_credential(&provider_id, old_key);
            }

            return Err(AppError::Other(format!(
                "Failed to update provider: {}",
                e
            )));
        }

        Ok(custom_translation_provider_view(&updated_def))
    }

    /// Remove a custom translation provider.
    pub fn remove(&self, provider_id: String) -> crate::Result<()> {
        // Reject builtin providers
        let builtin_ids = ["google-translate", "deeplx", "baidu-translate"];
        if builtin_ids.contains(&provider_id.as_str()) {
            return Err(AppError::Other("Cannot remove builtin provider".into()));
        }

        let mut custom_defs = self
            .config_file
            .load::<Vec<CustomTranslationProviderDef>>("custom_translation_providers")
            .unwrap_or_default();

        let index = custom_defs
            .iter()
            .position(|def| def.id == provider_id)
            .ok_or_else(|| AppError::Other(format!("Provider not found: {}", provider_id)))?;

        let removed_def = custom_defs[index].clone();

        // Step 0: Snapshot active providers list and order for rollback
        let active_providers = self.translation_coordinator.get_active();
        let active_ids: Vec<String> = active_providers
            .iter()
            .map(|p| p.read().id().to_string())
            .collect();
        let was_active = active_ids.contains(&provider_id);

        // Step 0.5: Collect credential field names BEFORE unregistering
        let credential_field_names: Vec<String> = if let Some(provider) = self.translation_coordinator.get(&provider_id) {
            provider.read().credential_fields().iter().map(|f| f.name.clone()).collect()
        } else {
            // Not registered, infer from custom LLM default
            vec!["api_key".to_string()]
        };

        // Snapshot credentials
        let snapshot = self
            .keychain
            .snapshot_provider_credentials(&provider_id, &credential_field_names);

        // Step 1: Remove from config first (lowest risk)
        custom_defs.remove(index);
        self.config_file
            .save("custom_translation_providers", &custom_defs)
            .map_err(|e| AppError::Other(format!("Failed to save config: {}", e)))?;

        // Step 2: Unregister from coordinator (track whether it was registered)
        let was_registered = self.translation_coordinator.get(&provider_id).is_some();

        if was_registered {
            if let Err(e) = self.translation_coordinator.unregister(&provider_id) {
                // Rollback config
                custom_defs.insert(index, removed_def.clone());
                let _ = self.config_file.save("custom_translation_providers", &custom_defs);
                return Err(AppError::Other(format!("Failed to unregister: {}", e)));
            }
        }

        // Step 3: Delete simple API key
        let delete_result = self.keychain.delete_provider_credential(&provider_id);
        if let Err(e) = &delete_result {
            let err_msg = format!("{}", e);
            // Only fail if key exists but deletion failed (idempotent delete)
            if !err_msg.contains("not found") && !err_msg.contains("Key not found") {
                // Complete rollback: config + credentials + provider registration + active state
                custom_defs.insert(index, removed_def.clone());
                let _ = self.config_file.save("custom_translation_providers", &custom_defs);

                // Restore credentials from snapshot
                let _ = self.keychain.restore_provider_credentials(&provider_id, &snapshot);

                // Re-register provider if it was registered before
                if was_registered {
                    let api_key = snapshot.api_key.and_then(|opt| opt).unwrap_or_default();
                    let provider = create_llm_translation_provider(
                        &removed_def,
                        self.http_client.clone(),
                        api_key,
                        self.config_file.clone(),
                    );
                    let _ = self.translation_coordinator.register(provider);

                    // Restore active state if it was active
                    if was_active {
                        let _ = self.translation_coordinator.activate(&provider_id);
                    }

                    // Restore active order by reordering
                    let _ = self.translation_coordinator.reorder_active(active_ids.clone());
                }

                return Err(AppError::Other(format!("Failed to delete credential: {}", e)));
            }
        }

        // Step 4: Delete structured credentials (using saved field names)
        if !credential_field_names.is_empty() {
            let _ = self.keychain.delete_provider_credentials(&provider_id, &credential_field_names);
        }

        Ok(())
    }

    /// List all translation providers with metadata.
    pub fn list_provider_infos(&self) -> Vec<ProviderInfo> {
        let all_providers = self.translation_coordinator.list_all();
        let active = self.translation_coordinator.get_active();
        let active_ids: Vec<_> = active.iter().map(|p| p.read().id().to_string()).collect();

        // Load custom provider definitions for extra metadata
        let custom_defs = self
            .config_file
            .load::<Vec<CustomTranslationProviderDef>>("custom_translation_providers")
            .unwrap_or_default();

        all_providers
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
            .collect()
    }

    /// Get the credential schema for a translation provider.
    pub fn credential_schema(&self, provider_id: String) -> crate::Result<Vec<CredentialField>> {
        let provider = self
            .translation_coordinator
            .get(&provider_id)
            .ok_or_else(|| AppError::Other(format!("Provider not found: {}", provider_id)))?;

        let fields = provider.read().credential_fields();
        Ok(fields)
    }

    /// Save credentials for a translation provider.
    pub fn save_credentials(
        &self,
        provider_id: String,
        credentials: Vec<CredentialValue>,
    ) -> crate::Result<()> {
        // Convert to HashMap for validation and processing
        let cred_map: HashMap<String, String> = credentials
            .into_iter()
            .map(|c| (c.key, c.value))
            .collect();

        // Get provider to validate schema
        let provider = self
            .translation_coordinator
            .get(&provider_id)
            .ok_or_else(|| AppError::Other(format!("Provider not found: {}", provider_id)))?;

        let expected_fields = provider.read().credential_fields();

        // Validate credentials before saving anything
        if provider_id == "deeplx" {
            validate_deeplx_credentials_map(&cred_map)?;
        } else {
            validate_required_credentials(&expected_fields, &cred_map)
                .map_err(|e| AppError::Other(e.to_string()))?;
        }

        // Validate that all provided credentials are non-blank
        for (key, value) in &cred_map {
            if value.trim().is_empty() {
                return Err(AppError::Other(format!(
                    "Credential '{}' cannot be blank",
                    key
                )));
            }
        }

        // Snapshot existing credentials for rollback
        let field_names: Vec<String> = expected_fields.iter().map(|f| f.name.clone()).collect();
        let snapshot = self
            .keychain
            .snapshot_provider_credentials(&provider_id, &field_names);

        // Save simple API key if applicable
        if cred_map.len() == 1 && cred_map.contains_key("api_key") {
            let api_key = cred_map.get("api_key").unwrap();
            if let Err(e) = self.keychain.save_provider_credential(&provider_id, api_key) {
                return Err(AppError::Other(format!("Failed to save credential: {}", e)));
            }
        }

        // Save structured credentials with transaction support
        if let Err(e) = self
            .keychain
            .save_provider_credentials_transactional(&provider_id, &cred_map, &snapshot)
        {
            // Rollback simple credential if we saved it
            if cred_map.len() == 1 && cred_map.contains_key("api_key") {
                if let Some(Some(ref old_key)) = snapshot.api_key {
                    let _ = self.keychain.save_provider_credential(&provider_id, old_key);
                } else if snapshot.api_key == Some(None) {
                    let _ = self.keychain.delete_provider_credential(&provider_id);
                }
            }
            return Err(AppError::Other(format!("Failed to save credentials: {}", e)));
        }

        // Reconfigure the provider with credentials
        if let Err(e) = self
            .translation_coordinator
            .reconfigure_provider(&provider_id, &cred_map)
        {
            // Complete rollback using snapshot
            let _ = self.keychain.restore_provider_credentials(&provider_id, &snapshot);
            return Err(AppError::Other(format!("Failed to reconfigure provider: {}", e)));
        }

        Ok(())
    }

    /// Test a custom translation provider by ID.
    pub async fn test_custom_provider(&self, provider_id: String) -> crate::Result<()> {
        let custom_defs = self
            .config_file
            .load::<Vec<CustomTranslationProviderDef>>("custom_translation_providers")
            .unwrap_or_default();

        let def = custom_defs
            .iter()
            .find(|def| def.id == provider_id)
            .ok_or_else(|| AppError::Other(format!("Provider not found: {}", provider_id)))?;

        let api_key = self
            .keychain
            .load_provider_credential(&provider_id)
            .map_err(|e| AppError::Other(format!("Failed to load provider credential: {}", e)))?;

        self.llm_introspection
            .test(def.protocol, &def.endpoint, &def.model, &api_key)
            .await
            .map_err(|e| AppError::Other(format!("Provider test failed: {}", e)))
    }
}

/// Validate DeepLX credentials based on mode.
fn validate_deeplx_credentials_map(credentials: &HashMap<String, String>) -> crate::Result<()> {
    let mode = credentials
        .get("mode")
        .map(String::as_str)
        .unwrap_or("deeplx");

    match mode {
        "deepl" => {
            // DeepL mode requires api_key
            if !credentials.contains_key("api_key") {
                return Err(AppError::Other("DeepL mode requires api_key".into()));
            }
            let api_key = credentials.get("api_key").unwrap();
            if api_key.trim().is_empty() {
                return Err(AppError::Other("DeepL api_key cannot be blank".into()));
            }
        }
        "deeplx" => {
            // DeepLX mode requires endpoint
            if !credentials.contains_key("endpoint") {
                return Err(AppError::Other("DeepLX mode requires endpoint".into()));
            }
            let endpoint = credentials.get("endpoint").unwrap();
            if endpoint.trim().is_empty() {
                return Err(AppError::Other("DeepLX endpoint cannot be blank".into()));
            }
        }
        other => {
            return Err(AppError::Other(format!("Invalid DeepLX mode: {}", other)));
        }
    }
    Ok(())
}

#[cfg(test)]
mod provider_configuration_tests {
    use super::*;
    use crate::application::providers::translation::TranslationCoordinator;
    use crate::infrastructure::http::{HttpClient, HttpResponse};
    use crate::infrastructure::storage::{ConfigFile, KeychainBackend};
    use anyhow::Result;
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    // Stub keychain for testing
    struct StubKeychainBackend {
        store: Mutex<HashMap<String, String>>,
    }

    impl StubKeychainBackend {
        fn new() -> Self {
            Self {
                store: Mutex::new(HashMap::new()),
            }
        }
    }

    impl KeychainBackend for StubKeychainBackend {
        fn save(&self, key: &str, value: &str) -> crate::Result<()> {
            self.store
                .lock()
                .unwrap()
                .insert(key.to_string(), value.to_string());
            Ok(())
        }

        fn load(&self, key: &str) -> crate::Result<String> {
            self.store
                .lock()
                .unwrap()
                .get(key)
                .cloned()
                .ok_or_else(|| crate::AppError::Other(format!("Key not found: {}", key)))
        }

        fn delete(&self, key: &str) -> crate::Result<()> {
            self.store.lock().unwrap().remove(key);
            Ok(())
        }
    }

    struct MockHttpClient;

    #[async_trait]
    impl HttpClient for MockHttpClient {
        async fn get(&self, _url: &str, _headers: HashMap<String, String>) -> Result<HttpResponse> {
            Ok(HttpResponse {
                status: 200,
                body: r#"{"data":[]}"#.to_string(),
                headers: HashMap::new(),
            })
        }

        async fn post(
            &self,
            _url: &str,
            _headers: HashMap<String, String>,
            _body: String,
        ) -> Result<HttpResponse> {
            Ok(HttpResponse {
                status: 200,
                body: r#"{"choices":[{"message":{"content":"OK"}}]}"#.to_string(),
                headers: HashMap::new(),
            })
        }
    }

    fn test_provider_configuration() -> ProviderConfiguration {
        let keychain = Arc::new(Keychain::with_backend(StubKeychainBackend::new()));
        let config_file = Arc::new(ConfigFile::new_temp());
        let http_client: Arc<dyn HttpClient> = Arc::new(MockHttpClient);
        let coordinator = Arc::new(TranslationCoordinator::new(config_file.clone()));
        let llm_introspection = Arc::new(crate::application::providers::LlmIntrospection::new(
            http_client.clone(),
        ));

        // Register builtin DeepL provider for testing
        use crate::application::providers::translation::DeepLProvider;
        let deeplx_provider = DeepLProvider::new(http_client.clone());
        let _ = coordinator.register(deeplx_provider);

        ProviderConfiguration::new(
            config_file,
            keychain,
            http_client,
            coordinator,
            llm_introspection,
        )
    }

    #[test]
    fn remove_rejects_builtin_providers() {
        let config = test_provider_configuration();

        let result = config.remove("google-translate".to_string());

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Cannot remove builtin provider"
        );
    }

    #[test]
    fn remove_returns_error_for_nonexistent_provider() {
        let config = test_provider_configuration();

        let result = config.remove("nonexistent-id".to_string());

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Provider not found"));
    }

    #[tokio::test]
    async fn test_custom_provider_returns_error_for_nonexistent() {
        let config = test_provider_configuration();

        let result = config.test_custom_provider("nonexistent-id".to_string()).await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Provider not found"));
    }

    #[test]
    fn save_credentials_validates_deeplx_mode_deepl_requires_api_key() {
        let config = test_provider_configuration();

        let credentials = vec![
            CredentialValue {
                key: "mode".to_string(),
                value: "deepl".to_string(),
            },
            CredentialValue {
                key: "endpoint".to_string(),
                value: "http://example.com".to_string(),
            },
        ];

        let result = config.save_credentials("deeplx".to_string(), credentials);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("DeepL mode requires api_key"));
    }

    #[test]
    fn save_credentials_validates_deeplx_mode_deeplx_requires_endpoint() {
        let config = test_provider_configuration();

        let credentials = vec![
            CredentialValue {
                key: "mode".to_string(),
                value: "deeplx".to_string(),
            },
            CredentialValue {
                key: "api_key".to_string(),
                value: "some-key".to_string(),
            },
        ];

        let result = config.save_credentials("deeplx".to_string(), credentials);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("DeepLX mode requires endpoint"));
    }

    #[test]
    fn save_credentials_rejects_blank_values() {
        let config = test_provider_configuration();

        let credentials = vec![
            CredentialValue {
                key: "mode".to_string(),
                value: "deeplx".to_string(),
            },
            CredentialValue {
                key: "endpoint".to_string(),
                value: "   ".to_string(), // blank
            },
        ];

        let result = config.save_credentials("deeplx".to_string(), credentials);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("cannot be blank"));
    }

    #[test]
    fn save_credentials_rejects_nonexistent_provider() {
        let config = test_provider_configuration();

        let credentials = vec![CredentialValue {
            key: "api_key".to_string(),
            value: "test-key".to_string(),
        }];

        let result = config.save_credentials("nonexistent-id".to_string(), credentials);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Provider not found"));
    }

    #[test]
    fn remove_succeeds_for_unregistered_provider() {
        let config = test_provider_configuration();

        // Add a custom provider to config but don't register it
        let def = CustomTranslationProviderDef {
            id: "test-custom-1".to_string(),
            name: "Test Custom".to_string(),
            protocol: LLMProtocol::OpenAI,
            endpoint: "https://api.example.com/v1".to_string(),
            model: "test-model".to_string(),
            reasoning_level: None,
            prompt_strategy_id: "default".to_string(),
            prompt_fallback_strategy_id: "default".to_string(),
        };

        let _ = config
            .config_file
            .save("custom_translation_providers", &vec![def]);

        // Save a credential
        let _ = config
            .keychain
            .save_provider_credential("test-custom-1", "test-key");

        // Remove should succeed even though provider is not registered
        let result = config.remove("test-custom-1".to_string());

        assert!(result.is_ok());

        // Verify config and keychain are cleaned up
        let remaining_defs = config
            .config_file
            .load::<Vec<CustomTranslationProviderDef>>("custom_translation_providers")
            .unwrap_or_default();
        assert!(remaining_defs.is_empty());

        let key_result = config.keychain.load_provider_credential("test-custom-1");
        assert!(key_result.is_err());
    }

    #[test]
    fn remove_succeeds_when_keychain_missing() {
        let config = test_provider_configuration();

        // Add a custom provider to config but don't save any credentials
        let def = CustomTranslationProviderDef {
            id: "test-custom-2".to_string(),
            name: "Test Custom 2".to_string(),
            protocol: LLMProtocol::OpenAI,
            endpoint: "https://api.example.com/v1".to_string(),
            model: "test-model".to_string(),
            reasoning_level: None,
            prompt_strategy_id: "default".to_string(),
            prompt_fallback_strategy_id: "default".to_string(),
        };

        let _ = config
            .config_file
            .save("custom_translation_providers", &vec![def]);

        // Remove should succeed even though keychain entry doesn't exist
        let result = config.remove("test-custom-2".to_string());

        assert!(result.is_ok());

        // Verify config is cleaned up
        let remaining_defs = config
            .config_file
            .load::<Vec<CustomTranslationProviderDef>>("custom_translation_providers")
            .unwrap_or_default();
        assert!(remaining_defs.is_empty());
    }
}

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::application::providers::common::CredentialField;
use crate::application::providers::translation::{LLMTranslationProvider, TranslationCoordinator};
use crate::application::providers::{
    LLMProtocol, LlmClientConfig, LlmRuntime, ProviderConfigStore, ProviderCredentialStore,
    ProviderPromptStrategy, ReasoningLevel, DEFAULT_PROMPT_STRATEGY_ID, SMART_PROMPT_STRATEGY_ID,
};
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

/// Add a new custom translation provider (internal helper, use ProviderConfiguration::add).
/// This function is not locked; callers must hold provider_state_lock.
pub(crate) fn add_custom_translation_provider(
    input: AddCustomTranslationProviderInput,
    config_store: Arc<dyn ProviderConfigStore>,
    credential_store: &dyn ProviderCredentialStore,
    llm_runtime: Arc<dyn LlmRuntime>,
    translation_coordinator: &TranslationCoordinator,
) -> crate::Result<CustomTranslationProviderView> {
    let id = create_custom_translation_provider_id();
    let def = build_custom_translation_provider_def(id.clone(), &input)?;

    credential_store
        .save_provider_credential(&id, &input.api_key)
        .map_err(|e| AppError::Other(format!("Failed to save API key: {}", e)))?;

    let mut custom_defs = config_store
        .load_custom_translation_providers()
        .unwrap_or_default();
    custom_defs.push(def.clone());

    config_store
        .save_custom_translation_providers(&custom_defs)
        .map_err(|e| {
            let rollback_err = credential_store
                .delete_provider_credential(&id)
                .err()
                .map(|re| format!(" keychain cleanup failed: {}", re));
            match rollback_err {
                Some(re) => AppError::Other(format!(
                    "Failed to save config: {}. Rollback also failed:{}",
                    e, re
                )),
                None => AppError::Other(format!("Failed to save config: {}", e)),
            }
        })?;

    let provider =
        create_llm_translation_provider(&def, llm_runtime, input.api_key, config_store.clone());
    translation_coordinator.register(provider).map_err(|e| {
        let mut rollback_errors: Vec<String> = Vec::new();
        custom_defs.pop();
        if let Err(re) = config_store.save_custom_translation_providers(&custom_defs) {
            rollback_errors.push(format!("config rollback: {}", re));
        }
        if let Err(re) = credential_store.delete_provider_credential(&id) {
            rollback_errors.push(format!("keychain rollback: {}", re));
        }
        if rollback_errors.is_empty() {
            AppError::Other(format!("Failed to register provider: {}", e))
        } else {
            AppError::Other(format!(
                "Failed to register provider: {}. Rollback also failed: {}",
                e,
                rollback_errors.join(", ")
            ))
        }
    })?;

    translation_coordinator.activate(&id).map_err(|e| {
        let mut rollback_errors: Vec<String> = Vec::new();
        if let Err(re) = translation_coordinator.unregister(&id) {
            rollback_errors.push(format!("unregister: {}", re));
        }
        custom_defs.pop();
        if let Err(re) = config_store.save_custom_translation_providers(&custom_defs) {
            rollback_errors.push(format!("config rollback: {}", re));
        }
        if let Err(re) = credential_store.delete_provider_credential(&id) {
            rollback_errors.push(format!("keychain rollback: {}", re));
        }
        if rollback_errors.is_empty() {
            AppError::Other(format!("Failed to activate provider: {}", e))
        } else {
            AppError::Other(format!(
                "Failed to activate provider: {}. Rollback also failed: {}",
                e,
                rollback_errors.join(", ")
            ))
        }
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
    llm_runtime: Arc<dyn LlmRuntime>,
    api_key: String,
    config_store: Arc<dyn ProviderConfigStore>,
) -> LLMTranslationProvider {
    let llm_client = llm_runtime.translation_client(LlmClientConfig {
        protocol: def.protocol,
        endpoint: def.endpoint.clone(),
        model: def.model.clone(),
        api_key,
    });

    LLMTranslationProvider::new(
        llm_client,
        def.id.clone(),
        def.name.clone(),
        def.reasoning_level,
        ProviderPromptStrategy {
            strategy_id: def.prompt_strategy_id.clone(),
            fallback_strategy_id: def.prompt_fallback_strategy_id.clone(),
        },
        config_store,
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

fn load_baidu_translation_credentials(
    credential_store: &dyn ProviderCredentialStore,
) -> Option<HashMap<String, String>> {
    credential_store
        .load_provider_credentials(
            "baidu-translate",
            &["app_id".to_string(), "secret_key".to_string()],
        )
        .ok()
        .or_else(|| {
            let app_id = credential_store
                .load_provider_credential("baidu_app_id")
                .ok()?;
            let secret_key = credential_store
                .load_provider_credential("baidu_secret_key")
                .ok()?;
            Some(HashMap::from([
                ("app_id".to_string(), app_id),
                ("secret_key".to_string(), secret_key),
            ]))
        })
}

fn load_deeplx_credentials(
    credential_store: &dyn ProviderCredentialStore,
) -> Option<HashMap<String, String>> {
    credential_store
        .load_provider_credentials(
            "deeplx",
            &[
                "mode".to_string(),
                "endpoint".to_string(),
                "api_key".to_string(),
            ],
        )
        .ok()
        .or_else(|| {
            let api_key = credential_store.load_provider_credential("deepl").ok()?;
            Some(HashMap::from([
                ("mode".to_string(), "deepl".to_string()),
                ("api_key".to_string(), api_key),
            ]))
        })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use crate::application::providers::common::{CredentialField, Provider};
    use crate::application::providers::translation::TranslationProvider;
    use crate::application::providers::{
        LLMClient, LLMProtocol, LLMRequest, LLMResponse, LlmClientConfig, LlmModelLister,
        LlmRuntime, ModelInfo, ReasoningLevel,
    };
    use crate::application::providers::{DEFAULT_PROMPT_STRATEGY_ID, SMART_PROMPT_STRATEGY_ID};
    use crate::domain::translation::TranslationRequest;
    use crate::infrastructure::storage::ConfigFile;
    use anyhow::Result;
    use async_trait::async_trait;

    use super::{
        build_custom_translation_provider_def, build_updated_custom_translation_provider_def,
        create_llm_translation_provider, validate_required_credentials,
        AddCustomTranslationProviderInput, CustomTranslationProviderDef,
        UpdateCustomTranslationProviderInput,
    };

    struct MockLlmClient {
        response: String,
        requests: Arc<Mutex<Vec<LLMRequest>>>,
    }

    #[async_trait]
    impl LLMClient for MockLlmClient {
        async fn generate(&self, request: &LLMRequest) -> Result<LLMResponse> {
            self.requests.lock().unwrap().push(request.clone());
            Ok(LLMResponse {
                text: self.response.clone(),
            })
        }
    }

    struct MockModelLister;

    #[async_trait]
    impl LlmModelLister for MockModelLister {
        async fn list_models(&self) -> Result<Vec<ModelInfo>> {
            Ok(Vec::new())
        }
    }

    struct MockLlmRuntime {
        response: String,
        requests: Arc<Mutex<Vec<LLMRequest>>>,
    }

    impl LlmRuntime for MockLlmRuntime {
        fn translation_client(&self, _config: LlmClientConfig) -> Arc<dyn LLMClient> {
            Arc::new(MockLlmClient {
                response: self.response.clone(),
                requests: self.requests.clone(),
            })
        }

        fn model_lister(&self, _config: LlmClientConfig) -> Arc<dyn LlmModelLister> {
            Arc::new(MockModelLister)
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

    fn mock_llm_runtime(response: &str) -> Arc<dyn LlmRuntime> {
        Arc::new(MockLlmRuntime {
            response: response.to_string(),
            requests: Arc::new(Mutex::new(Vec::new())),
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
            mock_llm_runtime("Bonjour"),
            "key".into(),
            Arc::new(ConfigFile::new_temp()),
        );

        assert_eq!(provider.id(), "custom-test");
        assert_eq!(provider.name(), "Custom Test");
    }

    #[tokio::test]
    async fn create_llm_translation_provider_accepts_supported_protocols() {
        let cases = [
            LLMProtocol::OpenAI,
            LLMProtocol::OpenAIResponses,
            LLMProtocol::Anthropic,
            LLMProtocol::Gemini,
        ];

        for protocol in cases {
            let def = custom_provider_def(protocol);
            let provider = create_llm_translation_provider(
                &def,
                mock_llm_runtime("Bonjour"),
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
        let requests = Arc::new(Mutex::new(Vec::new()));
        let llm_runtime = Arc::new(MockLlmRuntime {
            response: "Bonjour".to_string(),
            requests: requests.clone(),
        });
        let def = CustomTranslationProviderDef {
            model: "o3-mini".to_string(),
            reasoning_level: Some(ReasoningLevel::High),
            ..custom_provider_def(LLMProtocol::OpenAI)
        };

        let provider = create_llm_translation_provider(
            &def,
            llm_runtime,
            "key".into(),
            Arc::new(ConfigFile::new_temp()),
        );
        provider.translate(&translation_request()).await.unwrap();

        assert_eq!(
            requests.lock().unwrap()[0].options.reasoning,
            Some(ReasoningLevel::High)
        );
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
    config_store: Arc<dyn ProviderConfigStore>,
    credential_store: Arc<dyn ProviderCredentialStore>,
    llm_runtime: Arc<dyn LlmRuntime>,
    translation_coordinator: Arc<TranslationCoordinator>,
    llm_introspection: Arc<crate::application::providers::LlmIntrospection>,
    /// Serializes all provider state mutations (config + keychain + coordinator).
    /// Prevents concurrent add/update/remove from causing config/keychain/coordinator divergence.
    provider_state_lock: std::sync::Mutex<()>,
}

impl ProviderConfiguration {
    pub fn new(
        config_store: Arc<dyn ProviderConfigStore>,
        credential_store: Arc<dyn ProviderCredentialStore>,
        llm_runtime: Arc<dyn LlmRuntime>,
        translation_coordinator: Arc<TranslationCoordinator>,
        llm_introspection: Arc<crate::application::providers::LlmIntrospection>,
    ) -> Self {
        Self {
            config_store,
            credential_store,
            llm_runtime,
            translation_coordinator,
            llm_introspection,
            provider_state_lock: std::sync::Mutex::new(()),
        }
    }

    /// Acquires the provider state lock. Holds it to serialize provider mutations.
    fn lock_provider_state(&self) -> crate::Result<std::sync::MutexGuard<'_, ()>> {
        self.provider_state_lock
            .lock()
            .map_err(|e| crate::AppError::Other(format!("Provider state lock poisoned: {}", e)))
    }

    pub(crate) fn hydrate_credentials(&self) -> crate::Result<()> {
        let _guard = self.lock_provider_state()?;

        if let Some(credentials) = load_deeplx_credentials(self.credential_store.as_ref()) {
            if let Err(e) = self
                .translation_coordinator
                .reconfigure_provider("deeplx", &credentials)
            {
                log::warn!("Failed to hydrate DeepLX credentials: {}", e);
            }
        }

        if let Some(credentials) =
            load_baidu_translation_credentials(self.credential_store.as_ref())
        {
            if let Err(e) = self
                .translation_coordinator
                .reconfigure_provider("baidu-translate", &credentials)
            {
                log::warn!("Failed to hydrate Baidu Translate credentials: {}", e);
            }
        }

        self.register_custom_translation_providers();

        if let Err(e) = self.translation_coordinator.restore_from_config() {
            log::warn!(
                "Failed to restore active providers after credential hydration: {}",
                e
            );
        }

        Ok(())
    }

    fn register_custom_translation_providers(&self) {
        let Ok(custom_defs) = self.config_store.load_custom_translation_providers() else {
            return;
        };

        for def in custom_defs {
            let Ok(api_key) = self.credential_store.load_provider_credential(&def.id) else {
                continue;
            };

            let provider = create_llm_translation_provider(
                &def,
                self.llm_runtime.clone(),
                api_key,
                self.config_store.clone(),
            );
            if let Err(e) = self.translation_coordinator.register(provider) {
                log::warn!(
                    "Failed to register custom LLM provider '{}': {}",
                    def.name,
                    e
                );
            }
        }
    }

    /// Add a new custom translation provider.
    pub fn add(
        &self,
        input: AddCustomTranslationProviderInput,
    ) -> crate::Result<CustomTranslationProviderView> {
        let _guard = self.lock_provider_state()?;
        add_custom_translation_provider(
            input,
            self.config_store.clone(),
            self.credential_store.as_ref(),
            self.llm_runtime.clone(),
            &self.translation_coordinator,
        )
    }

    /// Update an existing custom translation provider.
    pub fn update(
        &self,
        provider_id: String,
        input: UpdateCustomTranslationProviderInput,
    ) -> crate::Result<CustomTranslationProviderView> {
        let _guard = self.lock_provider_state()?;

        // Load current state for rollback
        let mut custom_defs = self
            .config_store
            .load_custom_translation_providers()
            .unwrap_or_default();

        let index = custom_defs
            .iter()
            .position(|def| def.id == provider_id)
            .ok_or_else(|| AppError::Other(format!("Provider not found: {}", provider_id)))?;

        let old_def = custom_defs[index].clone();

        // Load old API key for rollback if we're changing it
        let old_api_key = if input
            .api_key
            .as_ref()
            .map(|k| !k.trim().is_empty())
            .unwrap_or(false)
        {
            // We're changing the key, load old for rollback - must succeed
            Some(
                self.credential_store
                    .load_provider_credential(&provider_id)
                    .map_err(|e| {
                        AppError::Other(format!("Cannot load existing key for rollback: {}", e))
                    })?,
            )
        } else {
            None
        };

        // Determine API key: use new if provided, otherwise load existing
        let api_key = if let Some(ref new_key) = input.api_key {
            let trimmed = new_key.trim();
            if trimmed.is_empty() {
                // If explicitly empty, load existing
                self.credential_store
                    .load_provider_credential(&provider_id)
                    .map_err(|e| {
                        AppError::Other(format!("Failed to load existing API key: {}", e))
                    })?
            } else {
                trimmed.to_string()
            }
        } else {
            self.credential_store
                .load_provider_credential(&provider_id)
                .map_err(|e| AppError::Other(format!("Failed to load existing API key: {}", e)))?
        };

        // Build updated definition
        let updated_def =
            build_updated_custom_translation_provider_def(provider_id.clone(), &input)?;

        // Step 1: Save config first (no side effects if this fails)
        custom_defs[index] = updated_def.clone();
        self.config_store
            .save_custom_translation_providers(&custom_defs)
            .map_err(|e| {
                // Restore in-memory state
                custom_defs[index] = old_def.clone();
                AppError::Other(format!("Failed to save config: {}", e))
            })?;

        // Step 2: Save new API key if provided and non-empty
        if let Some(ref new_key) = input.api_key {
            if !new_key.trim().is_empty() {
                if let Err(e) = self
                    .credential_store
                    .save_provider_credential(&provider_id, new_key.trim())
                {
                    // Rollback config
                    custom_defs[index] = old_def.clone();
                    let rollback_err = self
                        .config_store
                        .save_custom_translation_providers(&custom_defs)
                        .err()
                        .map(|re| format!(" config rollback failed: {}", re));
                    return Err(match rollback_err {
                        Some(re) => AppError::Other(format!(
                            "Failed to save API key: {}. Rollback also failed:{}",
                            e, re
                        )),
                        None => AppError::Other(format!("Failed to save API key: {}", e)),
                    });
                }
            }
        }

        // Step 3: Create and register the updated provider
        let provider = create_llm_translation_provider(
            &updated_def,
            self.llm_runtime.clone(),
            api_key,
            self.config_store.clone(),
        );

        // Step 4: Attempt to replace in coordinator - rollback on failure
        if let Err(e) = self.translation_coordinator.replace(provider) {
            let mut rollback_errors: Vec<String> = Vec::new();

            // Rollback config
            custom_defs[index] = old_def;
            if let Err(re) = self
                .config_store
                .save_custom_translation_providers(&custom_defs)
            {
                rollback_errors.push(format!("config rollback: {}", re));
            }

            // Rollback keychain if we saved a new key
            if let Some(ref old_key) = old_api_key {
                if let Err(re) = self
                    .credential_store
                    .save_provider_credential(&provider_id, old_key)
                {
                    rollback_errors.push(format!("keychain rollback: {}", re));
                }
            }

            if rollback_errors.is_empty() {
                return Err(AppError::Other(format!("Failed to update provider: {}", e)));
            } else {
                return Err(AppError::Other(format!(
                    "Failed to update provider: {}. Rollback also failed: {}",
                    e,
                    rollback_errors.join(", ")
                )));
            }
        }

        Ok(custom_translation_provider_view(&updated_def))
    }

    /// Remove a custom translation provider.
    pub fn remove(&self, provider_id: String) -> crate::Result<()> {
        let _guard = self.lock_provider_state()?;

        // Reject builtin providers
        let builtin_ids = ["google-translate", "deeplx", "baidu-translate"];
        if builtin_ids.contains(&provider_id.as_str()) {
            return Err(AppError::Other("Cannot remove builtin provider".into()));
        }

        let mut custom_defs = self
            .config_store
            .load_custom_translation_providers()
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
        let credential_field_names: Vec<String> =
            if let Some(provider) = self.translation_coordinator.get(&provider_id) {
                provider
                    .read()
                    .credential_fields()
                    .iter()
                    .map(|f| f.name.clone())
                    .collect()
            } else {
                // Not registered, infer from custom LLM default
                vec!["api_key".to_string()]
            };

        // Snapshot credentials
        let snapshot = self
            .credential_store
            .snapshot_provider_credentials(&provider_id, &credential_field_names)
            .map_err(|e| AppError::Other(format!("Failed to snapshot credentials: {}", e)))?;

        // Step 1: Remove from config first (lowest risk)
        custom_defs.remove(index);
        self.config_store
            .save_custom_translation_providers(&custom_defs)
            .map_err(|e| AppError::Other(format!("Failed to save config: {}", e)))?;

        // Step 2: Unregister from coordinator (track whether it was registered)
        let was_registered = self.translation_coordinator.get(&provider_id).is_some();

        if was_registered {
            if let Err(e) = self.translation_coordinator.unregister(&provider_id) {
                // Rollback config
                custom_defs.insert(index, removed_def.clone());
                let _ = self
                    .config_store
                    .save_custom_translation_providers(&custom_defs);
                return Err(AppError::Other(format!("Failed to unregister: {}", e)));
            }
        }

        // Helper to rollback everything and collect errors
        let mut rollback = || -> Vec<String> {
            let mut errors = Vec::new();

            custom_defs.insert(index, removed_def.clone());
            if let Err(e) = self
                .config_store
                .save_custom_translation_providers(&custom_defs)
            {
                errors.push(format!("config save: {}", e));
            }

            if let Err(e) = self
                .credential_store
                .restore_provider_credentials(&provider_id, &snapshot)
            {
                errors.push(format!("credential restore: {}", e));
            }

            if was_registered {
                let api_key = snapshot
                    .api_key
                    .clone()
                    .and_then(|opt| opt)
                    .unwrap_or_default();
                let provider = create_llm_translation_provider(
                    &removed_def,
                    self.llm_runtime.clone(),
                    api_key,
                    self.config_store.clone(),
                );
                if let Err(e) = self.translation_coordinator.register(provider) {
                    errors.push(format!("re-register: {}", e));
                }
                if was_active {
                    if let Err(e) = self.translation_coordinator.activate(&provider_id) {
                        errors.push(format!("re-activate: {}", e));
                    }
                }
                if let Err(e) = self
                    .translation_coordinator
                    .reorder_active(active_ids.clone())
                {
                    errors.push(format!("reorder active: {}", e));
                }
            }

            errors
        };

        // Step 3: Delete simple API key
        if let Err(e) = self
            .credential_store
            .delete_provider_credential(&provider_id)
        {
            // Only fail if key exists but deletion failed (idempotent delete)
            if !self.credential_store.is_not_found(&e) {
                let rollback_errors = rollback();
                if rollback_errors.is_empty() {
                    return Err(AppError::Other(format!(
                        "Failed to delete credential: {}",
                        e
                    )));
                } else {
                    return Err(AppError::Other(format!(
                        "Failed to delete credential: {}. Rollback also failed: {}",
                        e,
                        rollback_errors.join(", ")
                    )));
                }
            }
        }

        // Step 4: Delete structured credentials (using saved field names) with rollback on failure
        if !credential_field_names.is_empty() {
            if let Err(e) = self
                .credential_store
                .delete_provider_credentials(&provider_id, &credential_field_names)
            {
                let rollback_errors = rollback();
                if rollback_errors.is_empty() {
                    return Err(AppError::Other(format!(
                        "Failed to delete structured credentials: {}",
                        e
                    )));
                } else {
                    return Err(AppError::Other(format!(
                        "Failed to delete structured credentials: {}. Rollback also failed: {}",
                        e,
                        rollback_errors.join(", ")
                    )));
                }
            }
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
            .config_store
            .load_custom_translation_providers()
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
        let _guard = self.lock_provider_state()?;

        // Convert to HashMap for validation and processing
        let cred_map: HashMap<String, String> =
            credentials.into_iter().map(|c| (c.key, c.value)).collect();

        // Get provider to validate schema
        let provider = self
            .translation_coordinator
            .get(&provider_id)
            .ok_or_else(|| AppError::Other(format!("Provider not found: {}", provider_id)))?;

        let expected_fields = {
            let provider = provider.read();
            provider.validate_credentials(&cred_map)?;
            provider.credential_fields()
        };

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
            .credential_store
            .snapshot_provider_credentials(&provider_id, &field_names)
            .map_err(|e| AppError::Other(format!("Failed to snapshot credentials: {}", e)))?;

        // Save simple API key if applicable
        if cred_map.len() == 1 && cred_map.contains_key("api_key") {
            let api_key = cred_map.get("api_key").unwrap();
            if let Err(e) = self
                .credential_store
                .save_provider_credential(&provider_id, api_key)
            {
                return Err(AppError::Other(format!("Failed to save credential: {}", e)));
            }
        }

        // Save structured credentials with transaction support
        if let Err(e) = self
            .credential_store
            .save_provider_credentials_transactional(&provider_id, &cred_map, &snapshot)
        {
            // Rollback simple credential if we saved it
            if cred_map.len() == 1 && cred_map.contains_key("api_key") {
                if let Some(Some(ref old_key)) = snapshot.api_key {
                    let _ = self
                        .credential_store
                        .save_provider_credential(&provider_id, old_key);
                } else if snapshot.api_key == Some(None) {
                    let _ = self
                        .credential_store
                        .delete_provider_credential(&provider_id);
                }
            }
            return Err(AppError::Other(format!(
                "Failed to save credentials: {}",
                e
            )));
        }

        // Reconfigure the provider with credentials
        if let Err(e) = self
            .translation_coordinator
            .reconfigure_provider(&provider_id, &cred_map)
        {
            // Complete rollback using snapshot
            let _ = self
                .credential_store
                .restore_provider_credentials(&provider_id, &snapshot);
            return Err(AppError::Other(format!(
                "Failed to reconfigure provider: {}",
                e
            )));
        }

        Ok(())
    }

    /// Activate a translation provider by ID (with provider state lock).
    pub fn activate_provider(&self, id: String) -> crate::Result<()> {
        let _guard = self.lock_provider_state()?;
        self.translation_coordinator
            .activate(&id)
            .map_err(|e| AppError::Other(format!("Failed to activate provider: {}", e)))
    }

    /// Deactivate a translation provider by ID (with provider state lock).
    pub fn deactivate_provider(&self, id: String) -> crate::Result<()> {
        let _guard = self.lock_provider_state()?;
        self.translation_coordinator
            .deactivate(&id)
            .map_err(|e| AppError::Other(format!("Failed to deactivate provider: {}", e)))
    }

    /// Reorder active translation providers (with provider state lock).
    pub fn reorder_active_providers(&self, ordered_ids: Vec<String>) -> crate::Result<()> {
        let _guard = self.lock_provider_state()?;
        self.translation_coordinator
            .reorder_active(ordered_ids)
            .map_err(|e| AppError::Other(format!("Failed to reorder active providers: {}", e)))
    }

    /// Test a custom translation provider by ID.
    pub async fn test_custom_provider(&self, provider_id: String) -> crate::Result<()> {
        // Short-lived lock: clone out the values needed for async test
        let (protocol, endpoint, model, api_key) = {
            let _guard = self.lock_provider_state()?;

            let custom_defs = self
                .config_store
                .load_custom_translation_providers()
                .unwrap_or_default();

            let def = custom_defs
                .iter()
                .find(|def| def.id == provider_id)
                .ok_or_else(|| AppError::Other(format!("Provider not found: {}", provider_id)))?;

            let api_key = self
                .credential_store
                .load_provider_credential(&provider_id)
                .map_err(|e| {
                    AppError::Other(format!("Failed to load provider credential: {}", e))
                })?;

            (
                def.protocol,
                def.endpoint.clone(),
                def.model.clone(),
                api_key,
            )
        }; // Lock released here

        // Async test without holding the lock
        self.llm_introspection
            .test(protocol, &endpoint, &model, &api_key)
            .await
            .map_err(|e| AppError::Other(format!("Provider test failed: {}", e)))
    }
}

#[cfg(test)]
mod provider_configuration_tests {
    use super::*;
    use crate::application::providers::translation::TranslationCoordinator;
    use crate::application::providers::{HttpClient, HttpResponse, LlmRuntime};
    use crate::infrastructure::storage::{ConfigFile, Keychain, KeychainBackend};
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
                .ok_or_else(|| crate::AppError::Keychain(keyring::Error::NoEntry))
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

    fn llm_runtime_from_http(http_client: Arc<dyn HttpClient>) -> Arc<dyn LlmRuntime> {
        Arc::new(crate::infrastructure::llm::InfrastructureLlmRuntime::new(
            http_client,
        ))
    }

    fn test_provider_configuration() -> ProviderConfiguration {
        let keychain = Arc::new(Keychain::with_backend(StubKeychainBackend::new()));
        let config_file = Arc::new(ConfigFile::new_temp());
        let http_client: Arc<dyn HttpClient> = Arc::new(MockHttpClient);
        let llm_runtime = llm_runtime_from_http(http_client.clone());
        let coordinator = Arc::new(TranslationCoordinator::new(config_file.clone()));
        let llm_introspection = Arc::new(crate::application::providers::LlmIntrospection::new(
            llm_runtime.clone(),
        ));

        // Register builtin DeepL provider for testing
        use crate::application::providers::translation::DeepLProvider;
        let deeplx_provider = DeepLProvider::new(http_client.clone());
        let _ = coordinator.register(deeplx_provider);

        ProviderConfiguration::new(
            config_file,
            keychain,
            llm_runtime,
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

        let result = config
            .test_custom_provider("nonexistent-id".to_string())
            .await;

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
        assert!(result.unwrap_err().to_string().contains("cannot be blank"));
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
    fn hydrate_credentials_uses_provider_state_lock() {
        let config = test_provider_configuration();
        let poison_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = config.provider_state_lock.lock().unwrap();
            panic!("poison provider state lock");
        }));
        assert!(poison_result.is_err());

        let result = config.hydrate_credentials();

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Provider state lock poisoned"));
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
            .config_store
            .save_custom_translation_providers(&vec![def]);

        // Save a credential
        let _ = config
            .credential_store
            .save_provider_credential("test-custom-1", "test-key");

        // Remove should succeed even though provider is not registered
        let result = config.remove("test-custom-1".to_string());

        assert!(result.is_ok());

        // Verify config and keychain are cleaned up
        let remaining_defs = config
            .config_store
            .load_custom_translation_providers()
            .unwrap_or_default();
        assert!(remaining_defs.is_empty());

        let key_result = config
            .credential_store
            .load_provider_credential("test-custom-1");
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
            .config_store
            .save_custom_translation_providers(&vec![def]);

        // Remove should succeed even though keychain entry doesn't exist
        let result = config.remove("test-custom-2".to_string());

        assert!(result.is_ok());

        // Verify config is cleaned up
        let remaining_defs = config
            .config_store
            .load_custom_translation_providers()
            .unwrap_or_default();
        assert!(remaining_defs.is_empty());
    }

    // Failing keychain backend for testing rollback scenarios
    #[derive(Clone)]
    struct FailingKeychainBackend {
        store: Arc<Mutex<HashMap<String, String>>>,
        fail_on_delete_key: Arc<Mutex<Option<String>>>,
    }

    impl FailingKeychainBackend {
        fn new() -> Self {
            Self {
                store: Arc::new(Mutex::new(HashMap::new())),
                fail_on_delete_key: Arc::new(Mutex::new(None)),
            }
        }

        fn fail_on_delete(&self, key: String) {
            *self.fail_on_delete_key.lock().unwrap() = Some(key);
        }
    }

    impl KeychainBackend for FailingKeychainBackend {
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
                .ok_or_else(|| crate::AppError::Keychain(keyring::Error::NoEntry))
        }

        fn delete(&self, key: &str) -> crate::Result<()> {
            if let Some(ref fail_key) = *self.fail_on_delete_key.lock().unwrap() {
                if key == fail_key {
                    return Err(crate::AppError::Other(format!(
                        "Simulated delete failure for key: {}",
                        key
                    )));
                }
            }

            self.store.lock().unwrap().remove(key);
            Ok(())
        }
    }

    #[test]
    fn remove_rolls_back_all_state_on_structured_credential_delete_failure() {
        // Setup: create a provider configuration with failing keychain
        let failing_backend = FailingKeychainBackend::new();
        let backend_for_config = failing_backend.clone();

        let keychain = Arc::new(Keychain::with_backend(failing_backend));
        let config_file = Arc::new(ConfigFile::new_temp());
        let http_client: Arc<dyn HttpClient> = Arc::new(MockHttpClient);
        let llm_runtime = llm_runtime_from_http(http_client.clone());
        let coordinator = Arc::new(TranslationCoordinator::new(config_file.clone()));
        let llm_introspection = Arc::new(crate::application::providers::LlmIntrospection::new(
            llm_runtime.clone(),
        ));

        let config = ProviderConfiguration::new(
            config_file.clone(),
            keychain.clone(),
            llm_runtime.clone(),
            coordinator.clone(),
            llm_introspection,
        );

        // Add a custom provider with structured credentials
        let def = CustomTranslationProviderDef {
            id: "test-custom-3".to_string(),
            name: "Test Custom 3".to_string(),
            protocol: LLMProtocol::OpenAI,
            endpoint: "https://api.example.com/v1".to_string(),
            model: "test-model".to_string(),
            reasoning_level: None,
            prompt_strategy_id: "default".to_string(),
            prompt_fallback_strategy_id: "default".to_string(),
        };

        config_file
            .save("custom_translation_providers", &vec![def.clone()])
            .unwrap();

        // Register the provider
        let provider = create_llm_translation_provider(
            &def,
            llm_runtime,
            "test-key".to_string(),
            config_file.clone(),
        );
        coordinator.register(provider).unwrap();
        coordinator.activate("test-custom-3").unwrap();

        // Save credentials (simple + structured)
        keychain
            .save_provider_credential("test-custom-3", "simple-key")
            .unwrap();
        let mut structured = HashMap::new();
        structured.insert("api_key".to_string(), "struct-key".to_string());
        keychain
            .save_provider_credentials("test-custom-3", &structured)
            .unwrap();

        // Configure keychain to fail on structured credential deletion
        backend_for_config.fail_on_delete("provider:test-custom-3:credential:api_key".to_string());

        // Attempt to remove - should fail on structured credential deletion
        let result = config.remove("test-custom-3".to_string());

        // Should fail
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to delete structured credentials"));

        // Verify complete rollback:
        // 1. Config should still have the provider
        let remaining_defs = config_file
            .load::<Vec<CustomTranslationProviderDef>>("custom_translation_providers")
            .unwrap();
        assert_eq!(remaining_defs.len(), 1);
        assert_eq!(remaining_defs[0].id, "test-custom-3");

        // 2. Simple credential should still exist
        let simple_key = keychain.load_provider_credential("test-custom-3");
        assert!(simple_key.is_ok());
        assert_eq!(simple_key.unwrap(), "simple-key");

        // 3. Structured credential should still exist
        let struct_creds = keychain
            .load_provider_credentials("test-custom-3", &vec!["api_key".to_string()])
            .unwrap();
        assert_eq!(struct_creds.get("api_key").unwrap(), "struct-key");

        // 4. Provider should still be registered
        assert!(coordinator.get("test-custom-3").is_some());

        // 5. Provider should still be active
        let active = coordinator.get_active();
        let active_ids: Vec<String> = active.iter().map(|p| p.read().id().to_string()).collect();
        assert!(active_ids.contains(&"test-custom-3".to_string()));
    }

    #[test]
    fn concurrent_custom_adds_are_serialized() {
        use std::sync::{Arc, Barrier};
        use std::thread;

        let config = Arc::new(test_provider_configuration());

        // Barrier ensures all threads start simultaneously
        let barrier = Arc::new(Barrier::new(3));
        let mut handles = vec![];

        for i in 0..3 {
            let config_clone = config.clone();
            let barrier_clone = barrier.clone();

            handles.push(thread::spawn(move || {
                barrier_clone.wait(); // Sync start
                let input = AddCustomTranslationProviderInput {
                    name: format!("Concurrent Test {}", i),
                    protocol: "openai".to_string(),
                    endpoint: format!("https://api{}.example.com/v1", i),
                    model: "test-model".to_string(),
                    api_key: format!("key-{}", i),
                    reasoning_level: None,
                    prompt_strategy_id: None,
                    prompt_fallback_strategy_id: None,
                };
                config_clone.add(input)
            }));
        }

        // Collect results
        let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

        // All adds should succeed (serialized by lock)
        assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 3);

        // Verify invariant: config defs == keychain ids == coordinator ids
        let custom_defs = config
            .config_store
            .load_custom_translation_providers()
            .unwrap();
        let config_ids: std::collections::HashSet<_> =
            custom_defs.iter().map(|d| d.id.clone()).collect();

        // Check all added ids are in config
        for result in &results {
            if let Ok(view) = result {
                assert!(config_ids.contains(&view.id));
                // Keychain should have the credential
                assert!(config
                    .credential_store
                    .load_provider_credential(&view.id)
                    .is_ok());
                // Coordinator should have the provider
                assert!(config.translation_coordinator.get(&view.id).is_some());
            }
        }

        // No divergence: all 3 custom providers present everywhere
        assert_eq!(config_ids.len(), 3);
    }

    #[test]
    fn concurrent_remove_and_save_credentials_no_orphan() {
        use std::sync::{Arc, Barrier};
        use std::thread;

        let config = Arc::new(test_provider_configuration());

        // Pre-create a provider
        let add_input = AddCustomTranslationProviderInput {
            name: "Test Remove".to_string(),
            protocol: "openai".to_string(),
            endpoint: "https://api.example.com/v1".to_string(),
            model: "test-model".to_string(),
            api_key: "old-key".to_string(),
            reasoning_level: None,
            prompt_strategy_id: None,
            prompt_fallback_strategy_id: None,
        };
        let view = config.add(add_input).unwrap();
        let provider_id = view.id.clone();

        let barrier = Arc::new(Barrier::new(2));
        let config_clone1 = config.clone();
        let config_clone2 = config.clone();
        let barrier_clone1 = barrier.clone();
        let barrier_clone2 = barrier.clone();
        let provider_id_for_remove = provider_id.clone();
        let provider_id_for_save = provider_id.clone();

        // Thread 1: remove
        let h1 = thread::spawn(move || {
            barrier_clone1.wait();
            config_clone1.remove(provider_id_for_remove)
        });

        // Thread 2: save_credentials
        let h2 = thread::spawn(move || {
            barrier_clone2.wait();
            config_clone2.save_credentials(
                provider_id_for_save,
                vec![CredentialValue {
                    key: "api_key".to_string(),
                    value: "new-key".to_string(),
                }],
            )
        });

        let result1 = h1.join().unwrap();
        let result2 = h2.join().unwrap();

        // Exactly one succeeds (serialized by provider_state_lock).
        // The lock ensures mutual exclusion: remove and save_credentials cannot
        // interleave, so the loser sees "provider not found" or similar.
        assert!(result1.is_ok() ^ result2.is_ok());

        // Invariant: if remove succeeded, provider is gone from all stores
        if result1.is_ok() {
            let custom_defs = config
                .config_store
                .load_custom_translation_providers()
                .unwrap_or_default();
            assert!(!custom_defs.iter().any(|d| d.id == provider_id));
            assert!(config
                .credential_store
                .load_provider_credential(&provider_id)
                .is_err());
            assert!(config.translation_coordinator.get(&provider_id).is_none());
        }

        // If save_credentials succeeded, provider is still present with new credential
        if result2.is_ok() {
            let custom_defs = config
                .config_store
                .load_custom_translation_providers()
                .unwrap();
            assert!(custom_defs.iter().any(|d| d.id == provider_id));
            assert_eq!(
                config
                    .credential_store
                    .load_provider_credential(&provider_id)
                    .unwrap(),
                "new-key"
            );
            assert!(config.translation_coordinator.get(&provider_id).is_some());
        }
    }
}

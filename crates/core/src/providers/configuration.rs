use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::providers::common::CredentialField;
use crate::providers::translation::LLMTranslationProvider;
use crate::providers::translation::TranslationCoordinator;
use crate::providers::LLMProtocol;
use crate::providers::LlmClientConfig;
use crate::providers::LlmRuntime;
use crate::providers::ProviderConfigStore;
use crate::providers::ProviderCredentialStore;
use crate::providers::ProviderPromptStrategy;
use crate::providers::ReasoningLevel;
use crate::providers::DEFAULT_PROMPT_STRATEGY_ID;
use crate::providers::SMART_PROMPT_STRATEGY_ID;
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
#[derive(Clone, Serialize)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub is_configured: bool,
    pub requires_api_key: bool,
    pub is_active: bool,
    pub is_builtin: bool,
    pub is_local: bool,
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
                .map(|re| format!(" credential cleanup failed: {}", re));
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
            rollback_errors.push(format!("credential rollback: {}", re));
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
            rollback_errors.push(format!("credential rollback: {}", re));
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

    use crate::providers::common::CredentialField;
    use crate::providers::common::Provider;
    use crate::providers::translation::model::TranslationRequest;
    use crate::providers::translation::TranslationProvider;
    use crate::providers::LLMClient;
    use crate::providers::LLMProtocol;
    use crate::providers::LLMRequest;
    use crate::providers::LLMResponse;
    use crate::providers::LlmClientConfig;
    use crate::providers::LlmModelLister;
    use crate::providers::LlmRuntime;
    use crate::providers::ModelInfo;
    use crate::providers::ReasoningLevel;
    use crate::providers::DEFAULT_PROMPT_STRATEGY_ID;
    use crate::providers::SMART_PROMPT_STRATEGY_ID;
    use crate::test_support_config::MemoryConfigStore as SqliteConfigStore;
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
            Arc::new(SqliteConfigStore::new_temp()),
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
                Arc::new(SqliteConfigStore::new_temp()),
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
            Arc::new(SqliteConfigStore::new_temp()),
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
    llm_introspection: Arc<crate::providers::LlmIntrospection>,
    change_notifier: Option<Arc<dyn crate::providers::ProviderChangeNotifier>>,
    /// Serializes all provider state mutations (config + credentials + coordinator).
    /// Prevents concurrent add/update/remove from causing cross-store divergence.
    provider_state_lock: std::sync::Mutex<()>,
}

impl ProviderConfiguration {
    pub fn new(
        config_store: Arc<dyn ProviderConfigStore>,
        credential_store: Arc<dyn ProviderCredentialStore>,
        llm_runtime: Arc<dyn LlmRuntime>,
        translation_coordinator: Arc<TranslationCoordinator>,
        llm_introspection: Arc<crate::providers::LlmIntrospection>,
    ) -> Self {
        Self {
            config_store,
            credential_store,
            llm_runtime,
            translation_coordinator,
            llm_introspection,
            change_notifier: None,
            provider_state_lock: std::sync::Mutex::new(()),
        }
    }

    pub fn with_change_notifier(
        mut self,
        change_notifier: Arc<dyn crate::providers::ProviderChangeNotifier>,
    ) -> Self {
        self.change_notifier = Some(change_notifier);
        self
    }

    fn notify_changed(&self) {
        if let Some(notifier) = &self.change_notifier {
            notifier.providers_changed();
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
        let view = add_custom_translation_provider(
            input,
            self.config_store.clone(),
            self.credential_store.as_ref(),
            self.llm_runtime.clone(),
            &self.translation_coordinator,
        )?;
        self.notify_changed();
        Ok(view)
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

            // Roll back credentials if we saved a new key.
            if let Some(ref old_key) = old_api_key {
                if let Err(re) = self
                    .credential_store
                    .save_provider_credential(&provider_id, old_key)
                {
                    rollback_errors.push(format!("credential rollback: {}", re));
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

        let view = custom_translation_provider_view(&updated_def);
        self.notify_changed();
        Ok(view)
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

        // Step 3: Delete the known credential fields with rollback on failure.
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

        self.notify_changed();
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
                    is_local: provider.is_local(),
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

        // Persist the complete credential set atomically.
        if let Err(e) = self
            .credential_store
            .save_provider_credentials(&provider_id, &cred_map)
        {
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

        self.notify_changed();
        Ok(())
    }

    /// Activate a translation provider by ID (with provider state lock).
    pub fn activate_provider(&self, id: String) -> crate::Result<()> {
        let _guard = self.lock_provider_state()?;
        self.translation_coordinator
            .activate(&id)
            .map_err(|e| AppError::Other(format!("Failed to activate provider: {}", e)))?;
        self.notify_changed();
        Ok(())
    }

    /// Deactivate a translation provider by ID (with provider state lock).
    pub fn deactivate_provider(&self, id: String) -> crate::Result<()> {
        let _guard = self.lock_provider_state()?;
        self.translation_coordinator
            .deactivate(&id)
            .map_err(|e| AppError::Other(format!("Failed to deactivate provider: {}", e)))?;
        self.notify_changed();
        Ok(())
    }

    /// Reorder active translation providers (with provider state lock).
    pub fn reorder_active_providers(&self, ordered_ids: Vec<String>) -> crate::Result<()> {
        let _guard = self.lock_provider_state()?;
        self.translation_coordinator
            .reorder_active(ordered_ids)
            .map_err(|e| AppError::Other(format!("Failed to reorder active providers: {}", e)))?;
        self.notify_changed();
        Ok(())
    }

    /// Reorder inactive translation providers without changing their active state.
    pub fn reorder_inactive_providers(&self, ordered_ids: Vec<String>) -> crate::Result<()> {
        let _guard = self.lock_provider_state()?;
        let active_ids = self
            .translation_coordinator
            .get_active()
            .iter()
            .map(|provider| provider.read().id().to_string())
            .collect::<Vec<_>>();
        let mut expected_ids = self
            .translation_coordinator
            .list_all()
            .iter()
            .map(|provider| provider.read().id().to_string())
            .filter(|id| !active_ids.contains(id))
            .collect::<Vec<_>>();
        let mut received_ids = ordered_ids.clone();
        expected_ids.sort();
        received_ids.sort();
        if expected_ids != received_ids {
            return Err(AppError::Other(format!(
                "Inactive provider reorder validation failed: expected {:?}, got {:?}",
                expected_ids, received_ids
            )));
        }

        self.config_store
            .save_inactive_translation_providers(&ordered_ids)?;
        self.notify_changed();
        Ok(())
    }

    pub fn inactive_provider_order(&self) -> Vec<String> {
        self.config_store
            .load_inactive_translation_providers()
            .unwrap_or_default()
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

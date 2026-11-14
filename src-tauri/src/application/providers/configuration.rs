use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::application::providers::common::CredentialField;
use crate::application::providers::translation::{
    LLMTranslationProvider, TranslationCoordinator, TranslationProviderChange,
};
use crate::application::providers::{
    CredentialMutation, LLMProtocol, LlmClientConfig, LlmRuntime, ProviderConfigStore,
    ProviderCredentialStore, ProviderPromptStrategy, ReasoningLevel, TranslationProviderMutation,
    DEFAULT_PROMPT_STRATEGY_ID, SMART_PROMPT_STRATEGY_ID,
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
    use crate::infrastructure::storage::SqliteConfigStore;
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
    llm_introspection: Arc<crate::application::providers::LlmIntrospection>,
    change_notifier: Option<Arc<dyn crate::application::providers::ProviderChangeNotifier>>,
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
        llm_introspection: Arc<crate::application::providers::LlmIntrospection>,
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
        change_notifier: Arc<dyn crate::application::providers::ProviderChangeNotifier>,
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
    fn commit(
        &self,
        runtime: TranslationProviderChange,
        change: TranslationProviderMutation,
    ) -> crate::Result<()> {
        let active = change
            .active
            .clone()
            .unwrap_or_else(|| self.translation_coordinator.active_ids());
        self.translation_coordinator
            .commit_change(runtime, active, || {
                self.config_store
                    .commit_translation_provider_change(&change)
            })?;
        self.notify_changed();
        Ok(())
    }

    pub fn add(
        &self,
        input: AddCustomTranslationProviderInput,
    ) -> crate::Result<CustomTranslationProviderView> {
        let _guard = self.lock_provider_state()?;
        let id = create_custom_translation_provider_id();
        let definition = build_custom_translation_provider_def(id.clone(), &input)?;
        let provider = create_llm_translation_provider(
            &definition,
            self.llm_runtime.clone(),
            input.api_key.clone(),
            self.config_store.clone(),
        );
        let mut definitions = self
            .config_store
            .load_custom_translation_providers()
            .unwrap_or_default();
        definitions.push(definition.clone());
        let mut active = self.translation_coordinator.active_ids();
        active.push(id.clone());
        self.commit(
            TranslationProviderChange::Add(Arc::new(provider)),
            TranslationProviderMutation {
                definitions: Some(definitions),
                active: Some(active),
                credentials: Some(CredentialMutation::Save {
                    provider_id: id,
                    values: HashMap::from([("api_key".into(), input.api_key)]),
                }),
                ..Default::default()
            },
        )?;
        Ok(custom_translation_provider_view(&definition))
    }

    pub fn update(
        &self,
        provider_id: String,
        input: UpdateCustomTranslationProviderInput,
    ) -> crate::Result<CustomTranslationProviderView> {
        let _guard = self.lock_provider_state()?;
        let mut definitions = self
            .config_store
            .load_custom_translation_providers()
            .unwrap_or_default();
        let index = definitions
            .iter()
            .position(|definition| definition.id == provider_id)
            .ok_or_else(|| AppError::Other(format!("Provider not found: {provider_id}")))?;
        let definition =
            build_updated_custom_translation_provider_def(provider_id.clone(), &input)?;
        let api_key = match input
            .api_key
            .as_deref()
            .map(str::trim)
            .filter(|key| !key.is_empty())
        {
            Some(key) => key.to_string(),
            None => self
                .credential_store
                .load_provider_credential(&provider_id)?,
        };
        let provider = create_llm_translation_provider(
            &definition,
            self.llm_runtime.clone(),
            api_key.clone(),
            self.config_store.clone(),
        );
        definitions[index] = definition.clone();
        self.commit(
            TranslationProviderChange::Replace(Arc::new(provider)),
            TranslationProviderMutation {
                definitions: Some(definitions),
                credentials: Some(CredentialMutation::Save {
                    provider_id,
                    values: HashMap::from([("api_key".into(), api_key)]),
                }),
                ..Default::default()
            },
        )?;
        Ok(custom_translation_provider_view(&definition))
    }

    pub fn remove(&self, provider_id: String) -> crate::Result<()> {
        let _guard = self.lock_provider_state()?;
        if ["google-translate", "deeplx", "baidu-translate"].contains(&provider_id.as_str()) {
            return Err(AppError::Other("Cannot remove builtin provider".into()));
        }
        let mut definitions = self
            .config_store
            .load_custom_translation_providers()
            .unwrap_or_default();
        let index = definitions
            .iter()
            .position(|definition| definition.id == provider_id)
            .ok_or_else(|| AppError::Other(format!("Provider not found: {provider_id}")))?;
        definitions.remove(index);
        let mut active = self.translation_coordinator.active_ids();
        active.retain(|id| id != &provider_id);
        let mut inactive = self
            .config_store
            .load_inactive_translation_providers()
            .unwrap_or_default();
        inactive.retain(|id| id != &provider_id);
        self.commit(
            TranslationProviderChange::Remove(provider_id.clone()),
            TranslationProviderMutation {
                definitions: Some(definitions),
                active: Some(active),
                inactive: Some(inactive),
                credentials: Some(CredentialMutation::Delete {
                    provider_id,
                    fields: vec!["api_key".into()],
                }),
            },
        )
    }

    /// List all translation providers with metadata.
    pub fn list_provider_infos(&self) -> Vec<ProviderInfo> {
        let all_providers = self.translation_coordinator.list_all();
        let active = self.translation_coordinator.get_active();
        let active_ids: Vec<_> = active.iter().map(|p| p.id().to_string()).collect();

        // Load custom provider definitions for extra metadata
        let custom_defs = self
            .config_store
            .load_custom_translation_providers()
            .unwrap_or_default();

        all_providers
            .iter()
            .map(|p| {
                let provider = p;
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

        let fields = provider.credential_fields();
        Ok(fields)
    }

    /// Save credentials for a translation provider.
    pub fn save_credentials(
        &self,
        provider_id: String,
        credentials: Vec<CredentialValue>,
    ) -> crate::Result<()> {
        let _guard = self.lock_provider_state()?;
        let values: HashMap<String, String> = credentials
            .into_iter()
            .map(|credential| (credential.key, credential.value))
            .collect();
        let provider = self
            .translation_coordinator
            .get(&provider_id)
            .ok_or_else(|| AppError::Other(format!("Provider not found: {provider_id}")))?;
        provider.validate_credentials(&values)?;
        for (key, value) in &values {
            if value.trim().is_empty() {
                return Err(AppError::Other(format!(
                    "Credential '{key}' cannot be blank"
                )));
            }
        }
        let definition = self
            .config_store
            .load_custom_translation_providers()
            .unwrap_or_default()
            .into_iter()
            .find(|definition| definition.id == provider_id);
        let replacement = if let Some(definition) = definition {
            Arc::new(create_llm_translation_provider(
                &definition,
                self.llm_runtime.clone(),
                values["api_key"].clone(),
                self.config_store.clone(),
            )) as Arc<dyn crate::application::providers::TranslationProvider>
        } else {
            provider.with_credentials(&values)?
        };
        self.commit(
            TranslationProviderChange::Replace(replacement),
            TranslationProviderMutation {
                credentials: Some(CredentialMutation::Save {
                    provider_id,
                    values,
                }),
                ..Default::default()
            },
        )
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
            .map(|provider| provider.id().to_string())
            .collect::<Vec<_>>();
        let mut expected_ids = self
            .translation_coordinator
            .list_all()
            .iter()
            .map(|provider| provider.id().to_string())
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

#[cfg(test)]
mod provider_configuration_tests {
    use super::*;
    use crate::application::providers::translation::TranslationCoordinator;
    use crate::application::providers::{HttpClient, HttpResponse, LlmRuntime};
    use crate::infrastructure::storage::{Database, SqliteConfigStore, SqliteCredentialStore};
    use anyhow::Result;
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::Arc;

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
        test_provider_configuration_with_database().0
    }

    fn test_provider_configuration_with_database() -> (ProviderConfiguration, Arc<Database>) {
        let database = Arc::new(Database::in_memory().unwrap());
        let credential_store = Arc::new(SqliteCredentialStore::new(database.clone()));
        let config_store = Arc::new(SqliteConfigStore::new(database.clone()));
        let http_client: Arc<dyn HttpClient> = Arc::new(MockHttpClient);
        let llm_runtime = llm_runtime_from_http(http_client.clone());
        let coordinator = Arc::new(TranslationCoordinator::new(config_store.clone()));
        let llm_introspection = Arc::new(crate::application::providers::LlmIntrospection::new(
            llm_runtime.clone(),
        ));

        // Register builtin DeepL provider for testing
        use crate::application::providers::translation::DeepLProvider;
        let deeplx_provider = DeepLProvider::new(http_client.clone());
        let _ = coordinator.register(deeplx_provider);

        (
            ProviderConfiguration::new(
                config_store,
                credential_store,
                llm_runtime,
                coordinator,
                llm_introspection,
            ),
            database,
        )
    }

    #[test]
    fn custom_provider_credentials_are_not_written_to_settings_payload() {
        let (config, database) = test_provider_configuration_with_database();
        let secret = "sqlite-secret-sentinel";

        let view = config
            .add(AddCustomTranslationProviderInput {
                name: "Credential Boundary Test".to_string(),
                protocol: "openai".to_string(),
                endpoint: "https://api.example.com/v1".to_string(),
                model: "test-model".to_string(),
                api_key: secret.to_string(),
                reasoning_level: None,
                prompt_strategy_id: None,
                prompt_fallback_strategy_id: None,
            })
            .unwrap();

        let sqlite_payloads = database
            .with_connection(|connection| {
                Ok(connection.query_row(
                    "SELECT COALESCE(group_concat(payload_json, '\n'), '') FROM settings",
                    [],
                    |row| row.get::<_, String>(0),
                )?)
            })
            .unwrap();

        assert!(sqlite_payloads.contains("Credential Boundary Test"));
        assert!(!sqlite_payloads.contains(secret));
        assert_eq!(
            config
                .credential_store
                .load_provider_credential(&view.id)
                .unwrap(),
            secret
        );
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

        // Verify config and credentials are cleaned up.
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
    fn remove_succeeds_when_credentials_are_missing() {
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

        // Removal is idempotent when the credential entry does not exist.
        let result = config.remove("test-custom-2".to_string());

        assert!(result.is_ok());

        // Verify config is cleaned up
        let remaining_defs = config
            .config_store
            .load_custom_translation_providers()
            .unwrap_or_default();
        assert!(remaining_defs.is_empty());
    }

    #[test]
    fn failed_remove_keeps_configuration_credentials_runtime_and_order() {
        let (configuration, database) = test_provider_configuration_with_database();
        let view = configuration
            .add(AddCustomTranslationProviderInput {
                name: "Atomic delete".into(),
                protocol: "openai".into(),
                endpoint: "https://example.com".into(),
                model: "model".into(),
                api_key: "old-key".into(),
                reasoning_level: None,
                prompt_strategy_id: None,
                prompt_fallback_strategy_id: None,
            })
            .unwrap();
        let previous_provider = configuration.translation_coordinator.get(&view.id).unwrap();
        let previous_order = configuration.translation_coordinator.active_ids();
        let previous_inactive = vec![view.id.clone()];
        configuration
            .config_store
            .save_inactive_translation_providers(&previous_inactive)
            .unwrap();
        database.with_connection(|connection| {
            connection.execute_batch("CREATE TRIGGER fail_credential_delete BEFORE DELETE ON provider_credentials BEGIN SELECT RAISE(ABORT, 'injected delete failure'); END;")?;
            Ok(())
        }).unwrap();

        assert!(configuration.remove(view.id.clone()).is_err());
        assert_eq!(
            configuration
                .config_store
                .load_custom_translation_providers()
                .unwrap()[0]
                .id,
            view.id
        );
        assert_eq!(
            configuration
                .credential_store
                .load_provider_credential(&view.id)
                .unwrap(),
            "old-key"
        );
        assert_eq!(
            configuration.translation_coordinator.active_ids(),
            previous_order
        );
        assert_eq!(
            configuration
                .config_store
                .load_active_translation_providers()
                .unwrap(),
            previous_order
        );
        assert_eq!(
            configuration
                .config_store
                .load_inactive_translation_providers()
                .unwrap(),
            previous_inactive
        );
        assert!(Arc::ptr_eq(
            &previous_provider,
            &configuration.translation_coordinator.get(&view.id).unwrap()
        ));

        database
            .with_connection(|connection| {
                connection.execute_batch("DROP TRIGGER fail_credential_delete")?;
                Ok(())
            })
            .unwrap();
        configuration.remove(view.id.clone()).unwrap();
        assert!(configuration
            .config_store
            .load_active_translation_providers()
            .unwrap()
            .is_empty());
        assert!(configuration
            .config_store
            .load_inactive_translation_providers()
            .unwrap()
            .is_empty());
        assert!(configuration
            .config_store
            .load_custom_translation_providers()
            .unwrap()
            .is_empty());
        assert!(configuration
            .credential_store
            .load_provider_credential(&view.id)
            .is_err());
        assert!(configuration
            .translation_coordinator
            .get(&view.id)
            .is_none());
    }

    #[test]
    fn failed_add_and_update_do_not_publish_partial_provider_state() {
        let (configuration, database) = test_provider_configuration_with_database();
        database.with_connection(|connection| {
            connection.execute_batch("CREATE TRIGGER fail_credential_save BEFORE INSERT ON provider_credentials WHEN NEW.value = 'blocked' BEGIN SELECT RAISE(ABORT, 'injected save failure'); END;")?;
            Ok(())
        }).unwrap();
        let input = |key: &str| AddCustomTranslationProviderInput {
            name: "Atomic save".into(),
            protocol: "openai".into(),
            endpoint: "https://example.com".into(),
            model: "old-model".into(),
            api_key: key.into(),
            reasoning_level: None,
            prompt_strategy_id: None,
            prompt_fallback_strategy_id: None,
        };
        assert!(configuration.add(input("blocked")).is_err());
        assert!(configuration
            .config_store
            .load_custom_translation_providers()
            .is_err());
        assert!(configuration
            .translation_coordinator
            .get_active()
            .is_empty());
        let view = configuration.add(input("old-key")).unwrap();
        let previous_provider = configuration.translation_coordinator.get(&view.id).unwrap();
        assert!(configuration
            .update(
                view.id.clone(),
                UpdateCustomTranslationProviderInput {
                    name: "Changed".into(),
                    protocol: "openai".into(),
                    endpoint: "https://example.com".into(),
                    model: "new-model".into(),
                    api_key: Some("blocked".into()),
                    reasoning_level: None,
                    prompt_strategy_id: None,
                    prompt_fallback_strategy_id: None,
                }
            )
            .is_err());
        assert_eq!(
            configuration
                .config_store
                .load_custom_translation_providers()
                .unwrap()[0]
                .model,
            "old-model"
        );
        assert_eq!(
            configuration
                .credential_store
                .load_provider_credential(&view.id)
                .unwrap(),
            "old-key"
        );
        assert!(Arc::ptr_eq(
            &previous_provider,
            &configuration.translation_coordinator.get(&view.id).unwrap()
        ));
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

        // Verify the config, credential, and coordinator invariant.
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
                // The credential store should have the credential.
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

        assert!(result1.is_ok());
        if let Err(error) = result2 {
            assert!(error.to_string().contains("Provider not found"));
        }
        let definitions = config
            .config_store
            .load_custom_translation_providers()
            .unwrap();
        assert!(!definitions
            .iter()
            .any(|definition| definition.id == provider_id));
        assert!(config
            .credential_store
            .load_provider_credential(&provider_id)
            .is_err());
        assert!(config.translation_coordinator.get(&provider_id).is_none());
    }
}

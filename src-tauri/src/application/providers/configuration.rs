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

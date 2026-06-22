use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::application::providers::common::CredentialField;
use crate::application::providers::translation::{LLMTranslationProvider, TranslationCoordinator};
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
}

pub struct AddCustomTranslationProviderInput {
    pub name: String,
    pub protocol: String,
    pub endpoint: String,
    pub model: String,
    pub api_key: String,
    pub reasoning_level: Option<String>,
}

pub struct CustomTranslationProviderView {
    pub id: String,
    pub name: String,
    pub protocol: String,
    pub endpoint: String,
    pub model: String,
    pub reasoning_level: Option<String>,
}

pub fn add_custom_translation_provider(
    input: AddCustomTranslationProviderInput,
    config_file: &ConfigFile,
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

    let provider = create_llm_translation_provider(&def, http_client, input.api_key);
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
    if input.name.trim().is_empty() {
        return Err(AppError::Other("Name cannot be empty".into()));
    }
    if input.endpoint.trim().is_empty() {
        return Err(AppError::Other("Endpoint cannot be empty".into()));
    }
    if input.model.trim().is_empty() {
        return Err(AppError::Other("Model cannot be empty".into()));
    }
    if input.api_key.trim().is_empty() {
        return Err(AppError::Other("API key cannot be empty".into()));
    }

    Ok(CustomTranslationProviderDef {
        id,
        name: input.name.clone(),
        protocol: parse_llm_protocol(&input.protocol)?,
        endpoint: input.endpoint.clone(),
        model: input.model.clone(),
        reasoning_level: parse_reasoning_level(input.reasoning_level.as_deref())?,
    })
}

fn parse_llm_protocol(protocol: &str) -> crate::Result<LLMProtocol> {
    match protocol {
        "openai" => Ok(LLMProtocol::OpenAI),
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

fn custom_translation_provider_view(
    def: &CustomTranslationProviderDef,
) -> CustomTranslationProviderView {
    CustomTranslationProviderView {
        id: def.id.clone(),
        name: def.name.clone(),
        protocol: format!("{:?}", def.protocol).to_lowercase(),
        endpoint: def.endpoint.clone(),
        model: def.model.clone(),
        reasoning_level: def
            .reasoning_level
            .map(|level| format!("{:?}", level).to_lowercase()),
    }
}

pub fn create_llm_translation_provider(
    def: &CustomTranslationProviderDef,
    http_client: Arc<dyn HttpClient>,
    api_key: String,
) -> LLMTranslationProvider {
    let llm_client: Arc<dyn LLMClient> = match def.protocol {
        LLMProtocol::OpenAI => Arc::new(OpenAILLMClient::new(
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
    )
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
    use crate::domain::translation::TranslationRequest;
    use crate::infrastructure::http::{HttpClient, HttpResponse};
    use crate::infrastructure::llm::{LLMProtocol, ReasoningLevel};
    use anyhow::Result;
    use async_trait::async_trait;

    use super::{
        build_custom_translation_provider_def, create_llm_translation_provider,
        validate_required_credentials, AddCustomTranslationProviderInput,
        CustomTranslationProviderDef,
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
    fn create_llm_translation_provider_uses_custom_id_and_name() {
        let def = custom_provider_def(LLMProtocol::OpenAI);

        let provider = create_llm_translation_provider(
            &def,
            mock_http_client(r#"{"choices":[]}"#),
            "key".into(),
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

        let provider = create_llm_translation_provider(&def, http_client, "key".into());
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

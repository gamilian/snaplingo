use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::application::providers::common::CredentialField;
use crate::application::providers::translation::{LLMTranslationProvider, TranslationProvider};
use crate::infrastructure::http::HttpClient;
use crate::infrastructure::llm::{
    AnthropicLLMClient, GeminiLLMClient, LLMClient, LLMProtocol, OpenAILLMClient, ReasoningLevel,
};

/// Custom translation provider definition persisted in the user config.
#[derive(Clone, Serialize, Deserialize)]
pub struct CustomTranslationProviderDef {
    pub id: String,
    pub name: String,
    pub protocol: LLMProtocol,
    pub endpoint: String,
    pub model: String,
    pub reasoning_level: Option<ReasoningLevel>,
}

pub fn create_llm_translation_provider(
    def: &CustomTranslationProviderDef,
    http_client: Arc<dyn HttpClient>,
    api_key: String,
) -> Arc<dyn TranslationProvider> {
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

    Arc::new(LLMTranslationProvider::new(
        llm_client,
        def.id.clone(),
        def.name.clone(),
        def.reasoning_level,
    ))
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

    use crate::application::providers::common::CredentialField;

    use super::validate_required_credentials;

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

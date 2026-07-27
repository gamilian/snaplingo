use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::application::providers::common::CredentialField;
use crate::application::providers::configuration::{CredentialValue, ProviderInfo};
use crate::application::providers::ocr::{OcrCoordinator, OcrProviderConfiguration};
use crate::application::providers::translation::TranslationCoordinator;
use crate::application::providers::{
    AddCustomTranslationProviderInput, CustomTranslationProviderView, LLMProtocol,
    LlmIntrospection, ModelInfo, ProviderConfiguration, TranslationPromptConfiguration,
    TranslationPromptStrategyConfig, UpdateCustomTranslationProviderInput,
};
use crate::{AppError, Result};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OcrProviderInfo {
    pub id: String,
    pub name: String,
    pub is_configured: bool,
    pub requires_api_key: bool,
    pub is_active: bool,
}

pub struct ProviderModelListInput {
    pub endpoint: String,
    pub api_key: String,
}

pub struct ProviderConnectionTestInput {
    pub endpoint: String,
    pub api_key: String,
    pub model: String,
}

/// Owns provider administration policy while coordinators retain execution and active state.
pub struct ProviderAdministration {
    translation: Arc<TranslationCoordinator>,
    ocr: Arc<OcrCoordinator>,
    translation_configuration: Arc<ProviderConfiguration>,
    ocr_configuration: Arc<OcrProviderConfiguration>,
    llm_introspection: Arc<LlmIntrospection>,
    prompt_strategies: Arc<TranslationPromptConfiguration>,
}

impl ProviderAdministration {
    pub fn new(
        translation: Arc<TranslationCoordinator>,
        ocr: Arc<OcrCoordinator>,
        translation_configuration: Arc<ProviderConfiguration>,
        ocr_configuration: Arc<OcrProviderConfiguration>,
        llm_introspection: Arc<LlmIntrospection>,
        prompt_strategies: Arc<TranslationPromptConfiguration>,
    ) -> Self {
        Self {
            translation,
            ocr,
            translation_configuration,
            ocr_configuration,
            llm_introspection,
            prompt_strategies,
        }
    }

    pub(crate) fn hydrate_credentials(&self) {
        if let Err(error) = self.translation_configuration.hydrate_credentials() {
            log::warn!(
                "Failed to hydrate translation provider credentials: {}",
                error
            );
        }
        if let Err(error) = self.ocr_configuration.hydrate_credentials() {
            log::warn!("Failed to hydrate OCR provider credentials: {}", error);
        }
    }

    pub fn list_translation_providers(&self) -> Vec<ProviderInfo> {
        let providers = self.translation_configuration.list_provider_infos();
        let active_ids = self
            .translation
            .get_active()
            .iter()
            .map(|provider| provider.read().id().to_string())
            .collect::<Vec<_>>();
        order_provider_infos_for_display(providers, &active_ids)
    }

    pub fn activate_translation_provider(&self, provider_id: String) -> Result<()> {
        self.translation_configuration
            .activate_provider(provider_id)
    }

    pub fn deactivate_translation_provider(&self, provider_id: String) -> Result<()> {
        self.translation_configuration
            .deactivate_provider(provider_id)
    }

    pub fn reorder_active_translation_providers(&self, provider_ids: Vec<String>) -> Result<()> {
        self.translation_configuration
            .reorder_active_providers(provider_ids)
    }

    pub fn translation_credential_schema(
        &self,
        provider_id: String,
    ) -> Result<Vec<CredentialField>> {
        self.translation_configuration
            .credential_schema(provider_id)
    }

    pub fn configure_translation_provider_credentials(
        &self,
        provider_id: String,
        credentials: HashMap<String, String>,
    ) -> Result<()> {
        let credentials = credentials
            .into_iter()
            .map(|(key, value)| CredentialValue { key, value })
            .collect();
        self.translation_configuration
            .save_credentials(provider_id, credentials)
    }

    pub fn add_custom_translation_provider(
        &self,
        input: AddCustomTranslationProviderInput,
    ) -> Result<ProviderInfo> {
        self.translation_configuration
            .add(input)
            .map(|view| provider_info_from_custom_view(view, true))
    }

    pub fn update_custom_translation_provider(
        &self,
        provider_id: String,
        input: UpdateCustomTranslationProviderInput,
    ) -> Result<ProviderInfo> {
        let view = self
            .translation_configuration
            .update(provider_id.clone(), input)?;
        let is_active = self
            .translation
            .get_active()
            .iter()
            .any(|provider| provider.read().id() == provider_id);
        Ok(provider_info_from_custom_view(view, is_active))
    }

    pub fn list_translation_prompt_strategies(&self) -> TranslationPromptStrategyConfig {
        self.prompt_strategies.list()
    }

    pub fn save_translation_prompt_strategies(
        &self,
        config: TranslationPromptStrategyConfig,
    ) -> Result<TranslationPromptStrategyConfig> {
        self.prompt_strategies.save(config)
    }

    pub async fn list_openai_compatible_models(
        &self,
        input: ProviderModelListInput,
    ) -> Result<Vec<ModelInfo>> {
        self.list_models(LLMProtocol::OpenAI, input).await
    }

    pub async fn list_anthropic_models(
        &self,
        input: ProviderModelListInput,
    ) -> Result<Vec<ModelInfo>> {
        self.list_models(LLMProtocol::Anthropic, input).await
    }

    pub async fn list_gemini_models(
        &self,
        input: ProviderModelListInput,
    ) -> Result<Vec<ModelInfo>> {
        self.list_models(LLMProtocol::Gemini, input).await
    }

    pub async fn test_openai_compatible_provider(
        &self,
        input: ProviderConnectionTestInput,
    ) -> Result<()> {
        self.test_provider(LLMProtocol::OpenAI, input).await
    }

    pub async fn test_openai_responses_provider(
        &self,
        input: ProviderConnectionTestInput,
    ) -> Result<()> {
        self.test_provider(LLMProtocol::OpenAIResponses, input)
            .await
    }

    pub async fn test_anthropic_provider(&self, input: ProviderConnectionTestInput) -> Result<()> {
        self.test_provider(LLMProtocol::Anthropic, input).await
    }

    pub async fn test_gemini_provider(&self, input: ProviderConnectionTestInput) -> Result<()> {
        self.test_provider(LLMProtocol::Gemini, input).await
    }

    pub async fn test_custom_translation_provider(&self, provider_id: String) -> Result<()> {
        self.translation_configuration
            .test_custom_provider(provider_id)
            .await
    }

    pub fn remove_custom_translation_provider(&self, provider_id: String) -> Result<()> {
        self.translation_configuration.remove(provider_id)
    }

    pub fn list_ocr_providers(&self) -> Vec<OcrProviderInfo> {
        let active_id = self
            .ocr
            .get_active()
            .map(|provider| provider.read().id().to_string());

        self.ocr
            .list_all()
            .iter()
            .map(|provider| {
                let provider = provider.read();
                let id = provider.id().to_string();
                OcrProviderInfo {
                    id: id.clone(),
                    name: provider.name().to_string(),
                    is_configured: provider.is_configured(),
                    requires_api_key: provider.requires_api_key(),
                    is_active: active_id.as_ref() == Some(&id),
                }
            })
            .collect()
    }

    pub fn activate_ocr_provider(&self, provider_id: String) -> Result<()> {
        self.ocr.activate(&provider_id)
    }

    pub fn configure_ocr_provider(
        &self,
        provider_id: String,
        api_key: String,
        secret_key: Option<String>,
    ) -> Result<()> {
        let mut credentials = HashMap::from([("api_key".to_string(), api_key)]);
        if let Some(secret_key) = secret_key {
            credentials.insert("secret_key".to_string(), secret_key);
        }
        self.ocr_configuration
            .save_credentials(&provider_id, &credentials)
    }

    pub fn ocr_credential_schema(&self, provider_id: String) -> Result<Vec<CredentialField>> {
        self.ocr_configuration.credential_schema(&provider_id)
    }

    pub fn configure_ocr_provider_credentials(
        &self,
        provider_id: String,
        credentials: HashMap<String, String>,
    ) -> Result<()> {
        self.ocr_configuration
            .save_credentials(&provider_id, &credentials)
    }

    async fn list_models(
        &self,
        protocol: LLMProtocol,
        input: ProviderModelListInput,
    ) -> Result<Vec<ModelInfo>> {
        let endpoint = validate_non_blank(&input.endpoint, "API address")?;
        let api_key = validate_non_blank(&input.api_key, "API key")?;
        self.llm_introspection
            .list_models(protocol, endpoint, api_key)
            .await
            .map_err(|error| AppError::Other(format!("Failed to fetch model list: {}", error)))
    }

    async fn test_provider(
        &self,
        protocol: LLMProtocol,
        input: ProviderConnectionTestInput,
    ) -> Result<()> {
        let endpoint = validate_non_blank(&input.endpoint, "API address")?;
        let api_key = validate_non_blank(&input.api_key, "API key")?;
        let model = validate_non_blank(&input.model, "Model")?;
        self.llm_introspection
            .test(protocol, endpoint, model, api_key)
            .await
            .map_err(|error| AppError::Other(format!("Provider test failed: {}", error)))
    }
}

fn provider_info_from_custom_view(
    view: CustomTranslationProviderView,
    is_active: bool,
) -> ProviderInfo {
    ProviderInfo {
        id: view.id,
        name: view.name,
        is_configured: true,
        requires_api_key: true,
        is_active,
        is_builtin: false,
        protocol: Some(view.protocol),
        endpoint: Some(view.endpoint),
        model: Some(view.model),
        reasoning_level: view.reasoning_level,
        prompt_strategy_id: Some(view.prompt_strategy_id),
        prompt_fallback_strategy_id: Some(view.prompt_fallback_strategy_id),
    }
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

    let mut inactive = by_id.into_values().collect::<Vec<_>>();
    inactive.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.id.cmp(&right.id))
    });
    ordered.extend(inactive);
    ordered
}

fn validate_non_blank<'a>(value: &'a str, label: &str) -> Result<&'a str> {
    let value = value.trim();
    if value.is_empty() {
        Err(AppError::Other(format!("{} cannot be empty", label)))
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
        let ordered_ids = ordered
            .iter()
            .map(|provider| provider.id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            ordered_ids,
            vec!["custom-gpt", "google-translate", "deeplx"]
        );
    }

    #[test]
    fn introspection_validation_trims_values_and_rejects_blanks() {
        assert_eq!(validate_non_blank(" value ", "Field").unwrap(), "value");
        assert_eq!(
            validate_non_blank("  ", "API key").unwrap_err().to_string(),
            "API key cannot be empty"
        );
    }
}

use crate::application::providers::{
    CustomTranslationProviderDef, TranslationPromptStrategyConfig,
};
use crate::Result;
use std::collections::HashMap;

pub enum CredentialMutation {
    Save {
        provider_id: String,
        values: HashMap<String, String>,
    },
    Delete {
        provider_id: String,
        fields: Vec<String>,
    },
}

#[derive(Default)]
pub struct TranslationProviderMutation {
    pub definitions: Option<Vec<CustomTranslationProviderDef>>,
    pub active: Option<Vec<String>>,
    pub inactive: Option<Vec<String>>,
    pub credentials: Option<CredentialMutation>,
}

pub trait ProviderConfigStore: Send + Sync {
    /// Commits all durable parts of one provider change atomically.
    fn commit_translation_provider_change(
        &self,
        change: &TranslationProviderMutation,
    ) -> Result<()>;
    fn load_custom_translation_providers(&self) -> Result<Vec<CustomTranslationProviderDef>>;
    fn save_custom_translation_providers(
        &self,
        providers: &[CustomTranslationProviderDef],
    ) -> Result<()>;

    fn load_active_translation_providers(&self) -> Result<Vec<String>>;
    fn save_active_translation_providers(&self, provider_ids: &[String]) -> Result<()>;

    fn load_inactive_translation_providers(&self) -> Result<Vec<String>>;
    fn save_inactive_translation_providers(&self, provider_ids: &[String]) -> Result<()>;

    fn load_active_ocr_provider(&self) -> Result<String>;
    fn save_active_ocr_provider(&self, provider_id: &str) -> Result<()>;

    fn load_translation_prompt_strategies(&self) -> Result<TranslationPromptStrategyConfig>;
    fn save_translation_prompt_strategies(
        &self,
        config: &TranslationPromptStrategyConfig,
    ) -> Result<()>;
}

use crate::application::providers::{
    CustomTranslationProviderDef, TranslationPromptStrategyConfig,
};
use crate::Result;

pub trait ProviderConfigStore: Send + Sync {
    fn load_custom_translation_providers(&self) -> Result<Vec<CustomTranslationProviderDef>>;
    fn save_custom_translation_providers(
        &self,
        providers: &[CustomTranslationProviderDef],
    ) -> Result<()>;

    fn load_active_translation_providers(&self) -> Result<Vec<String>>;
    fn save_active_translation_providers(&self, provider_ids: &[String]) -> Result<()>;

    fn load_active_ocr_provider(&self) -> Result<String>;
    fn save_active_ocr_provider(&self, provider_id: &str) -> Result<()>;

    fn load_translation_prompt_strategies(&self) -> Result<TranslationPromptStrategyConfig>;
    fn save_translation_prompt_strategies(
        &self,
        config: &TranslationPromptStrategyConfig,
    ) -> Result<()>;
}

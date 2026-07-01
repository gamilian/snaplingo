pub mod common;
pub mod configuration;
pub mod ocr;
pub mod translation;
pub mod translation_prompt;

pub use common::Provider;
pub use configuration::{
    add_custom_translation_provider, build_updated_custom_translation_provider_def,
    create_llm_translation_provider, custom_translation_provider_view,
    validate_required_credentials, AddCustomTranslationProviderInput, CustomTranslationProviderDef,
    CustomTranslationProviderView, UpdateCustomTranslationProviderInput,
};
pub use ocr::OcrProvider;
pub use translation::TranslationProvider;
pub use translation_prompt::{
    default_prompt_strategy_config, merge_prompt_strategy_config, render_translation_system_prompt,
    sanitize_prompt_strategy_config, validate_prompt_strategy_config, ProviderPromptStrategy,
    TranslationPromptStrategy, TranslationPromptStrategyConfig, DEFAULT_PROMPT_STRATEGY_ID,
    SMART_PROMPT_STRATEGY_ID,
};

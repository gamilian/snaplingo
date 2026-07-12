pub mod common;
mod config_store;
pub mod configuration;
mod credential_store;
mod event_sink;
mod http_transport;
pub mod llm_introspection;
mod llm_runtime;
pub mod ocr;
pub mod translation;
pub mod translation_prompt;

pub use common::Provider;
pub(crate) use config_store::ProviderConfigStore;
pub use configuration::{
    build_updated_custom_translation_provider_def, create_llm_translation_provider,
    custom_translation_provider_view, validate_required_credentials,
    AddCustomTranslationProviderInput, CustomTranslationProviderDef, CustomTranslationProviderView,
    ProviderConfiguration, UpdateCustomTranslationProviderInput,
};
pub(crate) use credential_store::{CredentialSnapshot, ProviderCredentialStore};
pub use event_sink::ProviderEventSink;
pub use http_transport::{HttpClient, HttpResponse};
pub use llm_introspection::LlmIntrospection;
pub use llm_runtime::{
    LLMClient, LLMOptions, LLMProtocol, LLMRequest, LLMResponse, LlmClientConfig, LlmModelLister,
    LlmRuntime, ModelInfo, ReasoningLevel,
};
pub use ocr::OcrProvider;
pub use translation::TranslationProvider;
pub use translation_prompt::{
    default_prompt_strategy_config, merge_prompt_strategy_config, render_translation_system_prompt,
    sanitize_prompt_strategy_config, validate_prompt_strategy_config, ProviderPromptStrategy,
    TranslationPromptConfiguration, TranslationPromptStrategy, TranslationPromptStrategyConfig,
    DEFAULT_PROMPT_STRATEGY_ID, SMART_PROMPT_STRATEGY_ID,
};

mod coordinator;
mod impls;
mod trait_def;

#[cfg(test)]
mod coordinator_test;

pub use coordinator::TranslationCoordinator;
pub(crate) use coordinator::TranslationProviderChange;
pub use impls::{
    BaiduTranslateProvider, DeepLProvider, GoogleTranslateProvider, LLMTranslationProvider,
};
pub use trait_def::TranslationProvider;

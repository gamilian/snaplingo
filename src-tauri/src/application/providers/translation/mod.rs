mod trait_def;
mod coordinator;
mod impls;

#[cfg(test)]
mod coordinator_test;

pub use trait_def::TranslationProvider;
pub use coordinator::TranslationCoordinator;
pub use impls::{GoogleTranslateProvider, DeepLProvider, BaiduTranslateProvider, LLMTranslationProvider};

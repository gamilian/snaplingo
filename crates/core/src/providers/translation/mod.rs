mod coordinator;
mod impls;
pub mod model;
mod trait_def;

#[cfg(test)]
mod coordinator_test;

pub use coordinator::TranslationCoordinator;
pub use impls::{
    BaiduTranslateProvider, DeepLProvider, GoogleTranslateProvider, LLMTranslationProvider,
};
pub use trait_def::TranslationProvider;

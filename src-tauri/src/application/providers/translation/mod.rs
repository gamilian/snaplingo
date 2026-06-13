mod trait_def;
mod registry;
mod service;
mod impls;

#[cfg(test)]
mod registry_test;

#[cfg(test)]
mod service_test;

pub use trait_def::TranslationProvider;
pub use registry::TranslationRegistry;
pub use service::TranslationService;
pub use impls::{GoogleTranslateProvider, DeepLProvider, BaiduTranslateProvider};

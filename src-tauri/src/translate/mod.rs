pub mod provider;
mod google;
#[cfg(test)]
mod google_test;

pub use provider::{TranslationProvider, TranslationResult};
pub use google::GoogleTranslateProvider;

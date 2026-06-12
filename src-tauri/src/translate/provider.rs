use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Translation result containing translated text and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationResult {
    /// Provider identifier that produced this result
    pub provider_id: String,
    /// Translated text
    pub text: String,
    /// Auto-detected source language code (ISO 639-1, e.g., "en", "zh-CN", "ja")
    /// Present when language was auto-detected rather than specified
    pub detected_language: Option<String>,
}

/// Trait for translation providers
#[async_trait]
pub trait TranslationProvider: Send + Sync {
    /// Returns the name of the translation provider
    fn name(&self) -> &str;

    /// Returns a unique identifier for this provider
    fn id(&self) -> &str;

    /// Returns whether this provider requires an API key
    fn requires_api_key(&self) -> bool;

    /// Translates text from one language to another
    ///
    /// # Arguments
    /// * `text` - Text to translate
    /// * `source_lang` - Source language code (ISO 639-1)
    /// * `target_lang` - Target language code (ISO 639-1)
    ///
    /// # Returns
    /// Translation result containing translated text and metadata
    async fn translate(
        &self,
        text: &str,
        source_lang: &str,
        target_lang: &str,
    ) -> Result<TranslationResult>;
}

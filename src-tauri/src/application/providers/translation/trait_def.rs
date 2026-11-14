use crate::application::providers::common::Provider;
use crate::domain::translation::{TranslationRequest, TranslationResult};
use crate::Result;
use async_trait::async_trait;

/// Provider trait for translation services.
///
/// This trait extends the base Provider trait with translation-specific functionality.
/// Implementations must provide an async translate method that takes a TranslationRequest
/// and returns a TranslationResult.
#[async_trait]
pub trait TranslationProvider: Provider {
    /// Builds a replacement without mutating a provider used by an in-flight request.
    fn with_credentials(
        &self,
        _credentials: &std::collections::HashMap<String, String>,
    ) -> Result<std::sync::Arc<dyn TranslationProvider>> {
        Err(crate::AppError::Other(format!(
            "Provider {} does not support runtime credential reconfiguration",
            self.id()
        )))
    }

    /// Translates text from source language to target language.
    ///
    /// # Arguments
    ///
    /// * `request` - The translation request containing text and language information
    ///
    /// # Returns
    ///
    /// * `Result<TranslationResult>` - The translation result or an error
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * The provider is not configured
    /// * The API request fails
    /// * The response cannot be parsed
    async fn translate(&self, request: &TranslationRequest) -> Result<TranslationResult>;
}

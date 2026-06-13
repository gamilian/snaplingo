/// Base trait for all provider types (OCR, Translation, TTS).
///
/// This trait defines the common interface that all providers must implement,
/// regardless of their specific functionality.
pub trait Provider: Send + Sync {
    /// Returns the unique identifier for this provider.
    ///
    /// This ID is used internally to identify and reference the provider.
    /// Example: "google-translate", "tesseract-ocr"
    fn id(&self) -> &str;

    /// Returns the human-readable display name for this provider.
    ///
    /// This name is shown to users in the UI.
    /// Example: "Google Translate", "Tesseract OCR"
    fn name(&self) -> &str;

    /// Returns whether this provider is configured and ready to use.
    ///
    /// A provider is configured if it has all the necessary settings
    /// (e.g., API keys, endpoints) to perform its function.
    fn is_configured(&self) -> bool;

    /// Returns whether this provider requires an API key.
    ///
    /// This helps the UI determine whether to prompt for credentials.
    fn requires_api_key(&self) -> bool;
}

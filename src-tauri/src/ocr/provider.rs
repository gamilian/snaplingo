use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// OCR result containing detected text and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrResult {
    /// Extracted text from the image
    pub text: String,
    /// Confidence score (0.0-1.0), if provided by the OCR provider
    pub confidence: Option<f32>,
    /// Detected language code (ISO 639-1, e.g., "en", "zh-CN", "ja")
    pub language: Option<String>,
}

/// Trait for OCR providers
#[async_trait]
pub trait OcrProvider: Send + Sync {
    /// Returns the name of the OCR provider
    fn name(&self) -> &str;

    /// Returns a unique identifier for this provider
    fn id(&self) -> &str;

    /// Returns whether this provider requires an API key
    fn requires_api_key(&self) -> bool;

    /// Performs OCR on the provided image bytes
    ///
    /// # Arguments
    /// * `image_bytes` - Raw image data (PNG, JPEG, etc.)
    ///
    /// # Returns
    /// OCR result containing extracted text and metadata
    async fn recognize(&self, image_bytes: &[u8]) -> Result<OcrResult>;
}

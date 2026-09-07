use crate::application::providers::common::Provider;
use crate::domain::ocr::{OcrRequest, OcrResult};
use crate::Result;
use async_trait::async_trait;

/// Provider trait for OCR services.
///
/// This trait extends the base Provider trait with OCR-specific functionality.
/// Implementations must provide an async recognize method that takes an OcrRequest
/// and returns an OcrResult.
#[async_trait]
pub trait OcrProvider: Provider {
    /// Recognizes text from image data.
    ///
    /// # Arguments
    ///
    /// * `request` - The OCR request containing image data and optional language hint
    ///
    /// # Returns
    ///
    /// * `Result<OcrResult>` - The OCR result containing recognized text or an error
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * The provider is not configured
    /// * The API request fails
    /// * The response cannot be parsed
    async fn recognize(&self, request: &OcrRequest) -> Result<OcrResult>;

    /// Runs an optional provider-local recovery pass for low-quality results.
    ///
    /// Remote providers keep the default single-request behavior. Local engines
    /// may override this to preprocess the image and retry only when the first
    /// pass is empty or below their quality threshold.
    async fn recognize_with_recovery(&self, request: &OcrRequest) -> Result<OcrResult> {
        self.recognize(request).await
    }
}

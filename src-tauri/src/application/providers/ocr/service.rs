use super::registry::OcrRegistry;
use crate::domain::ocr::{OcrRequest, OcrResult};
use crate::Result;
use std::sync::{Arc, Mutex};

/// Service for managing OCR operations with single provider execution.
///
/// This service handles OCR requests by coordinating with the OcrRegistry
/// and executing recognition using the single active provider.
///
/// NOTE: History recording will be added in Phase 5
pub struct OcrService {
    registry: Arc<Mutex<OcrRegistry>>,
}

impl OcrService {
    /// Creates a new OcrService with the given registry.
    pub fn new(registry: Arc<Mutex<OcrRegistry>>) -> Self {
        Self { registry }
    }

    /// Recognizes text using the active provider.
    ///
    /// This method:
    /// 1. Gets the active provider from the registry
    /// 2. Calls the provider's recognize method
    /// 3. Returns the result
    ///
    /// # Arguments
    ///
    /// * `request` - The OCR request
    ///
    /// # Returns
    ///
    /// * `Result<OcrResult>` - Result from the active provider
    ///
    /// # Errors
    ///
    /// Returns an error if no provider is active.
    pub async fn recognize(&self, request: &OcrRequest) -> Result<OcrResult> {
        // Get active provider
        let active_provider = {
            let registry = self.registry.lock().unwrap();
            registry.get_active()
        };

        // Check if a provider is active
        let provider = active_provider
            .ok_or_else(|| "No active OCR provider".to_string())?;

        // Call provider's recognize method
        let result = provider.recognize(request).await?;

        // TODO(Phase 5): Add history recording here

        Ok(result)
    }
}

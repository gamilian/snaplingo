/// OCR provider implementations

use crate::providers::OcrProvider;

// Placeholder for Tesseract OCR
pub struct TesseractProvider;

impl OcrProvider for TesseractProvider {
    fn id(&self) -> &str {
        "tesseract"
    }

    fn name(&self) -> &str {
        "Tesseract OCR"
    }

    fn recognize(&self, _image: &[u8]) -> Result<String, String> {
        // TODO: Implement Tesseract OCR
        Err("Not implemented".to_string())
    }
}

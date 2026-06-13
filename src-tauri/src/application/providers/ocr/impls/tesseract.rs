use async_trait::async_trait;
use crate::application::providers::common::Provider;
use crate::application::providers::ocr::OcrProvider;
use crate::domain::ocr::{OcrRequest, OcrResult};
use crate::{AppError, Result};

/// Tesseract OCR provider (local, no API required).
///
/// This provider uses the tesseract-rs crate to perform OCR locally
/// without requiring any external API calls or API keys.
#[derive(Debug, Clone)]
pub struct TesseractProvider;

impl TesseractProvider {
    /// Creates a new Tesseract provider instance.
    pub fn new() -> Self {
        Self
    }
}

impl Default for TesseractProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl Provider for TesseractProvider {
    fn id(&self) -> &str {
        "tesseract"
    }

    fn name(&self) -> &str {
        "Tesseract OCR"
    }

    fn is_configured(&self) -> bool {
        // Tesseract is always configured since it's local and requires no API key
        true
    }

    fn requires_api_key(&self) -> bool {
        false
    }
}

#[async_trait]
impl OcrProvider for TesseractProvider {
    async fn recognize(&self, request: &OcrRequest) -> Result<OcrResult> {
        // Map language code to Tesseract language code (e.g., "zh-CN" -> "chi_sim")
        let lang = request.language.as_deref().and_then(map_language_code);

        // Create Tesseract instance with optional language
        let mut tess = tesseract::Tesseract::new(None, lang)
            .map_err(|e| AppError::Other(format!("Failed to initialize Tesseract: {}", e)))?;

        // Set image data from memory
        tess = tess.set_image_from_mem(&request.image_data)
            .map_err(|e| AppError::Other(format!("Failed to set image data: {}", e)))?;

        // Perform OCR
        let text = tess.get_text()
            .map_err(|e| AppError::Other(format!("Failed to recognize text: {}", e)))?;

        // Tesseract doesn't provide confidence scores easily in this API,
        // so we return None for confidence
        Ok(OcrResult {
            text,
            confidence: None,
        })
    }
}

/// Maps common language codes to Tesseract language codes.
///
/// Returns None for unknown languages, which will use Tesseract's default (English).
fn map_language_code(lang: &str) -> Option<&'static str> {
    match lang.to_lowercase().as_str() {
        "en" | "en-us" | "en-gb" => Some("eng"),
        "zh" | "zh-cn" | "zh-hans" => Some("chi_sim"),
        "zh-tw" | "zh-hk" | "zh-hant" => Some("chi_tra"),
        "ja" | "ja-jp" => Some("jpn"),
        "ko" | "ko-kr" => Some("kor"),
        "fr" | "fr-fr" => Some("fra"),
        "de" | "de-de" => Some("deu"),
        "es" | "es-es" => Some("spa"),
        "it" | "it-it" => Some("ita"),
        "pt" | "pt-pt" | "pt-br" => Some("por"),
        "ru" | "ru-ru" => Some("rus"),
        "ar" | "ar-sa" => Some("ara"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_traits() {
        let provider = TesseractProvider::new();

        assert_eq!(provider.id(), "tesseract");
        assert_eq!(provider.name(), "Tesseract OCR");
        assert!(provider.is_configured());
        assert!(!provider.requires_api_key());
    }

    #[test]
    fn test_language_mapping() {
        assert_eq!(map_language_code("en"), Some("eng"));
        assert_eq!(map_language_code("EN-US"), Some("eng"));
        assert_eq!(map_language_code("zh-CN"), Some("chi_sim"));
        assert_eq!(map_language_code("zh-TW"), Some("chi_tra"));
        assert_eq!(map_language_code("ja"), Some("jpn"));
        assert_eq!(map_language_code("unknown"), None);
    }

    #[test]
    fn test_default_impl() {
        let provider = TesseractProvider::default();
        assert_eq!(provider.id(), "tesseract");
    }

    // Note: Integration tests with actual OCR would require:
    // 1. Tesseract installed on the system
    // 2. Test image data
    // 3. Trained language data files
    // These are better suited for integration tests in a CI environment
}

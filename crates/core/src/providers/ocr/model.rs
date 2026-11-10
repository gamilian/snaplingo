use serde::{Deserialize, Serialize};

/// Request for OCR service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrRequest {
    pub image_data: Vec<u8>,
    pub language: Option<String>,
}

/// Result from OCR service
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OcrResult {
    pub text: String,
    pub confidence: Option<f32>,
    /// Normalized coordinates with an origin at the image's lower-left corner.
    #[serde(default)]
    pub lines: Vec<OcrLine>,
    #[serde(default)]
    pub detected_language: Option<String>,
    #[serde(default)]
    pub provider_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OcrLine {
    pub text: String,
    pub confidence: Option<f32>,
    pub bounding_box: Option<OcrBoundingBox>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct OcrBoundingBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl OcrResult {
    pub fn from_text(text: impl Into<String>, confidence: Option<f32>) -> Self {
        let text = text.into();
        let lines = text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|line| OcrLine {
                text: line.to_string(),
                confidence,
                bounding_box: None,
            })
            .collect();

        Self {
            text,
            confidence,
            lines,
            detected_language: None,
            provider_id: None,
        }
    }

    pub fn with_provider_id(mut self, provider_id: impl Into<String>) -> Self {
        self.provider_id = Some(provider_id.into());
        self
    }
}

pub fn infer_language_from_text(text: &str) -> Option<String> {
    if text
        .chars()
        .any(|character| ('\u{3040}'..='\u{30ff}').contains(&character))
    {
        return Some("ja".to_string());
    }
    if text
        .chars()
        .any(|character| ('\u{ac00}'..='\u{d7af}').contains(&character))
    {
        return Some("ko".to_string());
    }
    if text
        .chars()
        .any(|character| ('\u{0400}'..='\u{04ff}').contains(&character))
    {
        return Some("ru".to_string());
    }
    if text
        .chars()
        .any(|character| ('\u{3400}'..='\u{9fff}').contains(&character))
    {
        return Some("zh".to_string());
    }
    text.chars()
        .any(|character| character.is_ascii_alphabetic())
        .then_some("en".to_string())
}

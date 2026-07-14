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
}

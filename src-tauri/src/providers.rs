/// Provider system for OCR and Translation

pub mod ocr;
pub mod translation;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub provider_type: ProviderType,
    pub is_active: bool,
    pub requires_api_key: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    Ocr,
    Translation,
}

/// Trait for OCR providers
pub trait OcrProvider: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn recognize(&self, image: &[u8]) -> Result<String, String>;
}

/// Trait for translation providers
pub trait TranslationProvider: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn translate(&self, text: &str, from: &str, to: &str) -> Result<String, String>;
}

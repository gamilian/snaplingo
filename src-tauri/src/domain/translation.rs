use serde::{Deserialize, Serialize};

/// Request for translation service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationRequest {
    pub text: String,
    pub source_lang: String,
    pub target_lang: String,
    pub provider: String,
}

impl TranslationRequest {
    pub fn builder() -> TranslationRequestBuilder {
        TranslationRequestBuilder::default()
    }
}

#[derive(Default)]
pub struct TranslationRequestBuilder {
    text: Option<String>,
    source_lang: Option<String>,
    target_lang: Option<String>,
    provider: Option<String>,
}

impl TranslationRequestBuilder {
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    pub fn source_lang(mut self, lang: impl Into<String>) -> Self {
        self.source_lang = Some(lang.into());
        self
    }

    pub fn target_lang(mut self, lang: impl Into<String>) -> Self {
        self.target_lang = Some(lang.into());
        self
    }

    pub fn provider(mut self, provider: impl Into<String>) -> Self {
        self.provider = Some(provider.into());
        self
    }

    pub fn build(self) -> Result<TranslationRequest, String> {
        Ok(TranslationRequest {
            text: self.text.ok_or("text is required")?,
            source_lang: self.source_lang.ok_or("source_lang is required")?,
            target_lang: self.target_lang.ok_or("target_lang is required")?,
            provider: self.provider.ok_or("provider is required")?,
        })
    }
}

/// Result from translation service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationResult {
    pub translated_text: String,
    pub detected_language: Option<String>,
    pub confidence: Option<f32>,
}

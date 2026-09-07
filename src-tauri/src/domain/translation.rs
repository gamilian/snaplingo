use serde::{Deserialize, Serialize};

/// Request for translation service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationRequest {
    pub text: String,
    pub source_lang: String,
    pub target_lang: String,
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

    pub fn build(self) -> Result<TranslationRequest, String> {
        Ok(TranslationRequest {
            text: self.text.ok_or("text is required")?,
            source_lang: self.source_lang.ok_or("source_lang is required")?,
            target_lang: self.target_lang.ok_or("target_lang is required")?,
        })
    }
}

/// Result from translation service
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TranslationResult {
    pub provider_id: String,
    pub translated_text: String,
    pub detected_language: Option<String>,
    pub confidence: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<TranslationError>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TranslationError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

impl TranslationResult {
    pub fn success(
        provider_id: impl Into<String>,
        translated_text: impl Into<String>,
        detected_language: Option<String>,
        confidence: Option<f32>,
    ) -> Self {
        Self {
            provider_id: provider_id.into(),
            translated_text: translated_text.into(),
            detected_language,
            confidence,
            error: None,
            request_id: None,
            duration_ms: None,
        }
    }

    pub fn failure(
        provider_id: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
        retryable: bool,
    ) -> Self {
        Self {
            provider_id: provider_id.into(),
            translated_text: String::new(),
            detected_language: None,
            confidence: None,
            error: Some(TranslationError {
                code: code.into(),
                message: message.into(),
                retryable,
            }),
            request_id: None,
            duration_ms: None,
        }
    }
}

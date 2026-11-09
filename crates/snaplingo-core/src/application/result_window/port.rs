use async_trait::async_trait;
use serde::Serialize;

/// Identifies a single result-window payload handoff.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResultWindowRequestId(pub u64);

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResultWindowMode {
    Translation,
    Ocr,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResultWindowOrigin {
    Selection,
    Screenshot,
    Input,
    Ocr,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResultWindowOcrIntent {
    DisplayText,
    File,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultWindowPayload {
    pub mode: ResultWindowMode,
    pub origin: ResultWindowOrigin,
    pub text: String,
    pub auto_translate: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ocr_intent: Option<ResultWindowOcrIntent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_base64: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detected_language: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ResultWindowOpenRequest {
    Translation {
        text: String,
        auto_translate: bool,
        origin: ResultWindowOrigin,
        detected_language: Option<String>,
    },
    Ocr {
        text: String,
        intent: ResultWindowOcrIntent,
        image_base64: Option<String>,
        confidence: Option<f32>,
        detected_language: Option<String>,
    },
}

impl ResultWindowOpenRequest {
    pub fn manual_translation(text: String) -> Self {
        Self::Translation {
            text,
            auto_translate: false,
            origin: ResultWindowOrigin::Input,
            detected_language: None,
        }
    }

    pub fn selection_translation(text: String) -> Self {
        Self::Translation {
            text,
            auto_translate: true,
            origin: ResultWindowOrigin::Selection,
            detected_language: None,
        }
    }

    pub fn screenshot_translation(text: String, detected_language: Option<String>) -> Self {
        Self::Translation {
            text,
            auto_translate: true,
            origin: ResultWindowOrigin::Screenshot,
            detected_language,
        }
    }

    pub fn show_translation() -> Self {
        Self::manual_translation(String::new())
    }

    pub fn display_ocr(text: String) -> Self {
        Self::Ocr {
            text,
            intent: ResultWindowOcrIntent::DisplayText,
            image_base64: None,
            confidence: None,
            detected_language: None,
        }
    }

    pub fn capture_ocr(
        text: String,
        image_base64: Option<String>,
        confidence: Option<f32>,
        detected_language: Option<String>,
    ) -> Self {
        Self::Ocr {
            text,
            intent: ResultWindowOcrIntent::DisplayText,
            image_base64,
            confidence,
            detected_language,
        }
    }

    pub fn file_ocr() -> Self {
        Self::Ocr {
            text: String::new(),
            intent: ResultWindowOcrIntent::File,
            image_base64: None,
            confidence: None,
            detected_language: None,
        }
    }
}

#[async_trait]
pub trait ResultWindowWindowPort: Send + Sync {
    async fn show_or_create(&self) -> crate::Result<()>;
}

#[async_trait]
pub trait ResultWindowNotifierPort: Send + Sync {
    async fn notify_payload_ready(&self, request_id: ResultWindowRequestId) -> crate::Result<()>;
}

use async_trait::async_trait;
use serde::Serialize;

/// Identifies a single result-window payload handoff.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ResultWindowRequestId(pub(crate) u64);

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ResultWindowMode {
    Translation,
    Ocr,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ResultWindowOcrIntent {
    DisplayText,
    File,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResultWindowPayload {
    pub(crate) mode: ResultWindowMode,
    pub(crate) text: String,
    pub(crate) auto_translate: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) ocr_intent: Option<ResultWindowOcrIntent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) image_base64: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) confidence: Option<f32>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ResultWindowOpenRequest {
    Translation {
        text: String,
        auto_translate: bool,
    },
    InputTranslation,
    Ocr {
        text: String,
        intent: ResultWindowOcrIntent,
        image_base64: Option<String>,
        confidence: Option<f32>,
    },
}

impl ResultWindowOpenRequest {
    pub(crate) fn manual_translation(text: String) -> Self {
        Self::Translation {
            text,
            auto_translate: false,
        }
    }

    pub(crate) fn automatic_translation(text: String) -> Self {
        Self::Translation {
            text,
            auto_translate: true,
        }
    }

    pub(crate) fn show_translation() -> Self {
        Self::manual_translation(String::new())
    }

    pub(crate) fn input_translation() -> Self {
        Self::InputTranslation
    }

    pub(crate) fn display_ocr(text: String) -> Self {
        Self::Ocr {
            text,
            intent: ResultWindowOcrIntent::DisplayText,
            image_base64: None,
            confidence: None,
        }
    }

    pub(crate) fn capture_ocr(
        text: String,
        image_base64: Option<String>,
        confidence: Option<f32>,
    ) -> Self {
        Self::Ocr {
            text,
            intent: ResultWindowOcrIntent::DisplayText,
            image_base64,
            confidence,
        }
    }

    pub(crate) fn file_ocr() -> Self {
        Self::Ocr {
            text: String::new(),
            intent: ResultWindowOcrIntent::File,
            image_base64: None,
            confidence: None,
        }
    }
}

#[async_trait]
pub(crate) trait ResultWindowWindowPort: Send + Sync {
    async fn show_or_create(&self) -> crate::Result<()>;
}

#[async_trait]
pub(crate) trait ResultWindowClipboardPort: Send + Sync {
    async fn read_text(&self) -> crate::Result<String>;
}

#[async_trait]
pub(crate) trait ResultWindowNotifierPort: Send + Sync {
    async fn notify_payload_ready(&self, request_id: ResultWindowRequestId) -> crate::Result<()>;
}

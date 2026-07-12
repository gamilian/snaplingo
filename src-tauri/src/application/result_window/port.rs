use async_trait::async_trait;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ResultWindowMode {
    Translation,
    Ocr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ResultWindowOcrIntent {
    Show,
    DisplayText,
    File,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ResultWindowPayload {
    pub(crate) mode: ResultWindowMode,
    pub(crate) text: String,
    pub(crate) auto_translate: bool,
    pub(crate) ocr_intent: Option<ResultWindowOcrIntent>,
    pub(crate) image_base64: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
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
    },
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
    async fn notify_payload_ready(&self) -> crate::Result<()>;
}

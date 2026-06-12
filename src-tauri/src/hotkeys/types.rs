use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaptureMode {
    ScreenOcr,
    ClipboardOcr,
    DirectTranslate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyConfig {
    pub mode: CaptureMode,
    pub key: String,
    pub modifiers: Vec<String>,
}

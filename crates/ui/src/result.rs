//! Result-window drawing and input shared by desktop hosts.
mod draw;
mod icons;
mod state;

use crate::{Key, Modifiers, Point, RasterImage, Rect, Scene, Snapshot};
pub use state::ResultUi;

pub const LANGUAGES: &[(&str, &str)] = &[
    ("auto", "自动检测"),
    ("en", "英语"),
    ("zh-CN", "简体中文"),
    ("zh-TW", "繁体中文"),
    ("ja", "日语"),
    ("ko", "韩语"),
    ("es", "西班牙语"),
    ("fr", "法语"),
    ("de", "德语"),
    ("ru", "俄语"),
    ("ar", "阿拉伯语"),
];
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ResultMode {
    #[default]
    Translation,
    Ocr,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowStatus {
    Pending,
    Success,
    Error,
}
#[derive(Clone, Debug)]
pub struct ResultRow {
    pub id: String,
    pub name: String,
    pub status: RowStatus,
    pub text: String,
    pub error: Option<String>,
    pub favorite: bool,
    pub retryable: bool,
}
#[derive(Clone, Debug)]
pub struct ResultCopyToken {
    pub label: String,
    pub value: String,
}

#[derive(Clone, Debug, Default)]
pub struct ResultSnapshot {
    pub shell: Snapshot,
    pub revision: u64,
    pub epoch: u64,
    pub visible: bool,
    pub mode: ResultMode,
    pub source: String,
    pub composing: bool,
    pub source_language: String,
    pub detected_language: Option<String>,
    pub target_language: String,
    pub rows: Vec<ResultRow>,
    pub translating: bool,
    pub ocr_text: String,
    pub ocr_tokens: Vec<ResultCopyToken>,
    pub ocr_image: Option<RasterImage>,
    pub ocr_metadata: String,
    pub ocr_language: Option<String>,
    pub ocr_confidence: Option<f32>,
    pub ocr_running: bool,
    pub dialog_open: bool,
    pub favorite_pending: bool,
    pub ocr_favorite: bool,
    pub error: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ResultTarget {
    Close,
    Pin,
    Header,
    ToggleSource,
    ToggleRow(String),
    Source,
    SourceLanguage,
    TargetLanguage,
    Swap,
    Translate,
    ClearSource,
    CopySource,
    SpeakSource,
    FavoriteAll,
    FileOcr,
    OcrText,
    CopyOcr,
    CopyToken(String),
    SpeakOcr,
    FavoriteOcr,
    ClearOcr,
    ResultText(String),
    Retry(String),
    CopyResult(String),
    SpeakResult(String),
    FavoriteResult(String),
    Language(bool, String),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResultRole {
    Button,
    MultilineInput,
    Text,
    MenuItem,
}
#[derive(Clone, Debug)]
pub struct ResultControl {
    pub target: ResultTarget,
    pub rect: Rect,
    pub hit_rect: Rect,
    pub label: String,
    pub value: Option<String>,
    pub role: ResultRole,
    pub enabled: bool,
}
pub struct ResultFrame {
    pub scene: Scene,
    pub controls: Vec<ResultControl>,
    pub panel: Rect,
    pub cards: Vec<Rect>,
}
pub enum ResultInput {
    Activate(ResultTarget),
    Focus(ResultTarget),
    SetSource(String),
    PointerDown(Point),
    PointerMove(Point),
    PointerUp,
    Scroll(f32),
    Key(Key, Modifiers),
    Text(String),
    Selection(std::ops::Range<usize>),
    Preedit(String, Option<std::ops::Range<usize>>),
    Commit(String),
    ImeDisabled,
    FocusLost,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResultIntent {
    Edit(String, bool),
    SourceLanguage(String),
    TargetLanguage(String),
    Swap,
    Translate,
    Retry(String),
    FileOcr,
    ClearOcr,
    Copy(String),
    ReadClipboard,
    Speak(String, Option<String>),
    Favorite(Vec<String>),
    Close,
    Drag,
}

#[cfg(test)]
mod tests;

use crate::{Key, KeyStyle, Modifiers, Point, RasterImage, Rect, SettingsFrame, Snapshot};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LibraryKind {
    History,
    #[default]
    Favorites,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ContentKind {
    #[default]
    Translation,
    Ocr,
    Screenshot,
}
impl ContentKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Translation => "翻译",
            Self::Ocr => "OCR",
            Self::Screenshot => "截图",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ItemKey {
    pub kind: ContentKind,
    pub id: i64,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LibraryFilter {
    #[default]
    All,
    Translation,
    Ocr,
    Screenshot,
}
impl LibraryFilter {
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "全部",
            Self::Translation => "翻译",
            Self::Ocr => "OCR",
            Self::Screenshot => "截图",
        }
    }
    pub fn for_kind(kind: LibraryKind) -> &'static [Self] {
        if kind == LibraryKind::History {
            &[Self::All, Self::Translation, Self::Ocr]
        } else {
            &[Self::All, Self::Translation, Self::Ocr, Self::Screenshot]
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LibraryQuery {
    pub filter: LibraryFilter,
    pub search: String,
    pub page: usize,
}
#[derive(Clone, Debug)]
pub struct TranslationResult {
    pub provider: String,
    pub text: String,
    pub favorite: bool,
}
#[derive(Clone, Debug)]
pub enum LibraryContent {
    Translation {
        source: String,
        source_language: String,
        target_language: String,
        results: Vec<TranslationResult>,
    },
    Ocr {
        text: String,
        language: Option<String>,
        provider: String,
        confidence: Option<f32>,
        can_rerun: bool,
        favorite: bool,
    },
    Screenshot {
        width: u32,
        height: u32,
    },
}
#[derive(Clone, Debug)]
pub struct LibraryItem {
    pub key: ItemKey,
    /// The host formats clock/time-zone dependent labels in the current locale.
    pub relative_time: String,
    pub date: String,
    pub content: LibraryContent,
    pub thumbnail: Option<RasterImage>,
    pub note: Option<String>,
    pub tags: Vec<String>,
}
impl LibraryItem {
    pub fn title(&self, kind: LibraryKind) -> String {
        let text = match &self.content {
            LibraryContent::Translation { source, .. } => source,
            LibraryContent::Ocr { text, .. } => text,
            LibraryContent::Screenshot { width, height } => {
                return self
                    .note
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
                    .unwrap_or_else(|| format!("截图 {width} × {height}"))
            }
        };
        text.lines()
            .find(|line| !line.trim().is_empty())
            .map(str::trim)
            .unwrap_or(if kind == LibraryKind::History {
                "空白记录"
            } else {
                "空白收藏"
            })
            .into()
    }
    pub fn preview(&self, kind: LibraryKind) -> String {
        match &self.content {
            LibraryContent::Translation {
                source_language,
                target_language,
                results,
                ..
            } => format!(
                "{} · {source_language} → {target_language}",
                if kind == LibraryKind::History {
                    format!("{} 个服务", results.len())
                } else {
                    results
                        .first()
                        .map(|r| r.provider.clone())
                        .unwrap_or_default()
                }
            ),
            LibraryContent::Ocr {
                language, provider, ..
            } => format!("{provider} · {}", language.as_deref().unwrap_or("自动识别")),
            LibraryContent::Screenshot { width, height } => {
                if self.tags.is_empty() {
                    format!("{width} × {height}")
                } else {
                    self.tags.join(" · ")
                }
            }
        }
    }
}
#[derive(Clone, Debug)]
pub struct LibrarySnapshot {
    pub shell: Snapshot,
    pub kind: LibraryKind,
    pub revision: u64,
    pub query: LibraryQuery,
    pub items: Vec<LibraryItem>,
    pub total: usize,
    pub loading: bool,
    pub error: Option<String>,
}
impl Default for LibrarySnapshot {
    fn default() -> Self {
        Self {
            shell: Snapshot::default(),
            kind: LibraryKind::Favorites,
            revision: 0,
            query: LibraryQuery::default(),
            items: vec![],
            total: 0,
            loading: false,
            error: None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LibraryField {
    Search,
    Note(ItemKey),
    Tags(ItemKey),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CopyPart {
    Source,
    Result(usize),
    Ocr,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LibraryTarget {
    Search,
    Filter(LibraryFilter),
    Item(ItemKey),
    PreviousPage,
    NextPage,
    ClearHistory,
    Copy(ItemKey, CopyPart),
    Favorite(ItemKey, usize),
    Delete(ItemKey),
    Unfavorite(ItemKey),
    RerunOcr(ItemKey),
    CopyScreenshot(ItemKey),
    RevealScreenshot(ItemKey),
    Note(ItemKey),
    Tags(ItemKey),
}
impl LibraryTarget {
    pub(crate) fn field(self) -> Option<LibraryField> {
        match self {
            Self::Search => Some(LibraryField::Search),
            Self::Note(key) => Some(LibraryField::Note(key)),
            Self::Tags(key) => Some(LibraryField::Tags(key)),
            _ => None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibraryRole {
    Button,
    ListItem,
    SearchInput,
    TextInput,
    MultilineInput,
}
#[derive(Clone, Debug)]
pub struct LibraryControl {
    pub target: LibraryTarget,
    pub rect: Rect,
    pub hit_rect: Rect,
    pub role: LibraryRole,
    pub label: String,
    pub enabled: bool,
    pub selected: bool,
}
#[derive(Clone, Debug)]
pub struct LibraryFrame {
    pub page: SettingsFrame,
    pub controls: Vec<LibraryControl>,
}
impl LibraryFrame {
    pub fn hit_test(&self, point: Point) -> Option<LibraryTarget> {
        self.controls
            .iter()
            .rev()
            .find(|c| c.enabled && c.hit_rect.contains(point))
            .map(|c| c.target)
    }
}
#[derive(Clone, Debug)]
pub enum LibraryInput {
    Refresh,
    Snapshot(LibrarySnapshot),
    Activate(LibraryTarget),
    Focus(LibraryTarget),
    PointerDown(Point),
    PointerMove(Point),
    PointerUp,
    Scroll {
        position: Point,
        delta_y: f32,
    },
    KeyDown {
        key: Key,
        modifiers: Modifiers,
    },
    Text(String),
    ClipboardText(String),
    Selection(std::ops::Range<usize>),
    ImePreedit {
        text: String,
        selection: Option<std::ops::Range<usize>>,
    },
    ImeCommit(String),
    ImeDisabled,
    FocusLost,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LibraryIntent {
    Query {
        revision: u64,
        kind: LibraryKind,
        query: LibraryQuery,
    },
    CopyText(String),
    ReadClipboard,
    Favorite {
        key: ItemKey,
        result: usize,
    },
    Delete(ItemKey),
    /// Requests the shared Settings confirmation workflow before clearing history.
    ConfirmClearHistory(LibraryFilter),
    RerunOcrAndCopy(ItemKey),
    SaveMetadata {
        key: ItemKey,
        note: Option<String>,
        tags: Vec<String>,
    },
    CopyScreenshot(ItemKey),
    RevealScreenshot(ItemKey),
}

pub(crate) fn primary(style: KeyStyle, modifiers: Modifiers) -> bool {
    if style == KeyStyle::Mac {
        modifiers.meta
    } else {
        modifiers.control
    }
}

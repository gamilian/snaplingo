use crate::{Key, KeyStyle, Modifiers, OcrField, Page, Point, Section, Size, Theme};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OcrPreferences {
    pub gpu_acceleration: bool,
    pub recognition_language: String,
    pub preserve_formatting: bool,
    pub remove_chinese_spaces: bool,
    pub show_confidence: bool,
    pub window_position: String,
    pub hide_silent_status: bool,
}
impl Default for OcrPreferences {
    fn default() -> Self {
        Self {
            gpu_acceleration: true,
            recognition_language: "auto".into(),
            preserve_formatting: true,
            remove_chinese_spaces: true,
            show_confidence: false,
            window_position: "cursor".into(),
            hide_silent_status: false,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hotkey {
    pub value: String,
    pub default_value: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Capacity {
    pub retention_days: u32,
    pub history_used: Option<u32>,
    pub history_limit: u32,
    pub favorites_used: Option<u32>,
    pub favorites_limit: u32,
}
impl Default for Capacity {
    fn default() -> Self {
        Self {
            retention_days: 30,
            history_used: None,
            history_limit: 5000,
            favorites_used: None,
            favorites_limit: 1000,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub theme: Theme,
    pub dark: bool,
    pub key_style: KeyStyle,
    pub general: crate::GeneralPreferences,
    pub general_runtime: crate::GeneralRuntime,
    pub ocr: OcrPreferences,
    pub screenshot: crate::ScreenshotPreferences,
    pub screenshot_hotkeys: [Hotkey; 5],
    pub translation: crate::TranslationPreferences,
    pub translation_hotkeys: [Hotkey; 3],
    pub hotkeys: [Hotkey; 3],
    pub capacity: Capacity,
    pub error: Option<String>,
}
impl Default for Snapshot {
    fn default() -> Self {
        Self {
            theme: Theme::Native,
            dark: false,
            key_style: KeyStyle::Mac,
            general: crate::GeneralPreferences::default(),
            general_runtime: crate::GeneralRuntime::default(),
            ocr: OcrPreferences::default(),
            screenshot: crate::ScreenshotPreferences::default(),
            screenshot_hotkeys: ["F1", "⌘F1", "F3", "⇧F3", "⌘F3"].map(|v| Hotkey {
                value: v.into(),
                default_value: v.into(),
            }),
            translation: crate::TranslationPreferences::default(),
            translation_hotkeys: ["⌥D", "⌥S", "⌥A"].map(|v| Hotkey {
                value: v.into(),
                default_value: v.into(),
            }),
            hotkeys: ["⇧⌥S", "未设置", "未设置"].map(|v| Hotkey {
                value: v.into(),
                default_value: v.into(),
            }),
            capacity: Capacity::default(),
            error: None,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Number(u32),
    Toggle(bool),
    Text(String),
}
impl OcrField {
    pub const ALL: [Self; 7] = [
        Self::GpuAcceleration,
        Self::RecognitionLanguage,
        Self::PreserveFormatting,
        Self::RemoveChineseSpaces,
        Self::ShowConfidence,
        Self::WindowPosition,
        Self::HideSilentStatus,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::GpuAcceleration => "本地模型 GPU 加速",
            Self::RecognitionLanguage => "识别语言",
            Self::PreserveFormatting => "保留文本格式",
            Self::RemoveChineseSpaces => "中文自动去除空格",
            Self::ShowConfidence => "显示识别置信度",
            Self::WindowPosition => "OCR 窗口位置",
            Self::HideSilentStatus => "隐藏识别状态提示",
        }
    }
    pub fn choices(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Self::RecognitionLanguage => &[
                ("auto", "自动检测"),
                ("zh-CN", "中文简体"),
                ("zh-TW", "中文繁體"),
                ("en", "English"),
                ("ja", "日本語"),
                ("ko", "한국어"),
            ],
            Self::WindowPosition => &[
                ("center", "居中"),
                ("below-cursor", "鼠标下方"),
                ("cursor", "鼠标位置"),
                ("last-position", "上次停留位置"),
            ],
            _ => &[],
        }
    }
    pub fn value(self, s: &OcrPreferences) -> Value {
        match self {
            Self::GpuAcceleration => Value::Toggle(s.gpu_acceleration),
            Self::RecognitionLanguage => Value::Text(s.recognition_language.clone()),
            Self::PreserveFormatting => Value::Toggle(s.preserve_formatting),
            Self::RemoveChineseSpaces => Value::Toggle(s.remove_chinese_spaces),
            Self::ShowConfidence => Value::Toggle(s.show_confidence),
            Self::WindowPosition => Value::Text(s.window_position.clone()),
            Self::HideSilentStatus => Value::Toggle(s.hide_silent_status),
        }
    }
    pub(crate) fn apply(self, s: &mut OcrPreferences, v: &Value) {
        match (self, v) {
            (Self::GpuAcceleration, Value::Toggle(v)) => s.gpu_acceleration = *v,
            (Self::RecognitionLanguage, Value::Text(v)) => s.recognition_language = v.clone(),
            (Self::PreserveFormatting, Value::Toggle(v)) => s.preserve_formatting = *v,
            (Self::RemoveChineseSpaces, Value::Toggle(v)) => s.remove_chinese_spaces = *v,
            (Self::ShowConfidence, Value::Toggle(v)) => s.show_confidence = *v,
            (Self::WindowPosition, Value::Text(v)) => s.window_position = v.clone(),
            (Self::HideSilentStatus, Value::Toggle(v)) => s.hide_silent_status = *v,
            _ => {}
        }
    }
}
impl Page {
    pub const ALL: [Self; 7] = [
        Self::General,
        Self::Screenshot,
        Self::Translation,
        Self::Ocr,
        Self::Services,
        Self::Favorites,
        Self::History,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::General => "通用",
            Self::Screenshot => "截图",
            Self::Translation => "翻译",
            Self::Ocr => "OCR",
            Self::Services => "服务",
            Self::Favorites => "收藏夹",
            Self::History => "历史记录",
        }
    }
}
impl Section {
    pub fn for_page(page: Page) -> &'static [Self] {
        if page == Page::General {
            &[
                Self::Interface,
                Self::Permissions,
                Self::Network,
                Self::Maintenance,
                Self::Experimental,
                Self::About,
            ]
        } else if page == Page::Screenshot {
            &[Self::Hotkeys, Self::Output, Self::CaptureInterface]
        } else if page == Page::Translation {
            &[Self::Hotkeys, Self::Language, Self::Window, Self::Behavior]
        } else {
            &Self::ALL
        }
    }
    pub fn label_for(self, page: Page) -> &'static str {
        if self == Self::Window && page == Page::Translation {
            "窗口与输入框"
        } else {
            self.label()
        }
    }
    pub const ALL: [Self; 3] = [Self::Hotkeys, Self::Recognition, Self::Window];
    pub fn label(self) -> &'static str {
        match self {
            Self::Interface => "界面与启动",
            Self::Permissions => "系统权限",
            Self::Network => "网络",
            Self::Maintenance => "日志与维护",
            Self::Experimental => "实验性功能",
            Self::About => "关于",
            Self::Hotkeys => "快捷键",
            Self::Recognition => "识别与文本",
            Self::Window => "窗口与提示",
            Self::Language => "语言与取词",
            Self::Behavior => "翻译行为",
            Self::Output => "保存与输出",
            Self::CaptureInterface => "截图界面",
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    PointerMove(Point),
    PointerDown(Point),
    PointerUp(Point),
    Activate(crate::Target),
    Focus(crate::Target),
    SetValue(crate::Target, Value),
    Scroll {
        position: Point,
        delta_y: f32,
    },
    KeyDown {
        key: Key,
        modifiers: Modifiers,
        repeat: bool,
    },
    KeyUp {
        key: Key,
    },
    FocusLost,
    Snapshot(Snapshot),
    Resize(Size),
    /// OS-registered shortcuts are delivered through the host recording adapter.
    RecordedShortcut(String),
    TextInput(String),
    /// Host replacement/selection offsets in the committed UTF-8 string.
    TextSelection(std::ops::Range<usize>),
    ImePreedit {
        text: String,
        selection: Option<std::ops::Range<usize>>,
    },
    ImeCommit(String),
    ImeDisabled,
    ClipboardText(String),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    EditGeneral {
        field: crate::GeneralField,
        value: Value,
    },
    GeneralCommand(crate::GeneralCommand),
    Navigate(Page),
    EditScreenshot {
        field: crate::ScreenshotField,
        value: Value,
    },
    SelectScreenshotDirectory,
    SelectScreenshotColor {
        field: crate::ScreenshotField,
        rgba: crate::Color,
    },
    BeginLocalShortcutRecording {
        field: crate::ScreenshotField,
    },
    CopyToClipboard(String),
    ReadClipboard,
    EditTranslation {
        field: crate::TranslationField,
        value: Value,
    },
    EditOcr {
        field: OcrField,
        value: Value,
    },
    BeginHotkeyRecording {
        index: usize,
    },
    EndHotkeyRecording,
    SaveHotkey {
        index: usize,
        shortcut: String,
    },
    ResetHotkey {
        index: usize,
    },
    ResetHotkeys,
    CheckHotkeyConflicts,
    ConfigureCapacity,
}

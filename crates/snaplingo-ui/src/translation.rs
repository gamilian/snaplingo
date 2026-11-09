use crate::Value;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranslationPreferences {
    pub default_source_lang: String,
    pub default_target_lang: String,
    pub auto_translate: bool,
    pub auto_copy: bool,
    pub preserve_line_breaks: bool,
    pub incremental_translation: bool,
    pub window_always_on_top: bool,
    pub hide_on_blur: bool,
    pub selection_window_position: String,
    pub input_window_position: String,
    pub selection_input_state: String,
    pub screenshot_input_state: String,
    pub max_window_height_ratio: u8,
    pub window_width: u16,
    pub selection_text_mode: String,
}

impl Default for TranslationPreferences {
    fn default() -> Self {
        Self {
            default_source_lang: "auto".to_string(),
            default_target_lang: "zh-CN".to_string(),
            auto_translate: true,
            auto_copy: false,
            preserve_line_breaks: true,
            incremental_translation: false,
            window_always_on_top: true,
            hide_on_blur: true,
            selection_window_position: "below-cursor".to_string(),
            input_window_position: "center".to_string(),
            selection_input_state: "last".to_string(),
            screenshot_input_state: "last".to_string(),
            max_window_height_ratio: 70,
            window_width: 660,
            selection_text_mode: "smart".to_string(),
        }
    }
}

const POSITIONS: &[(&str, &str)] = &[
    ("center", "居中"),
    ("below-cursor", "鼠标下方"),
    ("cursor", "鼠标位置"),
    ("last-position", "上次停留位置"),
];
const SOURCE_LANGUAGES: &[(&str, &str)] = &[
    ("auto", "自动检测"),
    ("en", "英语"),
    ("zh-CN", "中文简体"),
    ("zh-TW", "中文繁体"),
    ("ja", "日语"),
    ("ko", "韩语"),
    ("es", "西班牙语"),
    ("fr", "法语"),
    ("de", "德语"),
    ("ru", "俄语"),
    ("ar", "阿拉伯语"),
];
const TARGET_LANGUAGES: &[(&str, &str)] = &[
    ("auto", "自动"),
    ("en", "英语"),
    ("zh-CN", "中文简体"),
    ("zh-TW", "中文繁体"),
    ("ja", "日语"),
    ("ko", "韩语"),
    ("es", "西班牙语"),
    ("fr", "法语"),
    ("de", "德语"),
    ("ru", "俄语"),
    ("ar", "阿拉伯语"),
];
const INPUT_STATES: &[(&str, &str)] = &[
    ("last", "上次状态"),
    ("collapsed", "总是折叠"),
    ("expanded", "总是展开"),
];
const SELECTION_MODES: &[(&str, &str)] = &[
    ("smart", "智能模式"),
    ("quality", "效果优先"),
    ("speed", "速度优先"),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TranslationField {
    DefaultSourceLang,
    DefaultTargetLang,
    SelectionTextMode,
    SelectionWindowPosition,
    InputWindowPosition,
    MaxWindowHeightRatio,
    WindowWidth,
    SelectionInputState,
    ScreenshotInputState,
    AutoTranslate,
    IncrementalTranslation,
    AutoCopy,
    PreserveLineBreaks,
    WindowAlwaysOnTop,
    HideOnBlur,
}

impl TranslationField {
    pub const ALL: [Self; 15] = [
        Self::DefaultSourceLang,
        Self::DefaultTargetLang,
        Self::SelectionTextMode,
        Self::SelectionWindowPosition,
        Self::InputWindowPosition,
        Self::MaxWindowHeightRatio,
        Self::WindowWidth,
        Self::SelectionInputState,
        Self::ScreenshotInputState,
        Self::AutoTranslate,
        Self::IncrementalTranslation,
        Self::AutoCopy,
        Self::PreserveLineBreaks,
        Self::WindowAlwaysOnTop,
        Self::HideOnBlur,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::DefaultSourceLang => "源语言",
            Self::DefaultTargetLang => "目标语言",
            Self::SelectionTextMode => "取词模式",
            Self::SelectionWindowPosition => "划词翻译与截图翻译",
            Self::InputWindowPosition => "手动翻译窗口",
            Self::MaxWindowHeightRatio => "最高屏幕占比",
            Self::WindowWidth => "窗口宽度",
            Self::SelectionInputState => "划词翻译",
            Self::ScreenshotInputState => "截图翻译",
            Self::AutoTranslate => "自动翻译",
            Self::IncrementalTranslation => "增量翻译",
            Self::AutoCopy => "翻译后自动复制",
            Self::PreserveLineBreaks => "保留原文换行",
            Self::WindowAlwaysOnTop => "结果窗口置顶",
            Self::HideOnBlur => "失去焦点时隐藏",
        }
    }

    pub fn choices(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Self::DefaultSourceLang => SOURCE_LANGUAGES,
            Self::DefaultTargetLang => TARGET_LANGUAGES,
            Self::SelectionTextMode => SELECTION_MODES,
            Self::SelectionWindowPosition | Self::InputWindowPosition => POSITIONS,
            Self::SelectionInputState | Self::ScreenshotInputState => INPUT_STATES,
            _ => &[],
        }
    }

    pub fn value(self, settings: &TranslationPreferences) -> Value {
        use Value::*;
        match self {
            Self::DefaultSourceLang => Text(settings.default_source_lang.clone()),
            Self::DefaultTargetLang => Text(settings.default_target_lang.clone()),
            Self::SelectionTextMode => Text(settings.selection_text_mode.clone()),
            Self::SelectionWindowPosition => Text(settings.selection_window_position.clone()),
            Self::InputWindowPosition => Text(settings.input_window_position.clone()),
            Self::MaxWindowHeightRatio => Number(settings.max_window_height_ratio.into()),
            Self::WindowWidth => Number(settings.window_width.into()),
            Self::SelectionInputState => Text(settings.selection_input_state.clone()),
            Self::ScreenshotInputState => Text(settings.screenshot_input_state.clone()),
            Self::AutoTranslate => Toggle(settings.auto_translate),
            Self::IncrementalTranslation => Toggle(settings.incremental_translation),
            Self::AutoCopy => Toggle(settings.auto_copy),
            Self::PreserveLineBreaks => Toggle(settings.preserve_line_breaks),
            Self::WindowAlwaysOnTop => Toggle(settings.window_always_on_top),
            Self::HideOnBlur => Toggle(settings.hide_on_blur),
        }
    }

    pub(crate) fn apply(
        self,
        settings: &mut TranslationPreferences,
        value: Value,
    ) -> Result<(), String> {
        use Value::*;
        match (self, value) {
            (Self::DefaultSourceLang, Text(value)) => {
                validate_choice(&value, self.choices())?;
                settings.default_source_lang = value;
            }
            (Self::DefaultTargetLang, Text(value)) => {
                validate_choice(&value, self.choices())?;
                settings.default_target_lang = value;
            }
            (Self::SelectionTextMode, Text(value)) => {
                validate_choice(&value, self.choices())?;
                settings.selection_text_mode = value;
            }
            (Self::SelectionWindowPosition, Text(value)) => {
                validate_choice(&value, self.choices())?;
                settings.selection_window_position = value;
            }
            (Self::InputWindowPosition, Text(value)) => {
                validate_choice(&value, self.choices())?;
                settings.input_window_position = value;
            }
            (Self::MaxWindowHeightRatio, Number(value)) if (30..=90).contains(&value) => {
                settings.max_window_height_ratio = value as u8
            }
            (Self::WindowWidth, Number(value)) if (300..=1000).contains(&value) => {
                settings.window_width = value as u16
            }
            (Self::SelectionInputState, Text(value)) => {
                validate_choice(&value, self.choices())?;
                settings.selection_input_state = value;
            }
            (Self::ScreenshotInputState, Text(value)) => {
                validate_choice(&value, self.choices())?;
                settings.screenshot_input_state = value;
            }
            (Self::AutoTranslate, Toggle(value)) => settings.auto_translate = value,
            (Self::IncrementalTranslation, Toggle(value)) => {
                settings.incremental_translation = value
            }
            (Self::AutoCopy, Toggle(value)) => settings.auto_copy = value,
            (Self::PreserveLineBreaks, Toggle(value)) => settings.preserve_line_breaks = value,
            (Self::WindowAlwaysOnTop, Toggle(value)) => settings.window_always_on_top = value,
            (Self::HideOnBlur, Toggle(value)) => settings.hide_on_blur = value,
            (Self::MaxWindowHeightRatio | Self::WindowWidth, Number(_)) => {
                return Err("Setting value is outside its supported range".into())
            }
            _ => return Err("Setting value does not match its control".into()),
        }
        Ok(())
    }
}

fn validate_choice(value: &str, choices: &[(&str, &str)]) -> Result<(), String> {
    if choices.iter().any(|(id, _)| *id == value) {
        Ok(())
    } else {
        Err("Setting value is not one of its supported choices".into())
    }
}

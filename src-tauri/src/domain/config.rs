use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct GeneralSettings {
    pub language: String,
    pub theme: String,
    pub start_on_boot: bool,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            language: "zh-CN".to_string(),
            theme: "system".to_string(),
            start_on_boot: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ScreenshotSettings {
    pub save_path: String,
    pub format: String,
    pub quality: u8,
    pub naming_rule: String,
    pub custom_file_name: String,
    pub auto_copy: bool,
    pub default_stroke_width: u8,
    pub default_font_size: u8,
    pub remember_last_tool: bool,
    pub show_selection_size: bool,
    pub show_magnifier: bool,
    pub pin_opacity: u8,
    pub pin_shadow: bool,
    pub annotation_colors: Vec<[u8; 4]>,
}

impl Default for ScreenshotSettings {
    fn default() -> Self {
        Self {
            save_path: String::new(),
            format: "png".to_string(),
            quality: 90,
            naming_rule: "timestamp".to_string(),
            custom_file_name: "SnapLingo".to_string(),
            auto_copy: false,
            default_stroke_width: 2,
            default_font_size: 24,
            remember_last_tool: true,
            show_selection_size: true,
            show_magnifier: false,
            pin_opacity: 100,
            pin_shadow: true,
            annotation_colors: vec![
                [255, 77, 79, 255],
                [40, 167, 69, 255],
                [24, 144, 255, 255],
                [250, 219, 20, 255],
                [255, 255, 255, 255],
                [0, 0, 0, 255],
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct TranslationSettings {
    pub default_source_lang: String,
    pub default_target_lang: String,
    pub auto_translate: bool,
    pub auto_copy: bool,
    pub preserve_line_breaks: bool,
    pub incremental_translation: bool,
    pub window_always_on_top: bool,
    pub hide_on_blur: bool,
}

impl Default for TranslationSettings {
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
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct OcrSettings {
    pub recognition_language: String,
    pub auto_copy: bool,
    pub preserve_formatting: bool,
    pub remove_chinese_spaces: bool,
    pub show_confidence: bool,
}

impl Default for OcrSettings {
    fn default() -> Self {
        Self {
            recognition_language: "auto".to_string(),
            auto_copy: true,
            preserve_formatting: true,
            remove_chinese_spaces: true,
            show_confidence: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct HistorySettings {
    pub auto_cleanup_enabled: bool,
    pub retention_days: u32,
    pub maximum_records: u32,
    pub maximum_favorites: u32,
}

impl Default for HistorySettings {
    fn default() -> Self {
        Self {
            auto_cleanup_enabled: false,
            retention_days: 30,
            maximum_records: 5000,
            maximum_favorites: 1000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct SettingsSnapshot {
    pub general: GeneralSettings,
    pub screenshot: ScreenshotSettings,
    pub translation: TranslationSettings,
    pub ocr: OcrSettings,
    pub history: HistorySettings,
}

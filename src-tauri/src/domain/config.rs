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
}

impl Default for ScreenshotSettings {
    fn default() -> Self {
        Self {
            save_path: String::new(),
            format: "png".to_string(),
            quality: 90,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct TranslationSettings {
    pub default_source_lang: String,
    pub default_target_lang: String,
}

impl Default for TranslationSettings {
    fn default() -> Self {
        Self {
            default_source_lang: "auto".to_string(),
            default_target_lang: "zh-CN".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct SettingsSnapshot {
    pub general: GeneralSettings,
    pub screenshot: ScreenshotSettings,
    pub translation: TranslationSettings,
}

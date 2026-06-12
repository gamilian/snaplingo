/// Configuration management
/// Handles ~/.snaplingo/config.json and system credential stores

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub version: String,
    pub general: GeneralConfig,
    pub screenshot: ScreenshotConfig,
    pub ocr: OcrConfig,
    pub translation: TranslationConfig,
    pub hotkeys: HotkeysConfig,
    pub history: HistoryConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub language: String,
    pub theme: String,
    pub start_on_boot: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotConfig {
    pub default_save_path: String,
    pub format: String,
    pub quality: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrConfig {
    pub active_provider: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationConfig {
    pub active_providers: Vec<String>,
    pub default_target_language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeysConfig {
    pub screenshot: String,
    pub ocr: String,
    pub ocr_translate: String,
    pub selection_translate: String,
    pub input_translate: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryConfig {
    pub record_screenshot: bool,
    pub record_ocr: bool,
    pub record_translation: bool,
    pub auto_cleanup_enabled: bool,
    pub max_age_days: u32,
    pub max_entries: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: "0.1.0".to_string(),
            general: GeneralConfig {
                language: "en".to_string(),
                theme: "system".to_string(),
                start_on_boot: false,
            },
            screenshot: ScreenshotConfig {
                default_save_path: "~/Pictures/SnapLingo".to_string(),
                format: "png".to_string(),
                quality: 95,
            },
            ocr: OcrConfig {
                active_provider: None,
            },
            translation: TranslationConfig {
                active_providers: vec![],
                default_target_language: "zh-CN".to_string(),
            },
            hotkeys: HotkeysConfig {
                screenshot: "F1".to_string(),
                ocr: "Option+A".to_string(),
                ocr_translate: "Option+S".to_string(),
                selection_translate: "Option+D".to_string(),
                input_translate: "Option+W".to_string(),
            },
            history: HistoryConfig {
                record_screenshot: false,
                record_ocr: true,
                record_translation: true,
                auto_cleanup_enabled: true,
                max_age_days: 30,
                max_entries: 1000,
            },
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, String> {
        // TODO: Load from ~/.snaplingo/config.json
        Ok(Config::default())
    }

    pub fn save(&self) -> Result<(), String> {
        // TODO: Save to ~/.snaplingo/config.json
        Ok(())
    }
}

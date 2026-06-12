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
    pub advanced: AdvancedConfig,
    pub custom_providers: Vec<CustomProvider>,
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
    pub default_tool_color: String,
    pub default_stroke_width: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrConfig {
    pub active_provider: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedConfig {
    pub proxy_url: Option<String>,
    pub log_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomProvider {
    pub id: String,
    pub name: String,
    pub provider_type: String,
    pub api_format: String,
    pub endpoint: String,
    pub model: String,
}

use super::*;
use anyhow::{Context, Result};
use std::path::PathBuf;

impl Default for Config {
    fn default() -> Self {
        Config {
            version: "1.0.0".to_string(),
            general: GeneralConfig {
                language: "en".to_string(),
                theme: "system".to_string(),
                start_on_boot: true,
            },
            screenshot: ScreenshotConfig {
                default_save_path: "~/Pictures/SnapLingo".to_string(),
                format: "png".to_string(),
                quality: 95,
                default_tool_color: "#FF0000".to_string(),
                default_stroke_width: 3,
            },
            ocr: OcrConfig {
                active_provider: "tesseract".to_string(),
            },
            translation: TranslationConfig {
                active_providers: vec!["google-translate".to_string()],
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
            advanced: AdvancedConfig {
                proxy_url: None,
                log_level: "info".to_string(),
            },
            custom_providers: vec![],
        }
    }
}

impl Config {
    pub fn load_or_default(path: &PathBuf) -> Result<Self> {
        if path.exists() {
            let content = std::fs::read_to_string(path)
                .with_context(|| format!("Failed to read config from {:?}", path))?;
            serde_json::from_str(&content)
                .with_context(|| format!("Failed to parse config from {:?}", path))
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self, path: &PathBuf) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory {:?}", parent))?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)
            .with_context(|| format!("Failed to write config to {:?}", path))
    }
}

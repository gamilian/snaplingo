use serde::{Deserialize, Serialize};

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub translation_provider: String,
    pub source_language: String,
    pub target_language: String,
    pub hotkey: Option<String>,
    pub auto_copy: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            translation_provider: "google".to_string(),
            source_language: "auto".to_string(),
            target_language: "en".to_string(),
            hotkey: None,
            auto_copy: false,
        }
    }
}

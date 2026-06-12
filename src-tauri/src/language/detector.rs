use anyhow::Result;
use lingua::{Language, LanguageDetectorBuilder};

pub struct LanguageDetector {
    detector: lingua::LanguageDetector,
}

impl LanguageDetector {
    pub fn new() -> Self {
        let detector = LanguageDetectorBuilder::from_all_languages().build();
        Self { detector }
    }

    pub fn detect(&self, text: &str) -> Result<String> {
        let detected = self.detector.detect_language_of(text)
            .ok_or_else(|| anyhow::anyhow!("Unable to detect language"))?;

        let lang_code = match detected {
            Language::Chinese => "zh-CN",
            Language::English => "en",
            Language::Spanish => "es",
            Language::Japanese => "ja",
            Language::French => "fr",
            Language::German => "de",
            Language::Korean => "ko",
            Language::Russian => "ru",
            _ => "en",
        };

        Ok(lang_code.to_string())
    }

    pub fn smart_target(&self, source: &str) -> String {
        if source.starts_with("zh") {
            "en".to_string()
        } else {
            "zh-CN".to_string()
        }
    }
}

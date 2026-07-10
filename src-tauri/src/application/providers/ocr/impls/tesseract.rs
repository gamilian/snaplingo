use std::sync::Arc;

use async_trait::async_trait;

use crate::application::providers::common::Provider;
use crate::application::providers::ocr::{OcrProvider, TesseractEngine};
use crate::domain::ocr::{OcrRequest, OcrResult};
use crate::Result;

#[derive(Clone)]
pub struct TesseractProvider {
    engine: Arc<dyn TesseractEngine>,
}

impl TesseractProvider {
    pub(crate) fn new(engine: Arc<dyn TesseractEngine>) -> Self {
        Self { engine }
    }
}

impl Provider for TesseractProvider {
    fn id(&self) -> &str {
        "tesseract"
    }

    fn name(&self) -> &str {
        "Tesseract OCR"
    }

    fn is_configured(&self) -> bool {
        true
    }

    fn requires_api_key(&self) -> bool {
        false
    }
}

#[async_trait]
impl OcrProvider for TesseractProvider {
    async fn recognize(&self, request: &OcrRequest) -> Result<OcrResult> {
        let available_languages = self.engine.available_languages()?;
        let language =
            tesseract_language_for_request(request.language.as_deref(), &available_languages);
        let text = self
            .engine
            .recognize(&request.image_data, language.as_deref())?;

        Ok(OcrResult {
            text,
            confidence: None,
        })
    }
}

fn tesseract_language_for_request(
    requested_language: Option<&str>,
    available_languages: &[String],
) -> Option<String> {
    if let Some(language) = requested_language.and_then(map_language_code) {
        return Some(language.to_string());
    }

    if tesseract_language_is_available(available_languages, "chi_sim") {
        if tesseract_language_is_available(available_languages, "eng") {
            Some("chi_sim+eng".to_string())
        } else {
            Some("chi_sim".to_string())
        }
    } else {
        Some("eng".to_string())
    }
}

fn tesseract_language_is_available(available_languages: &[String], language: &str) -> bool {
    available_languages
        .iter()
        .any(|available_language| available_language == language)
}

fn map_language_code(lang: &str) -> Option<&'static str> {
    match lang.to_lowercase().as_str() {
        "auto" => None,
        "multi" | "zh+en" | "zh-cn+en" | "chi_sim+eng" => Some("chi_sim+eng"),
        "en" | "en-us" | "en-gb" | "eng" => Some("eng"),
        "zh" | "zh-cn" | "zh-hans" | "chi_sim" => Some("chi_sim"),
        "zh-tw" | "zh-hk" | "zh-hant" | "chi_tra" => Some("chi_tra"),
        "ja" | "ja-jp" | "jpn" => Some("jpn"),
        "ko" | "ko-kr" | "kor" => Some("kor"),
        "fr" | "fr-fr" | "fra" => Some("fra"),
        "de" | "de-de" | "deu" => Some("deu"),
        "es" | "es-es" | "spa" => Some("spa"),
        "it" | "it-it" | "ita" => Some("ita"),
        "pt" | "pt-pt" | "pt-br" | "por" => Some("por"),
        "ru" | "ru-ru" | "rus" => Some("rus"),
        "ar" | "ar-sa" | "ara" => Some("ara"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::AppError;

    struct RecordingTesseractEngine {
        available_languages: Vec<String>,
        available_error: Option<String>,
        recognized_text: String,
        recognize_error: Option<String>,
        requested_languages: Mutex<Vec<Option<String>>>,
    }

    impl RecordingTesseractEngine {
        fn succeeds(available_languages: Vec<&str>, recognized_text: &str) -> Self {
            Self {
                available_languages: available_languages
                    .into_iter()
                    .map(ToString::to_string)
                    .collect(),
                available_error: None,
                recognized_text: recognized_text.to_string(),
                recognize_error: None,
                requested_languages: Mutex::new(Vec::new()),
            }
        }
    }

    impl TesseractEngine for RecordingTesseractEngine {
        fn available_languages(&self) -> Result<Vec<String>> {
            match &self.available_error {
                Some(error) => Err(AppError::Other(error.clone())),
                None => Ok(self.available_languages.clone()),
            }
        }

        fn recognize(&self, _image_data: &[u8], language: Option<&str>) -> Result<String> {
            self.requested_languages
                .lock()
                .unwrap()
                .push(language.map(ToString::to_string));
            match &self.recognize_error {
                Some(error) => Err(AppError::Other(error.clone())),
                None => Ok(self.recognized_text.clone()),
            }
        }
    }

    #[test]
    fn provider_metadata_requires_no_credentials() {
        let provider = TesseractProvider::new(Arc::new(RecordingTesseractEngine::succeeds(
            vec!["eng"],
            "text",
        )));

        assert_eq!(provider.id(), "tesseract");
        assert_eq!(provider.name(), "Tesseract OCR");
        assert!(provider.is_configured());
        assert!(!provider.requires_api_key());
    }

    #[test]
    fn maps_supported_language_codes() {
        assert_eq!(map_language_code("EN-US"), Some("eng"));
        assert_eq!(map_language_code("zh-CN"), Some("chi_sim"));
        assert_eq!(map_language_code("zh-TW"), Some("chi_tra"));
        assert_eq!(map_language_code("multi"), Some("chi_sim+eng"));
        assert_eq!(map_language_code("ja"), Some("jpn"));
        assert_eq!(map_language_code("unknown"), None);
    }

    #[tokio::test]
    async fn recognize_uses_explicit_language_mapping() {
        let engine = Arc::new(RecordingTesseractEngine::succeeds(vec!["eng"], "hello"));
        let provider = TesseractProvider::new(engine.clone());

        let result = provider
            .recognize(&OcrRequest {
                image_data: vec![1, 2, 3],
                language: Some("en-US".to_string()),
            })
            .await
            .unwrap();

        assert_eq!(result.text, "hello");
        assert_eq!(
            *engine.requested_languages.lock().unwrap(),
            vec![Some("eng".to_string())]
        );
    }

    #[tokio::test]
    async fn recognize_defaults_to_chinese_and_english_when_available() {
        let engine = Arc::new(RecordingTesseractEngine::succeeds(
            vec!["eng", "chi_sim"],
            "hello",
        ));
        let provider = TesseractProvider::new(engine.clone());

        provider
            .recognize(&OcrRequest {
                image_data: vec![1, 2, 3],
                language: None,
            })
            .await
            .unwrap();

        assert_eq!(
            *engine.requested_languages.lock().unwrap(),
            vec![Some("chi_sim+eng".to_string())]
        );
    }

    #[tokio::test]
    async fn recognize_propagates_engine_errors() {
        let provider = TesseractProvider::new(Arc::new(RecordingTesseractEngine {
            available_languages: Vec::new(),
            available_error: Some("engine unavailable".to_string()),
            recognized_text: String::new(),
            recognize_error: None,
            requested_languages: Mutex::new(Vec::new()),
        }));

        let error = provider
            .recognize(&OcrRequest {
                image_data: vec![1, 2, 3],
                language: None,
            })
            .await
            .unwrap_err();

        assert!(error.to_string().contains("engine unavailable"));
    }
}

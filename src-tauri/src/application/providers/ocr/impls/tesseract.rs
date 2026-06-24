use crate::application::providers::common::Provider;
use crate::application::providers::ocr::OcrProvider;
use crate::domain::ocr::{OcrRequest, OcrResult};
use crate::{AppError, Result};
use async_trait::async_trait;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

static TESSERACT_OCR_LOCK: Mutex<()> = Mutex::new(());

/// Tesseract OCR provider (local, no API required).
///
/// This provider uses the tesseract-rs crate to perform OCR locally
/// without requiring any external API calls or API keys.
#[derive(Debug, Clone)]
pub struct TesseractProvider;

impl TesseractProvider {
    /// Creates a new Tesseract provider instance.
    pub fn new() -> Self {
        Self
    }
}

impl Default for TesseractProvider {
    fn default() -> Self {
        Self::new()
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
        // Tesseract is always configured since it's local and requires no API key
        true
    }

    fn requires_api_key(&self) -> bool {
        false
    }
}

#[async_trait]
impl OcrProvider for TesseractProvider {
    async fn recognize(&self, request: &OcrRequest) -> Result<OcrResult> {
        // Check if Tesseract is available before attempting OCR
        if !is_tesseract_available() {
            return Err(AppError::Other(
                "Tesseract OCR is not available. Please install Tesseract OCR. \
                See docs/TESSERACT_SETUP.md for installation instructions."
                    .to_string(),
            ));
        }

        let frame = tesseract_frame_from_image_data(&request.image_data)?;

        // Map language code to Tesseract language code (e.g., "zh-CN" -> "chi_sim")
        let available_languages = available_tesseract_languages();
        let lang =
            tesseract_language_for_request(request.language.as_deref(), &available_languages);

        let _guard = TESSERACT_OCR_LOCK
            .lock()
            .map_err(|_| AppError::Other("Tesseract OCR lock poisoned".to_string()))?;

        // Create Tesseract instance with optional language
        let mut tess = tesseract::Tesseract::new(None, lang.as_deref())
            .map_err(|e| AppError::Other(format!("Failed to initialize Tesseract: {}", e)))?;

        // Set image data from normalized raw pixels, avoiding native image decoder crashes.
        tess = tess
            .set_frame(
                &frame.pixels,
                frame.width,
                frame.height,
                frame.bytes_per_pixel,
                frame.bytes_per_line,
            )
            .map_err(|e| AppError::Other(format!("Failed to set image data: {}", e)))?;

        tess = tess
            .recognize()
            .map_err(|e| AppError::Other(format!("Failed to recognize text: {}", e)))?;

        // Perform OCR
        let text = tess
            .get_text()
            .map_err(|e| AppError::Other(format!("Failed to recognize text: {}", e)))?;

        // Tesseract doesn't provide confidence scores easily in this API,
        // so we return None for confidence
        Ok(OcrResult {
            text,
            confidence: None,
        })
    }
}

#[derive(Debug)]
struct TesseractFrame {
    pixels: Vec<u8>,
    width: i32,
    height: i32,
    bytes_per_pixel: i32,
    bytes_per_line: i32,
}

fn tesseract_frame_from_image_data(image_data: &[u8]) -> Result<TesseractFrame> {
    let image = image::load_from_memory(image_data)
        .map_err(|e| AppError::Other(format!("Invalid OCR image data: {}", e)))?
        .to_luma8();

    let width = i32::try_from(image.width())
        .map_err(|_| AppError::Other(format!("OCR image width {} is too large", image.width())))?;
    let height = i32::try_from(image.height()).map_err(|_| {
        AppError::Other(format!("OCR image height {} is too large", image.height()))
    })?;

    if width == 0 || height == 0 {
        return Err(AppError::Other(
            "Invalid OCR image data: image dimensions must be non-zero".to_string(),
        ));
    }

    Ok(TesseractFrame {
        pixels: image.into_raw(),
        width,
        height,
        bytes_per_pixel: 1,
        bytes_per_line: width,
    })
}

/// Checks if Tesseract is available on the system.
///
/// This attempts to run `tesseract --version` to verify installation.
fn is_tesseract_available() -> bool {
    available_tesseract_executable().is_some()
}

fn available_tesseract_executable() -> Option<PathBuf> {
    let path = std::env::var_os("PATH");
    tesseract_executable_candidates(path.as_deref())
        .into_iter()
        .find(|candidate| tesseract_version_succeeds(candidate))
}

fn tesseract_version_succeeds(executable: &Path) -> bool {
    std::process::Command::new(executable)
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn available_tesseract_languages() -> Vec<String> {
    available_tesseract_executable()
        .as_deref()
        .map(tesseract_languages_from_executable)
        .unwrap_or_default()
}

fn tesseract_languages_from_executable(executable: &Path) -> Vec<String> {
    std::process::Command::new(executable)
        .arg("--list-langs")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| parse_tesseract_languages(&String::from_utf8_lossy(&output.stdout)))
        .unwrap_or_default()
}

fn parse_tesseract_languages(output: &str) -> Vec<String> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| !line.starts_with("List of available languages"))
        .map(ToString::to_string)
        .collect()
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

fn tesseract_executable_candidates(path: Option<&OsStr>) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(path) = path {
        for directory in std::env::split_paths(path) {
            push_unique_candidate(&mut candidates, directory.join(tesseract_binary_name()));
        }
    }

    for fallback in tesseract_fallback_executable_paths() {
        push_unique_candidate(&mut candidates, fallback);
    }

    candidates
}

fn push_unique_candidate(candidates: &mut Vec<PathBuf>, candidate: PathBuf) {
    if !candidates.contains(&candidate) {
        candidates.push(candidate);
    }
}

fn tesseract_binary_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "tesseract.exe"
    } else {
        "tesseract"
    }
}

fn tesseract_fallback_executable_paths() -> Vec<PathBuf> {
    if cfg!(target_os = "macos") {
        vec![
            PathBuf::from("/opt/homebrew/bin/tesseract"),
            PathBuf::from("/usr/local/bin/tesseract"),
            PathBuf::from("/opt/local/bin/tesseract"),
        ]
    } else if cfg!(target_os = "windows") {
        vec![
            PathBuf::from(r"C:\Program Files\Tesseract-OCR\tesseract.exe"),
            PathBuf::from(r"C:\Program Files (x86)\Tesseract-OCR\tesseract.exe"),
        ]
    } else {
        vec![
            PathBuf::from("/usr/bin/tesseract"),
            PathBuf::from("/usr/local/bin/tesseract"),
            PathBuf::from("/snap/bin/tesseract"),
        ]
    }
}

/// Maps common language codes to Tesseract language codes.
///
/// Returns None for unknown languages, which will use Tesseract's default (English).
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
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_provider_traits() {
        let provider = TesseractProvider::new();

        assert_eq!(provider.id(), "tesseract");
        assert_eq!(provider.name(), "Tesseract OCR");
        assert!(provider.is_configured());
        assert!(!provider.requires_api_key());
    }

    #[test]
    fn test_language_mapping() {
        assert_eq!(map_language_code("en"), Some("eng"));
        assert_eq!(map_language_code("eng"), Some("eng"));
        assert_eq!(map_language_code("EN-US"), Some("eng"));
        assert_eq!(map_language_code("zh-CN"), Some("chi_sim"));
        assert_eq!(map_language_code("chi_sim"), Some("chi_sim"));
        assert_eq!(map_language_code("zh-TW"), Some("chi_tra"));
        assert_eq!(map_language_code("chi_tra"), Some("chi_tra"));
        assert_eq!(map_language_code("multi"), Some("chi_sim+eng"));
        assert_eq!(map_language_code("ja"), Some("jpn"));
        assert_eq!(map_language_code("unknown"), None);
    }

    #[test]
    fn test_default_tesseract_language_prefers_chinese_and_english_when_available() {
        let available = vec!["eng".to_string(), "chi_sim".to_string()];

        assert_eq!(
            tesseract_language_for_request(None, &available),
            Some("chi_sim+eng".to_string())
        );
        assert_eq!(
            tesseract_language_for_request(Some("auto"), &available),
            Some("chi_sim+eng".to_string())
        );
    }

    #[test]
    fn test_default_tesseract_language_keeps_english_when_chinese_is_unavailable() {
        let available = vec!["eng".to_string()];

        assert_eq!(
            tesseract_language_for_request(None, &available),
            Some("eng".to_string())
        );
    }

    #[test]
    fn test_parse_tesseract_languages_skips_header() {
        let output = "List of available languages in \"/opt/homebrew/share/tessdata/\" (3):\neng\nchi_sim\nosd\n";

        assert_eq!(
            parse_tesseract_languages(output),
            vec!["eng".to_string(), "chi_sim".to_string(), "osd".to_string()]
        );
    }

    #[test]
    fn test_default_impl() {
        let provider = TesseractProvider::default();
        assert_eq!(provider.id(), "tesseract");
    }

    #[test]
    fn test_tesseract_candidates_include_path_entries_first() {
        let path = std::env::join_paths([PathBuf::from("first-bin"), PathBuf::from("second-bin")])
            .unwrap();

        let candidates = tesseract_executable_candidates(Some(path.as_os_str()));

        assert_eq!(
            candidates[0],
            PathBuf::from("first-bin").join(tesseract_binary_name())
        );
        assert_eq!(
            candidates[1],
            PathBuf::from("second-bin").join(tesseract_binary_name())
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_tesseract_candidates_include_homebrew_paths_on_macos() {
        let candidates = tesseract_executable_candidates(Some(std::ffi::OsStr::new(
            "/usr/bin:/bin:/usr/sbin:/sbin",
        )));

        assert!(candidates.contains(&PathBuf::from("/opt/homebrew/bin/tesseract")));
        assert!(candidates.contains(&PathBuf::from("/usr/local/bin/tesseract")));
    }

    #[test]
    fn test_tesseract_candidates_do_not_duplicate_fallbacks() {
        let fallbacks = tesseract_fallback_executable_paths();
        let fallback = fallbacks[0].clone();
        let fallback_dir = fallback.parent().unwrap();
        let path = std::env::join_paths([fallback_dir]).unwrap();

        let candidates = tesseract_executable_candidates(Some(path.as_os_str()));

        assert_eq!(
            candidates
                .iter()
                .filter(|candidate| **candidate == fallback)
                .count(),
            1
        );
    }

    #[test]
    fn test_tesseract_frame_rejects_invalid_image_data() {
        let error = tesseract_frame_from_image_data(&[1, 2, 3]).unwrap_err();

        assert!(error.to_string().contains("Invalid OCR image data"));
    }

    #[test]
    fn test_tesseract_frame_normalizes_png_to_grayscale_pixels() {
        let mut image = image::GrayImage::new(2, 1);
        image.put_pixel(0, 0, image::Luma([12]));
        image.put_pixel(1, 0, image::Luma([240]));

        let mut png = Vec::new();
        image
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();

        let frame = tesseract_frame_from_image_data(&png).unwrap();

        assert_eq!(frame.width, 2);
        assert_eq!(frame.height, 1);
        assert_eq!(frame.bytes_per_pixel, 1);
        assert_eq!(frame.bytes_per_line, 2);
        assert_eq!(frame.pixels, vec![12, 240]);
    }

    // Note: Integration tests with actual OCR would require:
    // 1. Tesseract installed on the system
    // 2. Test image data
    // 3. Trained language data files
    // These are better suited for integration tests in a CI environment
}

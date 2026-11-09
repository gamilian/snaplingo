use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::application::providers::ocr::TesseractEngine;
use crate::{AppError, Result};

static TESSERACT_OCR_LOCK: Mutex<()> = Mutex::new(());

pub struct SystemTesseractEngine;

impl SystemTesseractEngine {
    pub fn new() -> Self {
        Self
    }
}

impl TesseractEngine for SystemTesseractEngine {
    fn available_languages(&self) -> Result<Vec<String>> {
        let executable =
            available_tesseract_executable().ok_or_else(tesseract_unavailable_error)?;
        Ok(tesseract_languages_from_executable(&executable))
    }

    fn recognize(&self, image_data: &[u8], language: Option<&str>) -> Result<String> {
        let frame = tesseract_frame_from_image_data(image_data)?;
        let _guard = TESSERACT_OCR_LOCK
            .lock()
            .map_err(|_| AppError::Other("Tesseract OCR lock poisoned".to_string()))?;

        let mut tesseract = ::tesseract::Tesseract::new(None, language)
            .map_err(|error| AppError::Other(format!("Failed to initialize Tesseract: {error}")))?;
        tesseract = tesseract
            .set_frame(
                &frame.pixels,
                frame.width,
                frame.height,
                frame.bytes_per_pixel,
                frame.bytes_per_line,
            )
            .map_err(|error| AppError::Other(format!("Failed to set image data: {error}")))?;
        tesseract = tesseract
            .recognize()
            .map_err(|error| AppError::Other(format!("Failed to recognize text: {error}")))?;
        tesseract
            .get_text()
            .map_err(|error| AppError::Other(format!("Failed to recognize text: {error}")))
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
        .map_err(|error| AppError::Other(format!("Invalid OCR image data: {error}")))?
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

fn tesseract_unavailable_error() -> AppError {
    AppError::Other(
        "Tesseract OCR is not available. Please install Tesseract OCR. \
         See docs/TESSERACT_SETUP.md for installation instructions."
            .to_string(),
    )
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn parses_tesseract_languages_without_cli_header() {
        let output = "List of available languages in \"/opt/homebrew/share/tessdata/\" (3):\neng\nchi_sim\nosd\n";

        assert_eq!(
            parse_tesseract_languages(output),
            vec!["eng".to_string(), "chi_sim".to_string(), "osd".to_string()]
        );
    }

    #[test]
    fn executable_candidates_include_path_entries_first() {
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

    #[test]
    fn executable_candidates_do_not_duplicate_fallbacks() {
        let fallback = tesseract_fallback_executable_paths()[0].clone();
        let path = std::env::join_paths([fallback.parent().unwrap()]).unwrap();
        let candidates = tesseract_executable_candidates(Some(path.as_os_str()));

        assert_eq!(
            candidates
                .iter()
                .filter(|candidate| **candidate == fallback)
                .count(),
            1
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn executable_candidates_include_homebrew_paths_on_macos() {
        let candidates =
            tesseract_executable_candidates(Some(OsStr::new("/usr/bin:/bin:/usr/sbin:/sbin")));

        assert!(candidates.contains(&PathBuf::from("/opt/homebrew/bin/tesseract")));
        assert!(candidates.contains(&PathBuf::from("/usr/local/bin/tesseract")));
    }

    #[test]
    fn frame_rejects_invalid_image_data() {
        let error = tesseract_frame_from_image_data(&[1, 2, 3]).unwrap_err();

        assert!(error.to_string().contains("Invalid OCR image data"));
    }

    #[test]
    fn frame_normalizes_png_to_grayscale_pixels() {
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
}

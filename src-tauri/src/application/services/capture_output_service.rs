use std::borrow::Cow;
use std::path::{Path, PathBuf};

use arboard::{Clipboard, ImageData};
use image::ImageEncoder;

use crate::error::{AppError, Result};

pub struct CaptureOutputService;

pub enum ClipboardCaptureOutput {
    Png(Vec<u8>),
    Text(String),
}

impl CaptureOutputService {
    pub fn new() -> Self {
        Self
    }

    pub fn default_capture_save_path(&self) -> PathBuf {
        let base_dir = dirs::download_dir()
            .or_else(dirs::picture_dir)
            .or_else(dirs::home_dir)
            .unwrap_or_else(std::env::temp_dir);
        let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();

        capture_save_path(&base_dir, &timestamp)
    }

    pub fn quick_capture_save_path(&self, directory: Option<&str>) -> PathBuf {
        let base_dir = directory
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .map(configured_capture_save_dir_for_system)
            .unwrap_or_else(default_quick_capture_save_dir);
        let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();

        quick_capture_save_file_path(&base_dir, &timestamp)
    }

    pub async fn save_png(&self, data: &[u8], path: &Path) -> Result<PathBuf> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(path, data)?;

        Ok(path.to_path_buf())
    }

    pub async fn copy_png(&self, data: &[u8]) -> Result<()> {
        let image = Self::png_to_clipboard_image(data)?;
        let mut clipboard = Clipboard::new()
            .map_err(|e| AppError::System(format!("Failed to open clipboard: {}", e)))?;
        clipboard
            .set_image(image)
            .map_err(|e| AppError::System(format!("Failed to copy image to clipboard: {}", e)))
    }

    pub fn read_clipboard_png(&self) -> Result<Vec<u8>> {
        let mut clipboard = Clipboard::new()
            .map_err(|e| AppError::System(format!("Failed to open clipboard: {}", e)))?;
        let image = clipboard
            .get_image()
            .map_err(|e| AppError::System(format!("Failed to read image from clipboard: {}", e)))?;

        Self::clipboard_image_to_png(image)
    }

    pub fn read_clipboard_text(&self) -> Result<String> {
        let mut clipboard = Clipboard::new()
            .map_err(|e| AppError::System(format!("Failed to open clipboard: {}", e)))?;

        clipboard
            .get_text()
            .map_err(|e| AppError::System(format!("Failed to read text from clipboard: {}", e)))
    }

    pub fn read_clipboard_capture_output(&self) -> Result<ClipboardCaptureOutput> {
        match self.read_clipboard_png() {
            Ok(png_data) => Ok(ClipboardCaptureOutput::Png(png_data)),
            Err(image_error) => {
                let text = self.read_clipboard_text().map_err(|text_error| {
                    AppError::System(format!(
                        "{}; also failed to read text from clipboard: {}",
                        image_error, text_error
                    ))
                })?;

                Ok(ClipboardCaptureOutput::Text(text))
            }
        }
    }

    pub fn png_to_clipboard_image(data: &[u8]) -> Result<ImageData<'static>> {
        let image = image::load_from_memory(data)
            .map_err(|e| AppError::System(format!("Failed to decode PNG for clipboard: {}", e)))?
            .to_rgba8();
        let (width, height) = image.dimensions();

        Ok(ImageData {
            width: width as usize,
            height: height as usize,
            bytes: Cow::Owned(image.into_raw()),
        })
    }

    pub fn clipboard_image_to_png(image: ImageData<'_>) -> Result<Vec<u8>> {
        let mut png = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut png);
        encoder
            .write_image(
                &image.bytes,
                image.width as u32,
                image.height as u32,
                image::ExtendedColorType::Rgba8,
            )
            .map_err(|e| AppError::System(format!("Failed to encode clipboard image: {}", e)))?;

        Ok(png)
    }
}

pub(crate) fn capture_save_path(base_dir: &Path, timestamp: &str) -> PathBuf {
    base_dir.join(format!("SnapLingo-{}.png", timestamp))
}

pub(crate) fn quick_capture_save_file_path(base_dir: &Path, timestamp: &str) -> PathBuf {
    capture_save_path(base_dir, timestamp)
}

fn default_quick_capture_save_dir() -> PathBuf {
    dirs::picture_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(std::env::temp_dir)
        .join("SnapLingo")
}

pub(crate) fn configured_capture_save_dir(configured: &str, home_dir: &Path) -> PathBuf {
    if configured == "~" {
        return home_dir.to_path_buf();
    }

    if let Some(relative) = configured.strip_prefix("~/") {
        return home_dir.join(relative);
    }

    PathBuf::from(configured)
}

fn configured_capture_save_dir_for_system(configured: &str) -> PathBuf {
    if configured == "~" || configured.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return configured_capture_save_dir(configured, &home);
        }
    }

    PathBuf::from(configured)
}

impl Default for CaptureOutputService {
    fn default() -> Self {
        Self::new()
    }
}

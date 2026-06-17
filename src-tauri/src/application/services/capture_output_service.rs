use std::borrow::Cow;
use std::path::{Path, PathBuf};

use arboard::{Clipboard, ImageData};
use image::ImageEncoder;

use crate::error::{AppError, Result};

pub struct CaptureOutputService;

impl CaptureOutputService {
    pub fn new() -> Self {
        Self
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

impl Default for CaptureOutputService {
    fn default() -> Self {
        Self::new()
    }
}

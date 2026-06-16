use std::borrow::Cow;
use std::path::{Path, PathBuf};

use arboard::{Clipboard, ImageData};

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
}

impl Default for CaptureOutputService {
    fn default() -> Self {
        Self::new()
    }
}

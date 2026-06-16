use std::io::Cursor;

use crate::domain::capture::PhysicalRect;
use crate::error::{AppError, Result};

pub struct ImageCompositionService;

impl ImageCompositionService {
    pub fn new() -> Self {
        Self
    }

    pub fn crop_png(&self, png_data: &[u8], rect: &PhysicalRect) -> Result<Vec<u8>> {
        if rect.width == 0 || rect.height == 0 {
            return Err(AppError::System(
                "Cannot crop a zero-sized capture region".to_string(),
            ));
        }
        if rect.x < 0 || rect.y < 0 {
            return Err(AppError::System(
                "Cannot crop a capture region with negative coordinates".to_string(),
            ));
        }

        let image = image::load_from_memory(png_data)
            .map_err(|e| AppError::System(format!("Failed to decode PNG: {}", e)))?;
        let x = rect.x as u32;
        let y = rect.y as u32;
        if x + rect.width > image.width() || y + rect.height > image.height() {
            return Err(AppError::System(
                "Capture crop region exceeds frozen image bounds".to_string(),
            ));
        }

        let cropped = image.crop_imm(x, y, rect.width, rect.height);
        let mut output = Vec::new();
        cropped
            .write_to(&mut Cursor::new(&mut output), image::ImageFormat::Png)
            .map_err(|e| AppError::System(format!("Failed to encode cropped PNG: {}", e)))?;
        Ok(output)
    }
}

impl Default for ImageCompositionService {
    fn default() -> Self {
        Self::new()
    }
}

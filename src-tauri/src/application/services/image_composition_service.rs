use std::io::Cursor;

use crate::domain::capture::PhysicalRect;
use crate::error::{AppError, Result};

pub struct PngPlacement<'a> {
    pub png_data: &'a [u8],
    pub source_rect: PhysicalRect,
    pub destination_rect: PhysicalRect,
}

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

    pub fn compose_png(
        &self,
        width: u32,
        height: u32,
        placements: &[PngPlacement<'_>],
    ) -> Result<Vec<u8>> {
        if width == 0 || height == 0 {
            return Err(AppError::System(
                "Cannot compose a zero-sized capture output".to_string(),
            ));
        }

        let mut output = image::RgbaImage::from_pixel(width, height, image::Rgba([0, 0, 0, 0]));

        for placement in placements {
            if placement.source_rect.width == 0
                || placement.source_rect.height == 0
                || placement.destination_rect.width == 0
                || placement.destination_rect.height == 0
            {
                return Err(AppError::System(
                    "Cannot compose a zero-sized capture region".to_string(),
                ));
            }
            if placement.source_rect.x < 0
                || placement.source_rect.y < 0
                || placement.destination_rect.x < 0
                || placement.destination_rect.y < 0
            {
                return Err(AppError::System(
                    "Cannot compose a capture region with negative coordinates".to_string(),
                ));
            }

            let image = image::load_from_memory(placement.png_data)
                .map_err(|e| AppError::System(format!("Failed to decode PNG: {}", e)))?;
            let source_x = placement.source_rect.x as u32;
            let source_y = placement.source_rect.y as u32;
            if source_x + placement.source_rect.width > image.width()
                || source_y + placement.source_rect.height > image.height()
            {
                return Err(AppError::System(
                    "Capture compose source region exceeds frozen image bounds".to_string(),
                ));
            }

            let destination_x = placement.destination_rect.x as u32;
            let destination_y = placement.destination_rect.y as u32;
            if destination_x + placement.destination_rect.width > width
                || destination_y + placement.destination_rect.height > height
            {
                return Err(AppError::System(
                    "Capture compose destination region exceeds output bounds".to_string(),
                ));
            }

            let crop = image
                .crop_imm(
                    source_x,
                    source_y,
                    placement.source_rect.width,
                    placement.source_rect.height,
                )
                .to_rgba8();
            let composed = if crop.width() == placement.destination_rect.width
                && crop.height() == placement.destination_rect.height
            {
                crop
            } else {
                image::imageops::resize(
                    &crop,
                    placement.destination_rect.width,
                    placement.destination_rect.height,
                    image::imageops::FilterType::Triangle,
                )
            };

            image::imageops::overlay(
                &mut output,
                &composed,
                placement.destination_rect.x.into(),
                placement.destination_rect.y.into(),
            );
        }

        let mut output_png = Vec::new();
        image::DynamicImage::ImageRgba8(output)
            .write_to(&mut Cursor::new(&mut output_png), image::ImageFormat::Png)
            .map_err(|e| AppError::System(format!("Failed to encode composed PNG: {}", e)))?;
        Ok(output_png)
    }
}

impl Default for ImageCompositionService {
    fn default() -> Self {
        Self::new()
    }
}

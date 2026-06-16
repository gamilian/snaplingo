use std::io::Cursor;

use crate::domain::capture::PhysicalRect;
use crate::error::{AppError, Result};

pub struct PngPlacement<'a> {
    pub png_data: &'a [u8],
    pub source_rect: PhysicalRect,
    pub destination_rect: PhysicalRect,
}

pub struct ImageAnnotation {
    pub rect: PhysicalRect,
    pub color: [u8; 4],
    pub stroke_width: u32,
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
        self.compose_png_with_annotations(width, height, placements, &[])
    }

    pub fn compose_png_with_annotations(
        &self,
        width: u32,
        height: u32,
        placements: &[PngPlacement<'_>],
        annotations: &[ImageAnnotation],
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
        for annotation in annotations {
            draw_rectangle_annotation(&mut output, annotation);
        }

        let mut output_png = Vec::new();
        image::DynamicImage::ImageRgba8(output)
            .write_to(&mut Cursor::new(&mut output_png), image::ImageFormat::Png)
            .map_err(|e| AppError::System(format!("Failed to encode composed PNG: {}", e)))?;
        Ok(output_png)
    }
}

fn draw_rectangle_annotation(output: &mut image::RgbaImage, annotation: &ImageAnnotation) {
    if annotation.rect.width == 0 || annotation.rect.height == 0 {
        return;
    }

    let stroke_width = annotation.stroke_width.max(1);
    let color = image::Rgba(annotation.color);
    let output_width = output.width() as i64;
    let output_height = output.height() as i64;
    let left = annotation.rect.x as i64;
    let top = annotation.rect.y as i64;
    let right = left + annotation.rect.width as i64 - 1;
    let bottom = top + annotation.rect.height as i64 - 1;

    for stroke in 0..stroke_width as i64 {
        draw_horizontal_line(
            output,
            left,
            right,
            top + stroke,
            output_width,
            output_height,
            color,
        );
        draw_horizontal_line(
            output,
            left,
            right,
            bottom - stroke,
            output_width,
            output_height,
            color,
        );
        draw_vertical_line(
            output,
            top,
            bottom,
            left + stroke,
            output_width,
            output_height,
            color,
        );
        draw_vertical_line(
            output,
            top,
            bottom,
            right - stroke,
            output_width,
            output_height,
            color,
        );
    }
}

fn draw_horizontal_line(
    output: &mut image::RgbaImage,
    left: i64,
    right: i64,
    y: i64,
    output_width: i64,
    output_height: i64,
    color: image::Rgba<u8>,
) {
    if y < 0 || y >= output_height {
        return;
    }

    for x in left.max(0)..=right.min(output_width - 1) {
        output.put_pixel(x as u32, y as u32, color);
    }
}

fn draw_vertical_line(
    output: &mut image::RgbaImage,
    top: i64,
    bottom: i64,
    x: i64,
    output_width: i64,
    output_height: i64,
    color: image::Rgba<u8>,
) {
    if x < 0 || x >= output_width {
        return;
    }

    for y in top.max(0)..=bottom.min(output_height - 1) {
        output.put_pixel(x as u32, y as u32, color);
    }
}

impl Default for ImageCompositionService {
    fn default() -> Self {
        Self::new()
    }
}

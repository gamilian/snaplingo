use std::f64::consts::PI;
use std::io::Cursor;

use crate::domain::capture::{PhysicalPoint, PhysicalRect};
use crate::error::{AppError, Result};

pub struct PngPlacement<'a> {
    pub png_data: &'a [u8],
    pub source_rect: PhysicalRect,
    pub destination_rect: PhysicalRect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageAnnotation {
    Rectangle {
        rect: PhysicalRect,
        color: [u8; 4],
        stroke_width: u32,
    },
    Arrow {
        start: PhysicalPoint,
        end: PhysicalPoint,
        color: [u8; 4],
        stroke_width: u32,
    },
    Freehand {
        points: Vec<PhysicalPoint>,
        color: [u8; 4],
        stroke_width: u32,
    },
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
            draw_annotation(&mut output, annotation);
        }

        let mut output_png = Vec::new();
        image::DynamicImage::ImageRgba8(output)
            .write_to(&mut Cursor::new(&mut output_png), image::ImageFormat::Png)
            .map_err(|e| AppError::System(format!("Failed to encode composed PNG: {}", e)))?;
        Ok(output_png)
    }
}

fn draw_annotation(output: &mut image::RgbaImage, annotation: &ImageAnnotation) {
    match annotation {
        ImageAnnotation::Rectangle {
            rect,
            color,
            stroke_width,
        } => draw_rectangle_annotation(output, rect, *color, *stroke_width),
        ImageAnnotation::Arrow {
            start,
            end,
            color,
            stroke_width,
        } => draw_arrow_annotation(output, start, end, *color, *stroke_width),
        ImageAnnotation::Freehand {
            points,
            color,
            stroke_width,
        } => draw_freehand_annotation(output, points, *color, *stroke_width),
    }
}

fn draw_rectangle_annotation(
    output: &mut image::RgbaImage,
    rect: &PhysicalRect,
    color: [u8; 4],
    stroke_width: u32,
) {
    if rect.width == 0 || rect.height == 0 {
        return;
    }

    let stroke_width = stroke_width.max(1);
    let color = image::Rgba(color);
    let output_width = output.width() as i64;
    let output_height = output.height() as i64;
    let left = rect.x as i64;
    let top = rect.y as i64;
    let right = left + rect.width as i64 - 1;
    let bottom = top + rect.height as i64 - 1;

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

fn draw_arrow_annotation(
    output: &mut image::RgbaImage,
    start: &PhysicalPoint,
    end: &PhysicalPoint,
    color: [u8; 4],
    stroke_width: u32,
) {
    let color = image::Rgba(color);
    let stroke_width = stroke_width.max(1);
    draw_line(output, start.x, start.y, end.x, end.y, color, stroke_width);

    let dx = (end.x - start.x) as f64;
    let dy = (end.y - start.y) as f64;
    let length = dx.hypot(dy);
    if length <= f64::EPSILON {
        return;
    }

    let head_length = ((stroke_width as f64) * 6.0).max(6.0).min(length * 0.8);
    let line_angle = dy.atan2(dx);
    let head_angle = 35.0_f64.to_radians();
    for wing_angle in [line_angle + PI - head_angle, line_angle + PI + head_angle] {
        let wing_end = PhysicalPoint {
            x: (end.x as f64 + wing_angle.cos() * head_length).round() as i32,
            y: (end.y as f64 + wing_angle.sin() * head_length).round() as i32,
        };
        draw_line(
            output,
            end.x,
            end.y,
            wing_end.x,
            wing_end.y,
            color,
            stroke_width,
        );
    }
}

fn draw_freehand_annotation(
    output: &mut image::RgbaImage,
    points: &[PhysicalPoint],
    color: [u8; 4],
    stroke_width: u32,
) {
    if points.is_empty() {
        return;
    }

    let color = image::Rgba(color);
    let stroke_width = stroke_width.max(1);
    if points.len() == 1 {
        draw_stroked_point(output, points[0].x, points[0].y, color, stroke_width);
        return;
    }

    for segment in points.windows(2) {
        let start = &segment[0];
        let end = &segment[1];
        draw_line(output, start.x, start.y, end.x, end.y, color, stroke_width);
    }
}

fn draw_line(
    output: &mut image::RgbaImage,
    start_x: i32,
    start_y: i32,
    end_x: i32,
    end_y: i32,
    color: image::Rgba<u8>,
    stroke_width: u32,
) {
    let mut x = start_x;
    let mut y = start_y;
    let dx = (end_x - start_x).abs();
    let sx = if start_x < end_x { 1 } else { -1 };
    let dy = -(end_y - start_y).abs();
    let sy = if start_y < end_y { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        draw_stroked_point(output, x, y, color, stroke_width);
        if x == end_x && y == end_y {
            break;
        }
        let e2 = err * 2;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

fn draw_stroked_point(
    output: &mut image::RgbaImage,
    x: i32,
    y: i32,
    color: image::Rgba<u8>,
    stroke_width: u32,
) {
    let stroke_width = stroke_width.max(1) as i32;
    let min_offset = -(stroke_width / 2);
    let max_offset = (stroke_width - 1) / 2;

    for offset_y in min_offset..=max_offset {
        for offset_x in min_offset..=max_offset {
            put_pixel_if_in_bounds(output, x + offset_x, y + offset_y, color);
        }
    }
}

fn put_pixel_if_in_bounds(output: &mut image::RgbaImage, x: i32, y: i32, color: image::Rgba<u8>) {
    if x < 0 || y < 0 {
        return;
    }

    let x = x as u32;
    let y = y as u32;
    if x < output.width() && y < output.height() {
        output.put_pixel(x, y, color);
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

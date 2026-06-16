use std::f64::consts::PI;
use std::io::Cursor;

use ab_glyph::{point, Font, FontArc, Glyph, PxScale, ScaleFont};

use crate::domain::capture::{PhysicalPoint, PhysicalRect};
use crate::error::{AppError, Result};

const TEXT_FONT_BYTES: &[u8] = include_bytes!("../../../assets/fonts/NotoSans-Regular.ttf");

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
    Ellipse {
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
    Line {
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
    Highlight {
        points: Vec<PhysicalPoint>,
        color: [u8; 4],
        stroke_width: u32,
    },
    Mosaic {
        rect: PhysicalRect,
        block_size: u32,
    },
    Blur {
        rect: PhysicalRect,
        radius: u32,
    },
    Text {
        position: PhysicalPoint,
        text: String,
        color: [u8; 4],
        font_size: u32,
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
        ImageAnnotation::Ellipse {
            rect,
            color,
            stroke_width,
        } => draw_ellipse_annotation(output, rect, *color, *stroke_width),
        ImageAnnotation::Arrow {
            start,
            end,
            color,
            stroke_width,
        } => draw_arrow_annotation(output, start, end, *color, *stroke_width),
        ImageAnnotation::Line {
            start,
            end,
            color,
            stroke_width,
        } => draw_line_annotation(output, start, end, *color, *stroke_width),
        ImageAnnotation::Freehand {
            points,
            color,
            stroke_width,
        } => draw_freehand_annotation(output, points, *color, *stroke_width),
        ImageAnnotation::Highlight {
            points,
            color,
            stroke_width,
        } => draw_highlight_annotation(output, points, *color, *stroke_width),
        ImageAnnotation::Mosaic { rect, block_size } => {
            draw_mosaic_annotation(output, rect, *block_size)
        }
        ImageAnnotation::Blur { rect, radius } => draw_blur_annotation(output, rect, *radius),
        ImageAnnotation::Text {
            position,
            text,
            color,
            font_size,
        } => draw_text_annotation(output, position, text, *color, *font_size),
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

fn draw_ellipse_annotation(
    output: &mut image::RgbaImage,
    rect: &PhysicalRect,
    color: [u8; 4],
    stroke_width: u32,
) {
    if rect.width == 0 || rect.height == 0 {
        return;
    }

    let color = image::Rgba(color);
    let stroke_width = stroke_width.max(1);
    let radius_x = (rect.width.saturating_sub(1) as f64) / 2.0;
    let radius_y = (rect.height.saturating_sub(1) as f64) / 2.0;
    let center_x = rect.x as f64 + radius_x;
    let center_y = rect.y as f64 + radius_y;
    if radius_x <= f64::EPSILON || radius_y <= f64::EPSILON {
        draw_rectangle_annotation(output, rect, color.0, stroke_width);
        return;
    }

    let steps = ((radius_x + radius_y) * 12.0).ceil().max(36.0) as u32;
    for step in 0..steps {
        let angle = (step as f64 / steps as f64) * 2.0 * PI;
        let x = (center_x + radius_x * angle.cos()).round() as i32;
        let y = (center_y + radius_y * angle.sin()).round() as i32;
        draw_stroked_point(output, x, y, color, stroke_width);
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

fn draw_line_annotation(
    output: &mut image::RgbaImage,
    start: &PhysicalPoint,
    end: &PhysicalPoint,
    color: [u8; 4],
    stroke_width: u32,
) {
    draw_line(
        output,
        start.x,
        start.y,
        end.x,
        end.y,
        image::Rgba(color),
        stroke_width.max(1),
    );
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

fn draw_highlight_annotation(
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
        draw_alpha_stroked_point(output, points[0].x, points[0].y, color, stroke_width);
        return;
    }

    for segment in points.windows(2) {
        let start = &segment[0];
        let end = &segment[1];
        draw_alpha_line(output, start.x, start.y, end.x, end.y, color, stroke_width);
    }
}

fn draw_mosaic_annotation(output: &mut image::RgbaImage, rect: &PhysicalRect, block_size: u32) {
    if rect.width == 0 || rect.height == 0 {
        return;
    }

    let output_width = output.width() as i64;
    let output_height = output.height() as i64;
    let left = (rect.x as i64).max(0);
    let top = (rect.y as i64).max(0);
    let right = (rect.x as i64 + rect.width as i64).min(output_width);
    let bottom = (rect.y as i64 + rect.height as i64).min(output_height);
    if left >= right || top >= bottom {
        return;
    }

    let block_size = block_size.max(1) as i64;
    let mut y = top;
    while y < bottom {
        let block_bottom = (y + block_size).min(bottom);
        let mut x = left;
        while x < right {
            let block_right = (x + block_size).min(right);
            let color = average_block_color(output, x, y, block_right, block_bottom);

            for pixel_y in y..block_bottom {
                for pixel_x in x..block_right {
                    output.put_pixel(pixel_x as u32, pixel_y as u32, color);
                }
            }

            x += block_size;
        }

        y += block_size;
    }
}

fn draw_blur_annotation(output: &mut image::RgbaImage, rect: &PhysicalRect, radius: u32) {
    if rect.width == 0 || rect.height == 0 {
        return;
    }

    let output_width = output.width() as i64;
    let output_height = output.height() as i64;
    let left = (rect.x as i64).max(0);
    let top = (rect.y as i64).max(0);
    let right = (rect.x as i64 + rect.width as i64).min(output_width);
    let bottom = (rect.y as i64 + rect.height as i64).min(output_height);
    if left >= right || top >= bottom {
        return;
    }

    let width = (right - left) as u32;
    let height = (bottom - top) as u32;
    let region = image::imageops::crop_imm(output, left as u32, top as u32, width, height)
        .to_image();
    let blurred = image::imageops::blur(&region, radius.max(1) as f32);
    image::imageops::overlay(output, &blurred, left, top);
}

fn draw_text_annotation(
    output: &mut image::RgbaImage,
    position: &PhysicalPoint,
    text: &str,
    color: [u8; 4],
    font_size: u32,
) {
    if text.is_empty() {
        return;
    }

    let Ok(font) = FontArc::try_from_slice(TEXT_FONT_BYTES) else {
        return;
    };
    let font_size = font_size.max(1) as f32;
    let scale = PxScale::from(font_size);
    let scaled_font = font.as_scaled(scale);
    let line_height = scaled_font.height().max(font_size);
    let start_x = position.x as f32;
    let mut caret = point(start_x, position.y as f32);

    for character in text.chars() {
        if character == '\n' {
            caret.x = start_x;
            caret.y += line_height;
            continue;
        }

        let glyph_id = font.glyph_id(character);
        let glyph: Glyph = glyph_id.with_scale_and_position(scale, caret);
        caret.x += scaled_font.h_advance(glyph_id);

        if let Some(outlined_glyph) = font.outline_glyph(glyph) {
            outlined_glyph.draw(|x, y, coverage| {
                let alpha = ((color[3] as f32) * coverage).round() as u8;
                if alpha == 0 {
                    return;
                }

                blend_pixel_if_in_bounds(
                    output,
                    x as i32,
                    y as i32,
                    image::Rgba([color[0], color[1], color[2], alpha]),
                );
            });
        }
    }
}

fn average_block_color(
    output: &image::RgbaImage,
    left: i64,
    top: i64,
    right: i64,
    bottom: i64,
) -> image::Rgba<u8> {
    let mut totals = [0_u64; 4];
    let mut count = 0_u64;

    for y in top..bottom {
        for x in left..right {
            let pixel = output.get_pixel(x as u32, y as u32).0;
            for channel in 0..4 {
                totals[channel] += pixel[channel] as u64;
            }
            count += 1;
        }
    }

    image::Rgba([
        (totals[0] / count) as u8,
        (totals[1] / count) as u8,
        (totals[2] / count) as u8,
        (totals[3] / count) as u8,
    ])
}

fn draw_alpha_line(
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
        draw_alpha_stroked_point(output, x, y, color, stroke_width);
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

fn draw_alpha_stroked_point(
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
            blend_pixel_if_in_bounds(output, x + offset_x, y + offset_y, color);
        }
    }
}

fn blend_pixel_if_in_bounds(output: &mut image::RgbaImage, x: i32, y: i32, color: image::Rgba<u8>) {
    if x < 0 || y < 0 {
        return;
    }

    let x = x as u32;
    let y = y as u32;
    if x >= output.width() || y >= output.height() {
        return;
    }

    let source = color.0;
    let destination = output.get_pixel(x, y).0;
    let source_alpha = source[3] as u32;
    let inverse_alpha = 255_u32.saturating_sub(source_alpha);
    let blended = image::Rgba([
        (((source[0] as u32 * source_alpha) + (destination[0] as u32 * inverse_alpha)) / 255) as u8,
        (((source[1] as u32 * source_alpha) + (destination[1] as u32 * inverse_alpha)) / 255) as u8,
        (((source[2] as u32 * source_alpha) + (destination[2] as u32 * inverse_alpha)) / 255) as u8,
        (source_alpha + (destination[3] as u32 * inverse_alpha) / 255).min(255) as u8,
    ]);
    output.put_pixel(x, y, blended);
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

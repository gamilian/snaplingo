use crate::domain::capture::{PhysicalPoint, PhysicalRect};

fn coverage(rect: &PhysicalRect, x: u32, y: u32, radius: f32) -> f32 {
    if rect.width == 0 || rect.height == 0 {
        return 0.0;
    }
    let half_w = rect.width as f32 / 2.0;
    let half_h = rect.height as f32 / 2.0;
    let r = radius.max(0.0).min(half_w).min(half_h);
    let dx = (x as f32 + 0.5 - rect.x as f32 - half_w).abs() - (half_w - r);
    let dy = (y as f32 + 0.5 - rect.y as f32 - half_h).abs() - (half_h - r);
    let distance = dx.max(0.0).hypot(dy.max(0.0)) + dx.max(dy).min(0.0) - r;
    (0.5 - distance).clamp(0.0, 1.0)
}

fn bounds(image: &image::RgbaImage, rect: &PhysicalRect, padding: u32) -> (u32, u32, u32, u32) {
    let pad = padding as i64;
    let clamp_x = |x: i64| x.clamp(0, image.width() as i64) as u32;
    let clamp_y = |y: i64| y.clamp(0, image.height() as i64) as u32;
    (
        clamp_x(rect.x as i64 - pad),
        clamp_y(rect.y as i64 - pad),
        clamp_x(rect.x as i64 + rect.width as i64 + pad),
        clamp_y(rect.y as i64 + rect.height as i64 + pad),
    )
}

fn blend(pixel: &mut image::Rgba<u8>, color: [u8; 4], amount: f32) {
    let alpha = color[3] as f32 / 255.0 * amount;
    for channel in 0..3 {
        pixel[channel] =
            (pixel[channel] as f32 * (1.0 - alpha) + color[channel] as f32 * alpha).round() as u8;
    }
}

pub(super) fn multiply(pixel: &mut image::Rgba<u8>, color: [u8; 4], opacity: f32) {
    let alpha = color[3] as f32 / 255.0 * opacity;
    for channel in 0..3 {
        pixel[channel] = (pixel[channel] as f32
            * (1.0 - alpha + alpha * color[channel] as f32 / 255.0))
            .round() as u8;
    }
}

pub(super) fn rounded_rectangle(
    output: &mut image::RgbaImage,
    rect: &PhysicalRect,
    color: [u8; 4],
    stroke: u32,
    filled: bool,
    radius: u32,
) {
    let half = (stroke.max(1) as f32 / 2.0).ceil() as u32;
    let outer = PhysicalRect {
        x: rect.x - half as i32,
        y: rect.y - half as i32,
        width: rect.width.saturating_add(half * 2),
        height: rect.height.saturating_add(half * 2),
    };
    let inner = PhysicalRect {
        x: rect.x + half as i32,
        y: rect.y + half as i32,
        width: rect.width.saturating_sub(half * 2),
        height: rect.height.saturating_sub(half * 2),
    };
    let (x0, y0, x1, y1) = bounds(output, &outer, 0);
    for y in y0..y1 {
        for x in x0..x1 {
            let pixel = output.get_pixel_mut(x, y);
            if filled {
                blend(pixel, color, coverage(rect, x, y, radius as f32));
            }
            let border = (coverage(&outer, x, y, radius as f32 + half as f32)
                - coverage(&inner, x, y, (radius as f32 - half as f32).max(0.0)))
            .max(0.0);
            blend(pixel, color, border);
        }
    }
}

pub(super) fn highlight_region(
    output: &mut image::RgbaImage,
    rect: &PhysicalRect,
    color: [u8; 4],
    radius: u32,
) {
    let (x0, y0, x1, y1) = bounds(output, rect, 0);
    for y in y0..y1 {
        for x in x0..x1 {
            multiply(
                output.get_pixel_mut(x, y),
                color,
                coverage(rect, x, y, radius as f32) * 150.0 / 255.0,
            );
        }
    }
}

pub(super) fn blur_region(
    output: &mut image::RgbaImage,
    rect: &PhysicalRect,
    radius: f32,
    corner: u32,
) {
    let radius = radius.clamp(0.1, 256.0);
    let (x0, y0, x1, y1) = bounds(output, rect, (radius * 3.0).ceil() as u32);
    if x0 >= x1 || y0 >= y1 {
        return;
    }
    let cropped = image::imageops::crop_imm(output, x0, y0, x1 - x0, y1 - y0).to_image();
    let blurred = image::imageops::blur(&cropped, radius);
    let (left, top, right, bottom) = bounds(output, rect, 0);
    for y in top..bottom {
        for x in left..right {
            let amount = coverage(rect, x, y, corner as f32);
            blend(
                output.get_pixel_mut(x, y),
                blurred.get_pixel(x - x0, y - y0).0,
                amount,
            );
        }
    }
}

pub(super) fn apply_spotlights(
    output: &mut image::RgbaImage,
    original: &image::RgbaImage,
    regions: &[(&PhysicalRect, u32)],
) {
    if regions.is_empty() {
        return;
    }
    for pixel in output.pixels_mut() {
        for channel in 0..3 {
            pixel[channel] = (pixel[channel] as f32 * 0.53).round() as u8;
        }
    }
    for (rect, radius) in regions {
        let (x0, y0, x1, y1) = bounds(output, rect, 0);
        for y in y0..y1 {
            for x in x0..x1 {
                blend(
                    output.get_pixel_mut(x, y),
                    original.get_pixel(x, y).0,
                    coverage(rect, x, y, *radius as f32),
                );
            }
        }
    }
}

pub(super) fn arrow_head(
    output: &mut image::RgbaImage,
    a: &PhysicalPoint,
    b: &PhysicalPoint,
    c: &PhysicalPoint,
    color: [u8; 4],
) {
    let edge = |p: &PhysicalPoint, q: &PhysicalPoint, x: f32, y: f32| {
        (x - p.x as f32) * (q.y - p.y) as f32 - (y - p.y as f32) * (q.x - p.x) as f32
    };
    let left = a.x.min(b.x).min(c.x).max(0) as u32;
    let top = a.y.min(b.y).min(c.y).max(0) as u32;
    let right = a.x.max(b.x).max(c.x).max(0).min(output.width() as i32 - 1) as u32;
    let bottom = a.y.max(b.y).max(c.y).max(0).min(output.height() as i32 - 1) as u32;
    for y in top..=bottom {
        for x in left..=right {
            let edges = [
                edge(a, b, x as f32 + 0.5, y as f32 + 0.5),
                edge(b, c, x as f32 + 0.5, y as f32 + 0.5),
                edge(c, a, x as f32 + 0.5, y as f32 + 0.5),
            ];
            if edges.iter().all(|value| *value >= 0.0) || edges.iter().all(|value| *value <= 0.0) {
                blend(output.get_pixel_mut(x, y), color, 1.0);
            }
        }
    }
}

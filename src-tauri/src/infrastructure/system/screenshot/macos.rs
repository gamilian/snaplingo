use super::backend::{MonitorSnapshot, ScreenRegion, ScreenshotBackend};
use crate::domain::capture::{LogicalRect, PhysicalRect};
use crate::error::AppError;
use core_graphics::display::{CGDisplay, CGRect};
use core_graphics::image::CGImage;
use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder};
use std::io::Cursor;

pub struct MacOSScreenshotBackend;

impl MacOSScreenshotBackend {
    pub fn new() -> Self {
        Self
    }
}

/// Convert CGImage to PNG bytes
fn image_to_png(cg_image: CGImage) -> Result<Vec<u8>, AppError> {
    let width = cg_image.width();
    let height = cg_image.height();
    let bytes_per_row = cg_image.bytes_per_row();
    let data = cg_image.data();

    // CGImage data is in BGRA format, convert to RGBA
    let mut rgba_data = Vec::with_capacity(width * height * 4);
    for y in 0..height {
        let row_start = y * bytes_per_row;
        for x in 0..width {
            let pixel_start = row_start + x * 4;
            let b = data.bytes()[pixel_start];
            let g = data.bytes()[pixel_start + 1];
            let r = data.bytes()[pixel_start + 2];
            let a = data.bytes()[pixel_start + 3];

            rgba_data.push(r);
            rgba_data.push(g);
            rgba_data.push(b);
            rgba_data.push(a);
        }
    }

    // Encode as PNG
    let mut png_data = Vec::new();
    let mut cursor = Cursor::new(&mut png_data);
    let encoder = PngEncoder::new(&mut cursor);

    encoder
        .write_image(
            &rgba_data,
            width as u32,
            height as u32,
            ExtendedColorType::Rgba8,
        )
        .map_err(|e| AppError::System(format!("Failed to encode PNG: {}", e)))?;

    Ok(png_data)
}

#[async_trait::async_trait]
impl ScreenshotBackend for MacOSScreenshotBackend {
    async fn capture_monitor_snapshots(&self) -> Result<Vec<MonitorSnapshot>, AppError> {
        let display = CGDisplay::main();
        let image = display
            .image()
            .ok_or_else(|| AppError::System("Failed to capture screenshot".to_string()))?;
        let width = image.width() as u32;
        let height = image.height() as u32;
        let png_data = image_to_png(image)?;

        Ok(vec![MonitorSnapshot {
            id: "primary".to_string(),
            logical_bounds: LogicalRect {
                x: 0.0,
                y: 0.0,
                width: width as f64,
                height: height as f64,
            },
            physical_bounds: PhysicalRect {
                x: 0,
                y: 0,
                width,
                height,
            },
            scale_factor: 1.0,
            png_data,
        }])
    }

    async fn capture_full_screen(&self) -> Result<Vec<u8>, AppError> {
        let display = CGDisplay::main();
        let image = display
            .image()
            .ok_or_else(|| AppError::System("Failed to capture screenshot".to_string()))?;

        image_to_png(image)
    }

    async fn capture_region(&self, region: ScreenRegion) -> Result<Vec<u8>, AppError> {
        let rect = CGRect::new(
            &core_graphics::geometry::CGPoint::new(region.x as f64, region.y as f64),
            &core_graphics::geometry::CGSize::new(region.width as f64, region.height as f64),
        );

        let image = CGDisplay::screenshot(rect, 0, 0, 0)
            .ok_or_else(|| AppError::System("Failed to capture region screenshot".to_string()))?;

        image_to_png(image)
    }
}

use crate::error::AppError;
use super::backend::{ScreenshotBackend, ScreenRegion};
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

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;
}

fn ensure_screen_capture_access() -> Result<(), AppError> {
    if unsafe { CGPreflightScreenCaptureAccess() } {
        return Ok(());
    }

    let _ = unsafe { CGRequestScreenCaptureAccess() };

    if unsafe { CGPreflightScreenCaptureAccess() } {
        return Ok(());
    }

    Err(AppError::System(
        "SnapLingo 没有当前版本的 macOS 屏幕录制权限。请在“系统设置 > 隐私与安全性 > 屏幕录制”中允许 SnapLingo，然后退出并重新打开 SnapLingo。".to_string(),
    ))
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

    encoder.write_image(
        &rgba_data,
        width as u32,
        height as u32,
        ExtendedColorType::Rgba8,
    ).map_err(|e| AppError::System(format!("Failed to encode PNG: {}", e)))?;

    Ok(png_data)
}

#[async_trait::async_trait]
impl ScreenshotBackend for MacOSScreenshotBackend {
    async fn capture_full_screen(&self) -> Result<Vec<u8>, AppError> {
        ensure_screen_capture_access()?;

        let display = CGDisplay::main();
        let image = display.image()
            .ok_or_else(|| AppError::System("Failed to capture screenshot".to_string()))?;

        image_to_png(image)
    }

    async fn capture_region(&self, region: ScreenRegion) -> Result<Vec<u8>, AppError> {
        ensure_screen_capture_access()?;

        let rect = CGRect::new(
            &core_graphics::geometry::CGPoint::new(region.x as f64, region.y as f64),
            &core_graphics::geometry::CGSize::new(region.width as f64, region.height as f64),
        );

        let image = CGDisplay::screenshot(rect, 0, 0, 0)
            .ok_or_else(|| AppError::System("Failed to capture region screenshot".to_string()))?;

        image_to_png(image)
    }
}

use crate::error::AppError;

/// Rectangle region for partial screenshot capture.
///
/// Coordinates are in PHYSICAL screen pixels (not logical/scaled pixels).
/// On high-DPI displays, the frontend must multiply logical coordinates by
/// devicePixelRatio before passing them here.
#[derive(Debug, Clone, Copy)]
pub struct ScreenRegion {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Encode an RGBA image to PNG bytes.
///
/// Platform-agnostic helper shared by backends that produce `image::RgbaImage`
/// (e.g. XCap-based Windows/Linux backends). Kept here so it can be unit-tested
/// on any platform.
#[allow(dead_code)]
pub fn rgba_image_to_png(image: image::RgbaImage) -> Result<Vec<u8>, AppError> {
    use std::io::Cursor;
    let mut buf = Cursor::new(Vec::new());
    image
        .write_to(&mut buf, image::ImageFormat::Png)
        .map_err(|e| AppError::System(format!("Failed to encode PNG: {}", e)))?;
    Ok(buf.into_inner())
}

/// Platform-agnostic screenshot backend trait
#[async_trait::async_trait]
pub trait ScreenshotBackend: Send + Sync {
    /// Capture the entire screen
    /// Returns PNG-encoded image data
    async fn capture_full_screen(&self) -> Result<Vec<u8>, AppError>;

    /// Capture a specific region of the screen
    /// Returns PNG-encoded image data
    async fn capture_region(&self, region: ScreenRegion) -> Result<Vec<u8>, AppError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgba_image_to_png() {
        let img = image::RgbaImage::new(10, 10);
        let png = rgba_image_to_png(img).unwrap();
        // PNG magic bytes
        assert_eq!(&png[0..8], &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
    }
}

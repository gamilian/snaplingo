use crate::domain::capture::{LogicalRect, PhysicalRect};
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

/// Frozen screenshot data for one monitor.
#[derive(Debug, Clone)]
pub struct MonitorSnapshot {
    pub id: String,
    pub logical_bounds: LogicalRect,
    pub physical_bounds: PhysicalRect,
    pub scale_factor: f64,
    pub png_data: Vec<u8>,
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
    /// Capture all monitor snapshots needed to start a capture session.
    /// First implementations may return only the primary monitor.
    async fn capture_monitor_snapshots(&self) -> Result<Vec<MonitorSnapshot>, AppError>;

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
        assert_eq!(
            &png[0..8],
            &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
        );
    }

    #[test]
    fn monitor_snapshot_keeps_coordinate_metadata_with_png_data() {
        let snapshot = MonitorSnapshot {
            id: "primary".to_string(),
            logical_bounds: crate::domain::capture::LogicalRect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 50.0,
            },
            physical_bounds: crate::domain::capture::PhysicalRect {
                x: 0,
                y: 0,
                width: 200,
                height: 100,
            },
            scale_factor: 2.0,
            png_data: vec![1, 2, 3],
        };

        assert_eq!(snapshot.id, "primary");
        assert_eq!(snapshot.logical_bounds.width, 100.0);
        assert_eq!(snapshot.physical_bounds.width, 200);
        assert_eq!(snapshot.scale_factor, 2.0);
        assert_eq!(snapshot.png_data, vec![1, 2, 3]);
    }
}

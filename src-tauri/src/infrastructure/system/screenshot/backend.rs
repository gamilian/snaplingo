use crate::domain::capture::{LogicalPoint, LogicalRect, PhysicalRect};
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

#[derive(Debug, Clone)]
pub struct CapturedCursor {
    pub logical_position: LogicalPoint,
    pub hotspot: LogicalPoint,
    pub image_width: u32,
    pub image_height: u32,
    pub scale_factor: f64,
    pub png_data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WindowCandidate {
    pub id: String,
    pub title: String,
    pub app_name: String,
    pub logical_bounds: LogicalRect,
}

pub fn monitor_snapshot_from_physical_geometry(
    id: String,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    scale_factor: f64,
    png_data: Vec<u8>,
) -> MonitorSnapshot {
    let scale_factor = if scale_factor > 0.0 {
        scale_factor
    } else {
        1.0
    };

    MonitorSnapshot {
        id,
        logical_bounds: LogicalRect {
            x: x as f64 / scale_factor,
            y: y as f64 / scale_factor,
            width: width as f64 / scale_factor,
            height: height as f64 / scale_factor,
        },
        physical_bounds: PhysicalRect {
            x,
            y,
            width,
            height,
        },
        scale_factor,
        png_data,
    }
}

#[cfg(any(target_os = "windows", target_os = "linux", test))]
pub fn window_candidate_from_physical_geometry(
    id: String,
    title: String,
    app_name: String,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    monitors: &[MonitorSnapshot],
) -> Option<WindowCandidate> {
    if width == 0 || height == 0 {
        return None;
    }

    let physical_bounds = PhysicalRect {
        x,
        y,
        width,
        height,
    };
    let monitor = monitors.iter().max_by_key(|monitor| {
        physical_intersection_area(&physical_bounds, &monitor.physical_bounds)
    })?;

    if physical_intersection_area(&physical_bounds, &monitor.physical_bounds) == 0 {
        return None;
    }

    let scale = monitor.scale_factor.max(1.0);
    let logical_bounds = LogicalRect {
        x: monitor.logical_bounds.x + (x - monitor.physical_bounds.x) as f64 / scale,
        y: monitor.logical_bounds.y + (y - monitor.physical_bounds.y) as f64 / scale,
        width: width as f64 / scale,
        height: height as f64 / scale,
    };

    Some(WindowCandidate {
        id,
        title,
        app_name,
        logical_bounds,
    })
}

#[cfg(any(target_os = "windows", target_os = "linux", test))]
fn physical_intersection_area(a: &PhysicalRect, b: &PhysicalRect) -> u64 {
    let left = a.x.max(b.x);
    let top = a.y.max(b.y);
    let right = (a.x as i64 + a.width as i64).min(b.x as i64 + b.width as i64);
    let bottom = (a.y as i64 + a.height as i64).min(b.y as i64 + b.height as i64);

    if right <= left as i64 || bottom <= top as i64 {
        return 0;
    }

    ((right - left as i64) * (bottom - top as i64)) as u64
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

    async fn capture_window_candidates(
        &self,
        _monitors: &[MonitorSnapshot],
    ) -> Result<Vec<WindowCandidate>, AppError> {
        Ok(Vec::new())
    }

    async fn capture_cursor(
        &self,
        _monitors: &[MonitorSnapshot],
    ) -> Result<Option<CapturedCursor>, AppError> {
        Ok(None)
    }

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

    #[test]
    fn monitor_snapshot_from_physical_geometry_derives_logical_bounds() {
        let snapshot = super::monitor_snapshot_from_physical_geometry(
            "monitor-2".to_string(),
            -2560,
            0,
            2560,
            1440,
            2.0,
            vec![4, 5, 6],
        );

        assert_eq!(snapshot.id, "monitor-2");
        assert_eq!(
            snapshot.logical_bounds,
            crate::domain::capture::LogicalRect {
                x: -1280.0,
                y: 0.0,
                width: 1280.0,
                height: 720.0,
            }
        );
        assert_eq!(
            snapshot.physical_bounds,
            crate::domain::capture::PhysicalRect {
                x: -2560,
                y: 0,
                width: 2560,
                height: 1440,
            }
        );
        assert_eq!(snapshot.scale_factor, 2.0);
        assert_eq!(snapshot.png_data, vec![4, 5, 6]);
    }

    #[test]
    fn window_candidate_from_physical_geometry_uses_matching_monitor_scale() {
        let monitors = vec![monitor_snapshot_from_physical_geometry(
            "retina".to_string(),
            -2560,
            0,
            2560,
            1440,
            2.0,
            vec![],
        )];

        let candidate = super::window_candidate_from_physical_geometry(
            "window-7".to_string(),
            "Settings".to_string(),
            "System Settings".to_string(),
            -2360,
            100,
            800,
            600,
            &monitors,
        )
        .unwrap();

        assert_eq!(candidate.id, "window-7");
        assert_eq!(candidate.title, "Settings");
        assert_eq!(
            candidate.logical_bounds,
            crate::domain::capture::LogicalRect {
                x: -1180.0,
                y: 50.0,
                width: 400.0,
                height: 300.0,
            }
        );
    }

    #[test]
    fn window_candidate_from_physical_geometry_rejects_offscreen_windows() {
        let monitors = vec![monitor_snapshot_from_physical_geometry(
            "primary".to_string(),
            0,
            0,
            1000,
            800,
            1.0,
            vec![],
        )];

        let candidate = super::window_candidate_from_physical_geometry(
            "window-8".to_string(),
            "Hidden".to_string(),
            "Example".to_string(),
            1200,
            100,
            400,
            300,
            &monitors,
        );

        assert!(candidate.is_none());
    }
}

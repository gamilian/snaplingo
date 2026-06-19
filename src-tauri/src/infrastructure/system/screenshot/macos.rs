use super::backend::{
    CapturedCursor, MonitorSnapshot, ScreenRegion, ScreenshotBackend, WindowCandidate,
};
use super::xcap_common;
use crate::domain::capture::{LogicalPoint, LogicalRect};
use crate::error::AppError;
use core_graphics::display::{CGDisplay, CGRect};
use core_graphics::image::CGImage;
use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder};
use objc2_app_kit::{NSCursor, NSEvent};
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

fn captured_cursor_from_appkit_geometry(
    mouse_x: f64,
    mouse_y_from_bottom: f64,
    primary_screen_bounds: &LogicalRect,
    hotspot_x: f64,
    hotspot_y: f64,
    image_width_points: f64,
    image_height_points: f64,
    image_width_pixels: u32,
    image_height_pixels: u32,
    png_data: Vec<u8>,
) -> Option<CapturedCursor> {
    if image_width_points <= 0.0
        || image_height_points <= 0.0
        || image_width_pixels == 0
        || image_height_pixels == 0
    {
        return None;
    }
    let logical_position =
        appkit_mouse_to_logical_point(mouse_x, mouse_y_from_bottom, primary_screen_bounds)?;

    Some(CapturedCursor {
        logical_position,
        hotspot: LogicalPoint {
            x: hotspot_x,
            y: hotspot_y,
        },
        image_width: image_width_pixels,
        image_height: image_height_pixels,
        scale_factor: image_width_pixels as f64 / image_width_points,
        png_data,
    })
}

fn appkit_mouse_to_logical_point(
    mouse_x: f64,
    mouse_y_from_bottom: f64,
    primary_screen_bounds: &LogicalRect,
) -> Option<LogicalPoint> {
    if primary_screen_bounds.height <= 0.0 {
        return None;
    }

    Some(LogicalPoint {
        x: primary_screen_bounds.x + mouse_x,
        y: primary_screen_bounds.y + primary_screen_bounds.height - mouse_y_from_bottom,
    })
}

fn primary_screen_bounds(monitors: &[MonitorSnapshot]) -> Option<LogicalRect> {
    monitors
        .iter()
        .find(|monitor| monitor.physical_bounds.x == 0 && monitor.physical_bounds.y == 0)
        .or_else(|| monitors.first())
        .map(|monitor| monitor.logical_bounds.clone())
}

fn cursor_tiff_to_png_and_dimensions(tiff_data: &[u8]) -> Result<(Vec<u8>, u32, u32), AppError> {
    let image = image::load_from_memory(tiff_data)
        .map_err(|e| AppError::System(format!("Failed to decode cursor image: {}", e)))?;
    let width = image.width();
    let height = image.height();
    let rgba_data = image.to_rgba8();

    let mut png_data = Vec::new();
    let mut cursor = Cursor::new(&mut png_data);
    let encoder = PngEncoder::new(&mut cursor);
    encoder
        .write_image(&rgba_data, width, height, ExtendedColorType::Rgba8)
        .map_err(|e| AppError::System(format!("Failed to encode cursor PNG: {}", e)))?;

    Ok((png_data, width, height))
}

#[async_trait::async_trait]
impl ScreenshotBackend for MacOSScreenshotBackend {
    async fn capture_monitor_snapshots(&self) -> Result<Vec<MonitorSnapshot>, AppError> {
        xcap_common::capture_all_monitor_snapshots()
    }

    async fn capture_window_candidates(
        &self,
        monitors: &[MonitorSnapshot],
    ) -> Result<Vec<WindowCandidate>, AppError> {
        xcap_common::capture_window_candidates(monitors)
    }

    async fn capture_cursor(
        &self,
        monitors: &[MonitorSnapshot],
    ) -> Result<Option<CapturedCursor>, AppError> {
        let Some(primary_bounds) = primary_screen_bounds(monitors) else {
            return Ok(None);
        };
        #[allow(deprecated)]
        let system_cursor = NSCursor::currentSystemCursor();
        let Some(cursor) = system_cursor else {
            return Ok(None);
        };

        let image = cursor.image();
        let size = image.size();
        let hotspot = cursor.hotSpot();
        let mouse = NSEvent::mouseLocation();
        let Some(tiff) = image.TIFFRepresentation() else {
            return Ok(None);
        };
        let (png_data, image_width, image_height) =
            cursor_tiff_to_png_and_dimensions(&tiff.to_vec())?;

        Ok(captured_cursor_from_appkit_geometry(
            mouse.x,
            mouse.y,
            &primary_bounds,
            hotspot.x,
            hotspot.y,
            size.width,
            size.height,
            image_width,
            image_height,
            png_data,
        ))
    }

    async fn capture_full_screen(&self) -> Result<Vec<u8>, AppError> {
        ensure_screen_capture_access()?;

        let display = CGDisplay::main();
        let image = display
            .image()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_captured_cursor_from_appkit_bottom_left_coordinates() {
        let primary_bounds = crate::domain::capture::LogicalRect {
            x: 0.0,
            y: 0.0,
            width: 120.0,
            height: 100.0,
        };
        let cursor = captured_cursor_from_appkit_geometry(
            18.0,
            73.0,
            &primary_bounds,
            2.0,
            3.0,
            10.0,
            12.0,
            20,
            24,
            vec![9, 8, 7],
        )
        .unwrap();

        assert_eq!(
            cursor.logical_position,
            crate::domain::capture::LogicalPoint { x: 18.0, y: 27.0 }
        );
        assert_eq!(
            cursor.hotspot,
            crate::domain::capture::LogicalPoint { x: 2.0, y: 3.0 }
        );
        assert_eq!(cursor.image_width, 20);
        assert_eq!(cursor.image_height, 24);
        assert_eq!(cursor.scale_factor, 2.0);
        assert_eq!(cursor.png_data, vec![9, 8, 7]);
    }

    #[test]
    fn rejects_cursor_geometry_without_point_size() {
        let primary_bounds = crate::domain::capture::LogicalRect {
            x: 0.0,
            y: 0.0,
            width: 120.0,
            height: 100.0,
        };
        assert!(captured_cursor_from_appkit_geometry(
            18.0,
            73.0,
            &primary_bounds,
            2.0,
            3.0,
            0.0,
            12.0,
            20,
            24,
            vec![9, 8, 7],
        )
        .is_none());
    }

    #[test]
    fn maps_appkit_mouse_position_through_primary_logical_origin() {
        let primary_bounds = crate::domain::capture::LogicalRect {
            x: -200.0,
            y: 50.0,
            width: 120.0,
            height: 80.0,
        };

        assert_eq!(
            appkit_mouse_to_logical_point(25.0, 70.0, &primary_bounds),
            Some(crate::domain::capture::LogicalPoint { x: -175.0, y: 60.0 })
        );
    }

    #[test]
    fn finds_primary_screen_bounds_from_monitor_snapshots() {
        let monitors = vec![
            MonitorSnapshot {
                id: "secondary".to_string(),
                logical_bounds: crate::domain::capture::LogicalRect {
                    x: -100.0,
                    y: 0.0,
                    width: 100.0,
                    height: 50.0,
                },
                physical_bounds: crate::domain::capture::PhysicalRect {
                    x: -200,
                    y: 0,
                    width: 200,
                    height: 100,
                },
                scale_factor: 2.0,
                png_data: Vec::new(),
            },
            MonitorSnapshot {
                id: "primary".to_string(),
                logical_bounds: crate::domain::capture::LogicalRect {
                    x: 0.0,
                    y: 0.0,
                    width: 120.0,
                    height: 80.0,
                },
                physical_bounds: crate::domain::capture::PhysicalRect {
                    x: 0,
                    y: 0,
                    width: 240,
                    height: 160,
                },
                scale_factor: 2.0,
                png_data: Vec::new(),
            },
        ];

        assert_eq!(
            primary_screen_bounds(&monitors),
            Some(crate::domain::capture::LogicalRect {
                x: 0.0,
                y: 0.0,
                width: 120.0,
                height: 80.0,
            })
        );
    }

    #[test]
    fn converts_cursor_tiff_to_png_and_dimensions() {
        let image = image::RgbaImage::from_pixel(2, 3, image::Rgba([1, 2, 3, 255]));
        let mut tiff = Cursor::new(Vec::new());
        image.write_to(&mut tiff, image::ImageFormat::Tiff).unwrap();

        let (png, width, height) = cursor_tiff_to_png_and_dimensions(&tiff.into_inner()).unwrap();

        assert_eq!(&png[..8], &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
        assert_eq!(width, 2);
        assert_eq!(height, 3);
    }
}

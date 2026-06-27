use super::backend::{
    monitor_snapshot_from_physical_geometry, CapturedCursor, MonitorSnapshot, ScreenRegion,
    ScreenshotBackend, WindowCandidate,
};
use crate::domain::capture::{LogicalPoint, LogicalRect};
use crate::error::AppError;
use core_graphics::display::{CGDisplay, CGRect};
use core_graphics::image::CGImage;
use image::codecs::png::{
    CompressionType as PngCompressionType, FilterType as PngFilterType, PngEncoder,
};
use image::{ExtendedColorType, ImageEncoder};
use objc2_app_kit::{NSCursor, NSEvent};
use std::io::Cursor;
use std::time::Instant;
use xcap::{Monitor, Window};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ScreenCaptureAccessStatus {
    initially_granted: bool,
    request_attempted: bool,
    granted_after_request: bool,
}

fn prepare_screen_capture_access() -> ScreenCaptureAccessStatus {
    prepare_screen_capture_access_with(
        || unsafe { CGPreflightScreenCaptureAccess() },
        || unsafe { CGRequestScreenCaptureAccess() },
    )
}

fn prepare_screen_capture_access_with(
    preflight: impl Fn() -> bool,
    request: impl Fn() -> bool,
) -> ScreenCaptureAccessStatus {
    let initially_granted = preflight();
    if initially_granted {
        return ScreenCaptureAccessStatus {
            initially_granted,
            request_attempted: false,
            granted_after_request: true,
        };
    }

    let _ = request();

    ScreenCaptureAccessStatus {
        initially_granted,
        request_attempted: true,
        granted_after_request: preflight(),
    }
}

fn screen_capture_unavailable_error(
    operation: &str,
    access: ScreenCaptureAccessStatus,
) -> AppError {
    AppError::System(format!(
        "无法捕获屏幕：{}。屏幕录制权限预检：初始={}，已请求={}，请求后={}。请确认“系统设置 > 隐私与安全性 > 屏幕录制”中授权的是当前运行的 SnapLingo，然后完全退出并重新打开。{}",
        operation,
        access.initially_granted,
        access.request_attempted,
        access.granted_after_request,
        current_process_hint(),
    ))
}

fn current_process_hint() -> String {
    let executable_path = std::env::current_exe();
    let executable = executable_path
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|e| format!("无法读取当前运行路径：{}", e));
    let app_bundle = executable_path
        .as_deref()
        .ok()
        .and_then(macos_app_bundle_path)
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "未在 .app bundle 中运行".to_string());

    format!("当前运行路径：{}；App Bundle：{}。", executable, app_bundle)
}

fn macos_app_bundle_path(executable_path: &std::path::Path) -> Option<std::path::PathBuf> {
    executable_path
        .ancestors()
        .find(|path| path.extension().is_some_and(|extension| extension == "app"))
        .map(std::path::Path::to_path_buf)
}

/// Convert CGImage to PNG bytes
fn image_to_png(cg_image: CGImage) -> Result<Vec<u8>, AppError> {
    let total_start = Instant::now();
    let width = cg_image.width();
    let height = cg_image.height();
    let bytes_per_row = cg_image.bytes_per_row();
    let data = cg_image.data();

    let convert_start = Instant::now();
    let rgba_data = bgra_image_data_to_rgba(data.bytes(), width, height, bytes_per_row)?;
    let convert_ms = elapsed_ms(convert_start);

    // Encode as PNG
    let encode_start = Instant::now();
    let mut png_data = Vec::new();
    let mut cursor = Cursor::new(&mut png_data);
    let encoder = PngEncoder::new_with_quality(
        &mut cursor,
        capture_png_compression_type(),
        capture_png_filter_type(),
    );

    encoder
        .write_image(
            &rgba_data,
            width as u32,
            height as u32,
            ExtendedColorType::Rgba8,
        )
        .map_err(|e| AppError::System(format!("Failed to encode PNG: {}", e)))?;
    let encode_ms = elapsed_ms(encode_start);

    log::info!(
        "[capture-perf] image_to_png width={} height={} bytes_per_row={} png_bytes={} convert_ms={:.1} encode_ms={:.1} total_ms={:.1}",
        width,
        height,
        bytes_per_row,
        png_data.len(),
        convert_ms,
        encode_ms,
        elapsed_ms(total_start),
    );

    Ok(png_data)
}

fn bgra_image_data_to_rgba(
    data: &[u8],
    width: usize,
    height: usize,
    bytes_per_row: usize,
) -> Result<Vec<u8>, AppError> {
    let row_bytes = width
        .checked_mul(4)
        .ok_or_else(|| AppError::System("Screen capture row is too wide".to_string()))?;
    let output_len = row_bytes
        .checked_mul(height)
        .ok_or_else(|| AppError::System("Screen capture image is too large".to_string()))?;
    let required_len = if height == 0 {
        0
    } else {
        bytes_per_row
            .checked_mul(height - 1)
            .and_then(|offset| offset.checked_add(row_bytes))
            .ok_or_else(|| AppError::System("Screen capture buffer is too large".to_string()))?
    };

    if bytes_per_row < row_bytes || data.len() < required_len {
        return Err(AppError::System(format!(
            "Screen capture buffer is too small: row_bytes={}, bytes_per_row={}, height={}, data_len={}",
            row_bytes,
            bytes_per_row,
            height,
            data.len(),
        )));
    }

    let mut rgba_data = vec![0; output_len];
    for y in 0..height {
        let src_start = y * bytes_per_row;
        let dst_start = y * row_bytes;
        rgba_data[dst_start..dst_start + row_bytes]
            .copy_from_slice(&data[src_start..src_start + row_bytes]);
    }

    for pixel in rgba_data.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }

    Ok(rgba_data)
}

fn capture_png_compression_type() -> PngCompressionType {
    PngCompressionType::Fast
}

fn capture_png_filter_type() -> PngFilterType {
    PngFilterType::NoFilter
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
    let encoder = PngEncoder::new_with_quality(
        &mut cursor,
        capture_png_compression_type(),
        capture_png_filter_type(),
    );
    encoder
        .write_image(&rgba_data, width, height, ExtendedColorType::Rgba8)
        .map_err(|e| AppError::System(format!("Failed to encode cursor PNG: {}", e)))?;

    Ok((png_data, width, height))
}

fn capture_visible_display_snapshots() -> Result<Vec<MonitorSnapshot>, AppError> {
    let access = prepare_screen_capture_access();

    let mut monitors = Monitor::all()
        .map_err(|e| AppError::System(format!("Failed to enumerate monitors: {}", e)))?;
    monitors.sort_by_key(|monitor| {
        if monitor.is_primary().unwrap_or(false) {
            0
        } else {
            1
        }
    });

    monitors
        .iter()
        .map(|monitor| capture_visible_display_snapshot(monitor, access))
        .collect()
}

fn capture_visible_display_snapshot(
    monitor: &Monitor,
    access: ScreenCaptureAccessStatus,
) -> Result<MonitorSnapshot, AppError> {
    let display_id = monitor
        .id()
        .map_err(|e| AppError::System(format!("Failed to read monitor id: {}", e)))?;
    let display = CGDisplay::new(display_id);
    let image = display
        .image()
        .ok_or_else(|| screen_capture_unavailable_error("显示器截图返回空图像", access))?;
    let width = image.width() as u32;
    let height = image.height() as u32;
    let png_data = image_to_png(image)?;
    let x = monitor
        .x()
        .map_err(|e| AppError::System(format!("Failed to read monitor x: {}", e)))?;
    let y = monitor
        .y()
        .map_err(|e| AppError::System(format!("Failed to read monitor y: {}", e)))?;
    let scale_factor = monitor.scale_factor().unwrap_or(1.0).max(1.0) as f64;
    let id = format!("monitor-{}", display_id);

    Ok(monitor_snapshot_from_visible_display_capture(
        id,
        x,
        y,
        width,
        height,
        scale_factor,
        png_data,
    ))
}

fn monitor_snapshot_from_visible_display_capture(
    id: String,
    logical_x: i32,
    logical_y: i32,
    physical_width: u32,
    physical_height: u32,
    scale_factor: f64,
    png_data: Vec<u8>,
) -> MonitorSnapshot {
    let scale_factor = scale_factor.max(1.0);
    monitor_snapshot_from_physical_geometry(
        id,
        logical_to_physical_origin(logical_x, scale_factor),
        logical_to_physical_origin(logical_y, scale_factor),
        physical_width,
        physical_height,
        scale_factor,
        png_data,
    )
}

fn logical_to_physical_origin(value: i32, scale_factor: f64) -> i32 {
    (value as f64 * scale_factor).round() as i32
}

fn capture_visible_window_candidates(
    monitors: &[MonitorSnapshot],
) -> Result<Vec<WindowCandidate>, AppError> {
    let windows = Window::all()
        .map_err(|e| AppError::System(format!("Failed to enumerate windows: {}", e)))?;

    let mut candidates = Vec::new();
    for (index, window) in windows.iter().enumerate() {
        let Ok(is_minimized) = window.is_minimized() else {
            continue;
        };
        if is_minimized {
            continue;
        }

        let title = window.title().unwrap_or_default();
        let app_name = window.app_name().unwrap_or_default();
        if should_skip_window_candidate(&title, &app_name) {
            continue;
        }

        let Ok(width) = window.width() else {
            continue;
        };
        let Ok(height) = window.height() else {
            continue;
        };
        if width < 2 || height < 2 {
            continue;
        }

        let Ok(x) = window.x() else {
            continue;
        };
        let Ok(y) = window.y() else {
            continue;
        };
        let id = window
            .id()
            .map(|id| format!("window-{}", id))
            .unwrap_or_else(|_| format!("window-{}", index));

        if let Some(candidate) = window_candidate_from_logical_geometry(
            id, title, app_name, x, y, width, height, monitors,
        ) {
            candidates.push(candidate);
        }
    }

    Ok(candidates)
}

fn window_candidate_from_logical_geometry(
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

    let bounds = LogicalRect {
        x: x as f64,
        y: y as f64,
        width: width as f64,
        height: height as f64,
    };
    let monitor = monitors.iter().max_by(|a, b| {
        logical_intersection_area(&bounds, &a.logical_bounds)
            .partial_cmp(&logical_intersection_area(&bounds, &b.logical_bounds))
            .unwrap_or(std::cmp::Ordering::Equal)
    })?;

    if logical_intersection_area(&bounds, &monitor.logical_bounds) <= 0.0 {
        return None;
    }

    Some(WindowCandidate {
        id,
        title,
        app_name,
        logical_bounds: bounds,
    })
}

fn logical_intersection_area(a: &LogicalRect, b: &LogicalRect) -> f64 {
    let left = a.x.max(b.x);
    let top = a.y.max(b.y);
    let right = (a.x + a.width).min(b.x + b.width);
    let bottom = (a.y + a.height).min(b.y + b.height);

    if right <= left || bottom <= top {
        return 0.0;
    }

    (right - left) * (bottom - top)
}

fn elapsed_ms(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}

fn should_skip_window_candidate(title: &str, app_name: &str) -> bool {
    let title = title.to_ascii_lowercase();
    let app_name = app_name.to_ascii_lowercase();

    title == "snaplingo capture"
        || title == "snaplingo pin"
        || app_name == "snaplingo capture"
        || app_name == "snaplingo pin"
}

#[async_trait::async_trait]
impl ScreenshotBackend for MacOSScreenshotBackend {
    async fn capture_monitor_snapshots(&self) -> Result<Vec<MonitorSnapshot>, AppError> {
        capture_visible_display_snapshots()
    }

    async fn capture_window_candidates(
        &self,
        monitors: &[MonitorSnapshot],
    ) -> Result<Vec<WindowCandidate>, AppError> {
        capture_visible_window_candidates(monitors)
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

    fn current_cursor_position(
        &self,
        monitors: &[MonitorSnapshot],
    ) -> Result<Option<LogicalPoint>, AppError> {
        let Some(primary_bounds) = primary_screen_bounds(monitors) else {
            return Ok(None);
        };
        let mouse = NSEvent::mouseLocation();

        Ok(appkit_mouse_to_logical_point(
            mouse.x,
            mouse.y,
            &primary_bounds,
        ))
    }

    async fn capture_full_screen(&self) -> Result<Vec<u8>, AppError> {
        let access = prepare_screen_capture_access();

        let display = CGDisplay::main();
        let image = display
            .image()
            .ok_or_else(|| screen_capture_unavailable_error("全屏截图返回空图像", access))?;

        image_to_png(image)
    }

    async fn capture_region(&self, region: ScreenRegion) -> Result<Vec<u8>, AppError> {
        let access = prepare_screen_capture_access();

        let rect = CGRect::new(
            &core_graphics::geometry::CGPoint::new(region.x as f64, region.y as f64),
            &core_graphics::geometry::CGSize::new(region.width as f64, region.height as f64),
        );

        let image = CGDisplay::screenshot(rect, 0, 0, 0)
            .ok_or_else(|| screen_capture_unavailable_error("区域截图返回空图像", access))?;

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

    #[test]
    fn converts_bgra_display_rows_to_contiguous_rgba() {
        let rgba = bgra_image_data_to_rgba(
            &[
                3, 2, 1, 4, 7, 6, 5, 8, 99, 99, 11, 10, 9, 12, 15, 14, 13, 16, 88, 88,
            ],
            2,
            2,
            10,
        )
        .unwrap();

        assert_eq!(
            rgba,
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]
        );
    }

    #[test]
    fn capture_session_png_encoder_uses_low_latency_options() {
        assert_eq!(
            capture_png_compression_type(),
            image::codecs::png::CompressionType::Fast
        );
        assert_eq!(
            capture_png_filter_type(),
            image::codecs::png::FilterType::NoFilter
        );
    }

    #[test]
    fn screen_capture_access_probe_keeps_running_when_preflight_stays_false() {
        let status = prepare_screen_capture_access_with(|| false, || false);

        assert_eq!(
            status,
            ScreenCaptureAccessStatus {
                initially_granted: false,
                request_attempted: true,
                granted_after_request: false,
            }
        );
    }

    #[test]
    fn capture_failure_error_includes_permission_probe_state() {
        let error = screen_capture_unavailable_error(
            "测试截图",
            ScreenCaptureAccessStatus {
                initially_granted: false,
                request_attempted: true,
                granted_after_request: false,
            },
        );
        let message = match error {
            AppError::System(message) => message,
            other => panic!("unexpected error: {other:?}"),
        };

        assert!(message.contains("测试截图"));
        assert!(message.contains("屏幕录制权限预检：初始=false，已请求=true，请求后=false"));
        assert!(message.contains("当前运行路径"));
        assert!(message.contains("App Bundle"));
    }

    #[test]
    fn builds_monitor_snapshot_from_logical_display_origin_and_physical_capture() {
        let snapshot = monitor_snapshot_from_visible_display_capture(
            "monitor-42".to_string(),
            -1280,
            0,
            2560,
            1440,
            2.0,
            vec![1, 2, 3],
        );

        assert_eq!(snapshot.id, "monitor-42");
        assert_eq!(snapshot.png_data, vec![1, 2, 3]);
        assert_eq!(
            snapshot.physical_bounds,
            crate::domain::capture::PhysicalRect {
                x: -2560,
                y: 0,
                width: 2560,
                height: 1440,
            }
        );
        assert_eq!(
            snapshot.logical_bounds,
            crate::domain::capture::LogicalRect {
                x: -1280.0,
                y: 0.0,
                width: 1280.0,
                height: 720.0,
            }
        );
    }

    #[test]
    fn builds_window_candidate_from_logical_window_geometry() {
        let monitors = vec![monitor_snapshot_from_visible_display_capture(
            "monitor-42".to_string(),
            -1280,
            0,
            2560,
            1440,
            2.0,
            Vec::new(),
        )];

        let candidate = window_candidate_from_logical_geometry(
            "window-7".to_string(),
            "Editor".to_string(),
            "Code".to_string(),
            -1200,
            100,
            400,
            300,
            &monitors,
        )
        .unwrap();

        assert_eq!(
            candidate.logical_bounds,
            crate::domain::capture::LogicalRect {
                x: -1200.0,
                y: 100.0,
                width: 400.0,
                height: 300.0,
            }
        );
    }
}

use super::{geometry::logical_rect_from_physical, xcap_common};
use crate::application::CaptureSessionSource;
use crate::domain::capture::{
    CapturedCursor, ControlCandidate, LogicalPoint, LogicalRect, MonitorLayout, MonitorSnapshot,
    ScreenRegion, WindowCandidate,
};
use crate::error::AppError;
use windows::Win32::Foundation::{HANDLE, POINT};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, SelectObject, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HGDIOBJ,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CopyIcon, DestroyIcon, DrawIconEx, GetCursorInfo, GetCursorPos, GetIconInfo, GetSystemMetrics,
    CURSORINFO, CURSOR_SHOWING, DI_NORMAL, HICON, ICONINFO, SM_CXCURSOR, SM_CYCURSOR,
};
use windows::Win32::{
    System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
        COINIT_MULTITHREADED,
    },
    UI::Accessibility::{CUIAutomation, IUIAutomation},
};
use xcap::Monitor;

/// Windows screenshot backend using the cross-platform XCap crate.
pub struct WindowsCaptureSessionSource;

impl WindowsCaptureSessionSource {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl CaptureSessionSource for WindowsCaptureSessionSource {
    async fn capture_monitor_snapshots(&self) -> Result<Vec<MonitorSnapshot>, AppError> {
        let mut snapshots = xcap_common::capture_all_monitor_snapshots()?;
        normalize_windows_snapshot_coordinates(&mut snapshots);
        Ok(snapshots)
    }

    async fn capture_monitor_snapshot(
        &self,
        monitor_id: &str,
    ) -> Result<MonitorSnapshot, AppError> {
        self.capture_monitor_snapshots()
            .await?
            .into_iter()
            .find(|snapshot| snapshot.id == monitor_id)
            .ok_or_else(|| AppError::System(format!("Capture monitor not found: {monitor_id}")))
    }

    async fn capture_monitor_layouts(&self) -> Result<Vec<MonitorLayout>, AppError> {
        let mut layouts = xcap_common::capture_all_monitor_layouts()?;
        normalize_windows_layout_coordinates(&mut layouts);
        Ok(layouts)
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
        capture_windows_cursor(monitors)
    }

    async fn capture_control_candidate(
        &self,
        point: &LogicalPoint,
    ) -> Result<Option<ControlCandidate>, AppError> {
        let point = point.clone();
        tokio::task::spawn_blocking(move || capture_control_candidate_at(&point))
            .await
            .map_err(|error| AppError::System(format!("Control detection task failed: {error}")))?
    }

    async fn capture_region(&self, region: ScreenRegion) -> Result<Vec<u8>, AppError> {
        xcap_common::capture_region_png(region)
    }

    fn current_cursor_position(
        &self,
        monitors: &[MonitorSnapshot],
    ) -> Result<Option<LogicalPoint>, AppError> {
        let mut point = POINT::default();
        unsafe { GetCursorPos(&mut point) }.map_err(|error| {
            AppError::System(format!("Failed to read cursor position: {error}"))
        })?;
        Ok(super::geometry::logical_point_from_physical_geometry(
            point.x, point.y, monitors,
        ))
    }
}

fn capture_control_candidate_at(
    point: &LogicalPoint,
) -> Result<Option<ControlCandidate>, AppError> {
    let scale_factor = windows_primary_scale_factor()?;
    let physical_point = POINT {
        x: (point.x * scale_factor).round() as i32,
        y: (point.y * scale_factor).round() as i32,
    };
    let _com = ComApartment::initialize()?;
    let automation: IUIAutomation =
        unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER) }.map_err(
            |error| AppError::System(format!("Failed to start Windows UI Automation: {error}")),
        )?;
    unsafe { automation.SetConnectionTimeout(200) }.map_err(|error| {
        AppError::System(format!(
            "Failed to configure Windows UI Automation: {error}"
        ))
    })?;
    let element = unsafe { automation.ElementFromPoint(physical_point) }
        .map_err(|error| AppError::System(format!("Failed to find Windows UI element: {error}")))?;
    let process_id = unsafe { element.CurrentProcessId() }.map_err(|error| {
        AppError::System(format!(
            "Failed to read Windows UI element process: {error}"
        ))
    })?;
    if process_id == std::process::id() as i32 {
        return Err(AppError::System(
            "Windows UI Automation selected the capture overlay".to_string(),
        ));
    }
    let bounds = unsafe { element.CurrentBoundingRectangle() }.map_err(|error| {
        AppError::System(format!("Failed to read Windows UI element bounds: {error}"))
    })?;

    Ok(control_candidate_from_physical_bounds(
        bounds.left,
        bounds.top,
        bounds.right,
        bounds.bottom,
        scale_factor,
    ))
}

struct ComApartment;

impl ComApartment {
    fn initialize() -> Result<Self, AppError> {
        unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.map_err(|error| {
            AppError::System(format!(
                "Failed to initialize COM for UI Automation: {error}"
            ))
        })?;
        Ok(Self)
    }
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}

fn windows_primary_scale_factor() -> Result<f64, AppError> {
    Monitor::all()
        .map_err(|error| {
            AppError::System(format!("Failed to enumerate Windows monitors: {error}"))
        })?
        .into_iter()
        .find(|monitor| monitor.is_primary().unwrap_or(false))
        .map(|monitor| monitor.scale_factor().unwrap_or(1.0).max(1.0) as f64)
        .ok_or_else(|| AppError::System("No primary Windows monitor found".to_string()))
}

fn control_candidate_from_physical_bounds(
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    scale_factor: f64,
) -> Option<ControlCandidate> {
    if right - left < 2 || bottom - top < 2 {
        return None;
    }
    let scale_factor = scale_factor.max(1.0);
    let bounds = LogicalRect {
        x: left as f64 / scale_factor,
        y: top as f64 / scale_factor,
        width: (right - left) as f64 / scale_factor,
        height: (bottom - top) as f64 / scale_factor,
    };

    Some(ControlCandidate {
        id: format!(
            "control-{:.0}-{:.0}-{:.0}-{:.0}",
            bounds.x, bounds.y, bounds.width, bounds.height
        ),
        logical_bounds: bounds,
    })
}

fn capture_windows_cursor(
    monitors: &[MonitorSnapshot],
) -> Result<Option<CapturedCursor>, AppError> {
    let mut cursor = CURSORINFO {
        cbSize: std::mem::size_of::<CURSORINFO>() as u32,
        ..Default::default()
    };
    unsafe { GetCursorInfo(&mut cursor) }
        .map_err(|error| AppError::System(format!("Failed to read cursor: {error}")))?;
    if cursor.flags.0 & CURSOR_SHOWING.0 == 0 {
        return Ok(None);
    }

    let Some(logical_position) = super::geometry::logical_point_from_physical_geometry(
        cursor.ptScreenPos.x,
        cursor.ptScreenPos.y,
        monitors,
    ) else {
        return Ok(None);
    };
    let scale_factor = monitors
        .iter()
        .find(|monitor| {
            let bounds = &monitor.physical_bounds;
            cursor.ptScreenPos.x >= bounds.x
                && cursor.ptScreenPos.y >= bounds.y
                && cursor.ptScreenPos.x < bounds.x.saturating_add_unsigned(bounds.width)
                && cursor.ptScreenPos.y < bounds.y.saturating_add_unsigned(bounds.height)
        })
        .map(|monitor| monitor.scale_factor.max(1.0))
        .unwrap_or(1.0);

    let icon = unsafe { CopyIcon(HICON(cursor.hCursor.0)) }
        .map_err(|error| AppError::System(format!("Failed to copy cursor icon: {error}")))?;
    let result = cursor_icon_png(icon, scale_factor).map(|(png_data, width, height, hotspot)| {
        CapturedCursor {
            logical_position,
            hotspot,
            image_width: width,
            image_height: height,
            scale_factor,
            png_data,
        }
    });
    let _ = unsafe { DestroyIcon(icon) };
    result.map(Some)
}

fn cursor_icon_png(
    icon: HICON,
    scale_factor: f64,
) -> Result<(Vec<u8>, u32, u32, LogicalPoint), AppError> {
    let mut icon_info = ICONINFO::default();
    unsafe { GetIconInfo(icon, &mut icon_info) }
        .map_err(|error| AppError::System(format!("Failed to inspect cursor icon: {error}")))?;

    let result = cursor_icon_png_with_info(icon, &icon_info, scale_factor);
    delete_icon_info_bitmaps(&icon_info);
    result
}

fn cursor_icon_png_with_info(
    icon: HICON,
    icon_info: &ICONINFO,
    scale_factor: f64,
) -> Result<(Vec<u8>, u32, u32, LogicalPoint), AppError> {
    let width = unsafe { GetSystemMetrics(SM_CXCURSOR) }.max(1);
    let height = unsafe { GetSystemMetrics(SM_CYCURSOR) }.max(1);
    let mut bitmap_info = BITMAPINFO::default();
    bitmap_info.bmiHeader = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: width,
        biHeight: -height,
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB.0,
        ..Default::default()
    };

    let dc = unsafe { CreateCompatibleDC(None) };
    if dc.0 == 0 {
        return Err(AppError::System(
            "Failed to create cursor drawing context".to_string(),
        ));
    }
    let mut bits = std::ptr::null_mut();
    let bitmap = match unsafe {
        CreateDIBSection(dc, &bitmap_info, DIB_RGB_COLORS, &mut bits, HANDLE(0), 0)
    } {
        Ok(bitmap) => bitmap,
        Err(error) => {
            unsafe {
                DeleteDC(dc);
            }
            return Err(AppError::System(format!(
                "Failed to create cursor bitmap: {error}"
            )));
        }
    };
    let previous = unsafe { SelectObject(dc, HGDIOBJ(bitmap.0)) };
    let draw_result = unsafe { DrawIconEx(dc, 0, 0, icon, width, height, 0, None, DI_NORMAL) };
    if let Err(error) = draw_result {
        unsafe {
            SelectObject(dc, previous);
            DeleteObject(HGDIOBJ(bitmap.0));
            DeleteDC(dc);
        }
        return Err(AppError::System(format!(
            "Failed to draw cursor icon: {error}"
        )));
    }

    let rgba = (|| {
        let pixel_len = (width as usize)
            .checked_mul(height as usize)
            .and_then(|length| length.checked_mul(4))
            .ok_or_else(|| AppError::System("Cursor image is too large".to_string()))?;
        Ok::<_, AppError>(
            unsafe { std::slice::from_raw_parts(bits.cast::<u8>(), pixel_len) }.to_vec(),
        )
    })();
    unsafe {
        SelectObject(dc, previous);
        DeleteObject(HGDIOBJ(bitmap.0));
        DeleteDC(dc);
    }
    let mut rgba = rgba?;
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    let image = image::RgbaImage::from_raw(width as u32, height as u32, rgba)
        .ok_or_else(|| AppError::System("Failed to create cursor image".to_string()))?;

    Ok((
        super::image::rgba_image_to_png(image)?,
        width as u32,
        height as u32,
        LogicalPoint {
            x: icon_info.xHotspot as f64 / scale_factor.max(1.0),
            y: icon_info.yHotspot as f64 / scale_factor.max(1.0),
        },
    ))
}

fn delete_icon_info_bitmaps(icon_info: &ICONINFO) {
    unsafe {
        if icon_info.hbmColor.0 != 0 {
            let _ = DeleteObject(HGDIOBJ(icon_info.hbmColor.0));
        }
        if icon_info.hbmMask.0 != 0 {
            let _ = DeleteObject(HGDIOBJ(icon_info.hbmMask.0));
        }
    }
}

fn normalize_windows_snapshot_coordinates(monitors: &mut [MonitorSnapshot]) {
    let scale = monitors
        .first()
        .map(|monitor| monitor.scale_factor.max(1.0))
        .unwrap_or(1.0);
    for monitor in monitors {
        monitor.logical_bounds = logical_rect_from_physical(
            monitor.physical_bounds.x,
            monitor.physical_bounds.y,
            monitor.physical_bounds.width,
            monitor.physical_bounds.height,
            scale,
        );
        monitor.scale_factor = scale;
    }
}

fn normalize_windows_layout_coordinates(monitors: &mut [MonitorLayout]) {
    let scale = monitors
        .first()
        .map(|monitor| monitor.scale_factor.max(1.0))
        .unwrap_or(1.0);
    for monitor in monitors {
        monitor.logical_bounds = logical_rect_from_physical(
            monitor.physical_bounds.x,
            monitor.physical_bounds.y,
            monitor.physical_bounds.width,
            monitor.physical_bounds.height,
            scale,
        );
        monitor.scale_factor = scale;
    }
}

#[cfg(test)]
mod tests {
    use super::control_candidate_from_physical_bounds;
    use crate::domain::capture::LogicalRect;

    #[test]
    fn converts_ui_automation_physical_bounds_to_capture_coordinates() {
        let candidate = control_candidate_from_physical_bounds(300, 150, 900, 450, 1.5).unwrap();

        assert_eq!(candidate.id, "control-200-100-400-200");
        assert_eq!(
            candidate.logical_bounds,
            LogicalRect {
                x: 200.0,
                y: 100.0,
                width: 400.0,
                height: 200.0,
            }
        );
    }

    #[test]
    fn rejects_empty_ui_automation_bounds() {
        assert!(control_candidate_from_physical_bounds(10, 10, 11, 100, 1.0).is_none());
        assert!(control_candidate_from_physical_bounds(10, 10, 100, 11, 1.0).is_none());
    }
}

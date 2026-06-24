#[cfg(any(target_os = "windows", target_os = "linux"))]
use super::backend::ScreenRegion;
#[cfg(any(target_os = "windows", target_os = "linux"))]
use super::backend::{
    monitor_snapshot_from_physical_geometry, rgba_image_to_png,
    window_candidate_from_physical_geometry, MonitorSnapshot, WindowCandidate,
};
#[cfg(any(target_os = "windows", target_os = "linux"))]
use crate::error::AppError;
#[cfg(any(target_os = "windows", target_os = "linux"))]
use xcap::{Monitor, Window};

/// Get the primary monitor (not just the first enumerated one).
#[cfg(any(target_os = "windows", target_os = "linux"))]
fn get_primary_monitor() -> Result<Monitor, AppError> {
    Monitor::all()
        .map_err(|e| with_platform_hint(format!("Failed to enumerate monitors: {}", e)))?
        .into_iter()
        .find(|monitor| monitor.is_primary().unwrap_or(false))
        .ok_or_else(|| AppError::System("No primary monitor found".to_string()))
}

/// Wrap an error message with a platform-specific troubleshooting hint.
#[cfg(any(target_os = "windows", target_os = "linux"))]
fn with_platform_hint(msg: String) -> AppError {
    let hint = if cfg!(target_os = "linux") {
        " (Linux: ensure xdg-desktop-portal is running for Wayland support)"
    } else {
        ""
    };
    AppError::System(format!("{}{}", msg, hint))
}

/// Capture the primary monitor's full screen as PNG bytes.
#[cfg(any(target_os = "windows", target_os = "linux"))]
pub fn capture_full_screen_png() -> Result<Vec<u8>, AppError> {
    let primary = get_primary_monitor()?;
    let image = primary
        .capture_image()
        .map_err(|e| with_platform_hint(format!("Screenshot failed: {}", e)))?;
    rgba_image_to_png(image)
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
pub fn capture_all_monitor_snapshots() -> Result<Vec<MonitorSnapshot>, AppError> {
    let mut monitors = Monitor::all()
        .map_err(|e| with_platform_hint(format!("Failed to enumerate monitors: {}", e)))?;
    monitors.sort_by_key(|monitor| {
        if monitor.is_primary().unwrap_or(false) {
            0
        } else {
            1
        }
    });

    monitors
        .iter()
        .enumerate()
        .map(|(index, monitor)| capture_monitor_snapshot(monitor, index))
        .collect()
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
pub fn capture_window_candidates(
    monitors: &[MonitorSnapshot],
) -> Result<Vec<WindowCandidate>, AppError> {
    let windows = Window::all()
        .map_err(|e| with_platform_hint(format!("Failed to enumerate windows: {}", e)))?;

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

        if let Some(candidate) = window_candidate_from_physical_geometry(
            id, title, app_name, x, y, width, height, monitors,
        ) {
            candidates.push(candidate);
        }
    }

    Ok(candidates)
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn should_skip_window_candidate(title: &str, app_name: &str) -> bool {
    let title = title.to_ascii_lowercase();
    let app_name = app_name.to_ascii_lowercase();

    title == "snaplingo capture"
        || title == "snaplingo pin"
        || app_name == "snaplingo capture"
        || app_name == "snaplingo pin"
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn capture_monitor_snapshot(
    monitor: &Monitor,
    fallback_index: usize,
) -> Result<MonitorSnapshot, AppError> {
    let image = monitor
        .capture_image()
        .map_err(|e| with_platform_hint(format!("Screenshot failed: {}", e)))?;
    let width = image.width();
    let height = image.height();
    let png_data = rgba_image_to_png(image)?;
    let x = monitor
        .x()
        .map_err(|e| with_platform_hint(format!("Failed to read monitor x: {}", e)))?;
    let y = monitor
        .y()
        .map_err(|e| with_platform_hint(format!("Failed to read monitor y: {}", e)))?;
    let scale_factor = monitor.scale_factor().unwrap_or(1.0).max(1.0) as f64;
    let id = monitor
        .id()
        .map(|id| format!("monitor-{}", id))
        .unwrap_or_else(|_| format!("monitor-{}", fallback_index));

    Ok(monitor_snapshot_from_physical_geometry(
        id,
        x,
        y,
        width,
        height,
        scale_factor,
        png_data,
    ))
}

/// Capture a region of the primary monitor as PNG bytes.
///
/// Uses XCap's native `capture_region`, which handles global-to-local
/// coordinate mapping internally.
#[cfg(any(target_os = "windows", target_os = "linux"))]
pub fn capture_region_png(region: ScreenRegion) -> Result<Vec<u8>, AppError> {
    let primary = get_primary_monitor()?;
    let image = primary
        .capture_region(
            region.x.max(0) as u32,
            region.y.max(0) as u32,
            region.width,
            region.height,
        )
        .map_err(|e| with_platform_hint(format!("Region capture failed: {}", e)))?;
    rgba_image_to_png(image)
}

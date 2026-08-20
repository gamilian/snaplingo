#[cfg(any(target_os = "windows", target_os = "linux"))]
use super::geometry::{
    monitor_layout_from_physical_geometry, monitor_snapshot_from_physical_geometry,
    window_candidate_from_physical_geometry,
};
#[cfg(any(target_os = "windows", target_os = "linux"))]
use super::image::rgba_image_to_png;
#[cfg(any(target_os = "windows", target_os = "linux"))]
use crate::domain::capture::{MonitorLayout, MonitorSnapshot, WindowCandidate};
#[cfg(any(target_os = "windows", target_os = "linux", test))]
use crate::domain::capture::{PhysicalRect, ScreenRegion};
#[cfg(any(target_os = "windows", target_os = "linux"))]
use crate::error::AppError;
#[cfg(any(target_os = "windows", target_os = "linux"))]
use xcap::{Monitor, Window};

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
pub fn capture_monitor_snapshot_by_id(monitor_id: &str) -> Result<MonitorSnapshot, AppError> {
    let mut monitors = Monitor::all()
        .map_err(|e| with_platform_hint(format!("Failed to enumerate monitors: {}", e)))?;
    monitors.sort_by_key(|monitor| {
        if monitor.is_primary().unwrap_or(false) {
            0
        } else {
            1
        }
    });
    for (index, monitor) in monitors.iter().enumerate() {
        let id = monitor
            .id()
            .map(|id| format!("monitor-{id}"))
            .unwrap_or_else(|_| format!("monitor-{index}"));
        if id == monitor_id {
            return capture_monitor_snapshot(monitor, index);
        }
    }
    Err(AppError::System(format!(
        "Capture monitor not found: {monitor_id}"
    )))
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
pub fn capture_all_monitor_layouts() -> Result<Vec<MonitorLayout>, AppError> {
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
        .map(|(index, monitor)| capture_monitor_layout(monitor, index))
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

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn capture_monitor_layout(
    monitor: &Monitor,
    fallback_index: usize,
) -> Result<MonitorLayout, AppError> {
    let x = monitor
        .x()
        .map_err(|e| with_platform_hint(format!("Failed to read monitor x: {}", e)))?;
    let y = monitor
        .y()
        .map_err(|e| with_platform_hint(format!("Failed to read monitor y: {}", e)))?;
    let width = monitor
        .width()
        .map_err(|e| with_platform_hint(format!("Failed to read monitor width: {}", e)))?;
    let height = monitor
        .height()
        .map_err(|e| with_platform_hint(format!("Failed to read monitor height: {}", e)))?;
    let scale_factor = monitor.scale_factor().unwrap_or(1.0).max(1.0) as f64;
    let id = monitor
        .id()
        .map(|id| format!("monitor-{}", id))
        .unwrap_or_else(|_| format!("monitor-{}", fallback_index));

    Ok(monitor_layout_from_physical_geometry(
        id,
        x,
        y,
        width,
        height,
        scale_factor,
    ))
}

/// Capture a region contained by one monitor as PNG bytes.
#[cfg(any(target_os = "windows", target_os = "linux"))]
pub fn capture_region_png(region: ScreenRegion) -> Result<Vec<u8>, AppError> {
    let monitors = Monitor::all()
        .map_err(|e| with_platform_hint(format!("Failed to enumerate monitors: {e}")))?;

    for monitor in monitors {
        let bounds = PhysicalRect {
            x: monitor
                .x()
                .map_err(|e| with_platform_hint(format!("Failed to read monitor x: {e}")))?,
            y: monitor
                .y()
                .map_err(|e| with_platform_hint(format!("Failed to read monitor y: {e}")))?,
            width: monitor
                .width()
                .map_err(|e| with_platform_hint(format!("Failed to read monitor width: {e}")))?,
            height: monitor
                .height()
                .map_err(|e| with_platform_hint(format!("Failed to read monitor height: {e}")))?,
        };
        let Some(local_region) = local_region_for_monitor(region, &bounds) else {
            continue;
        };

        let image = monitor
            .capture_region(
                local_region.x as u32,
                local_region.y as u32,
                local_region.width,
                local_region.height,
            )
            .map_err(|e| with_platform_hint(format!("Region capture failed: {e}")))?;
        return rgba_image_to_png(image);
    }

    Err(AppError::System(
        "Capture region must be contained by one monitor".to_string(),
    ))
}

#[cfg(any(target_os = "windows", target_os = "linux", test))]
fn local_region_for_monitor(region: ScreenRegion, monitor: &PhysicalRect) -> Option<ScreenRegion> {
    let right = region.x.checked_add_unsigned(region.width)?;
    let bottom = region.y.checked_add_unsigned(region.height)?;
    let monitor_right = monitor.x.checked_add_unsigned(monitor.width)?;
    let monitor_bottom = monitor.y.checked_add_unsigned(monitor.height)?;

    (region.x >= monitor.x
        && region.y >= monitor.y
        && right <= monitor_right
        && bottom <= monitor_bottom)
        .then_some(ScreenRegion {
            x: region.x - monitor.x,
            y: region.y - monitor.y,
            width: region.width,
            height: region.height,
        })
}

#[cfg(test)]
mod tests {
    use super::local_region_for_monitor;
    use crate::domain::capture::{PhysicalRect, ScreenRegion};

    #[test]
    fn converts_global_secondary_monitor_coordinates_to_local_coordinates() {
        let local = local_region_for_monitor(
            ScreenRegion {
                x: -1800,
                y: 50,
                width: 400,
                height: 300,
            },
            &PhysicalRect {
                x: -1920,
                y: 0,
                width: 1920,
                height: 1080,
            },
        )
        .unwrap();

        assert_eq!(
            (local.x, local.y, local.width, local.height),
            (120, 50, 400, 300)
        );
    }

    #[test]
    fn rejects_regions_that_span_monitors() {
        assert!(local_region_for_monitor(
            ScreenRegion {
                x: -100,
                y: 0,
                width: 200,
                height: 100,
            },
            &PhysicalRect {
                x: -1920,
                y: 0,
                width: 1920,
                height: 1080,
            },
        )
        .is_none());
    }
}

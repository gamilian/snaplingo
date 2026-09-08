use crate::domain::capture::{LogicalRect, MonitorSnapshot};

/// Native adapters supply their monitor geometry; the session owns the shared
/// coordinate convention used by frozen pixels, selection, and its window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureCoordinatePolicy {
    NativeLogical,
    /// One spanning WebView uses the primary monitor's scale for the desktop.
    /// The source must return the primary monitor first.
    PrimaryMonitorScale,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaptureWindowGeometry {
    pub bounds: LogicalRect,
    /// Physical pixels per CSS pixel for a desktop with one uniform scale.
    /// Native logical coordinate systems retain their own window conversion.
    pub desktop_scale: Option<f64>,
}

impl CaptureCoordinatePolicy {
    pub(crate) fn normalize(self, monitors: &mut [MonitorSnapshot]) -> Option<f64> {
        if self == CaptureCoordinatePolicy::NativeLogical {
            return None;
        }

        let scale = monitors.first()?.scale_factor.max(1.0);
        for monitor in monitors {
            let physical = &monitor.physical_bounds;
            monitor.scale_factor = scale;
            monitor.logical_bounds = LogicalRect {
                x: physical.x as f64 / scale,
                y: physical.y as f64 / scale,
                width: physical.width as f64 / scale,
                height: physical.height as f64 / scale,
            };
        }
        Some(scale)
    }
}

pub(super) fn capture_window_geometry(
    monitors: &[MonitorSnapshot],
    desktop_scale: Option<f64>,
) -> Option<CaptureWindowGeometry> {
    let first = monitors.first()?.logical_bounds.clone();
    let bounds = monitors.iter().skip(1).fold(first, |bounds, monitor| {
        let monitor = &monitor.logical_bounds;
        let left = bounds.x.min(monitor.x);
        let top = bounds.y.min(monitor.y);
        let right = (bounds.x + bounds.width).max(monitor.x + monitor.width);
        let bottom = (bounds.y + bounds.height).max(monitor.y + monitor.height);
        LogicalRect {
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
        }
    });
    Some(CaptureWindowGeometry {
        bounds,
        desktop_scale,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::capture::PhysicalRect;

    #[test]
    fn native_logical_geometry_keeps_per_monitor_scale_and_origins() {
        let mut snapshots = vec![
            monitor("primary", 0, 0, 200, 200, 2.0),
            monitor("left", -150, -150, 150, 150, 1.5),
        ];
        snapshots[1].logical_bounds.x = -100.0;
        snapshots[1].logical_bounds.y = -100.0;
        let original = snapshots[1].logical_bounds.clone();
        let scale = CaptureCoordinatePolicy::NativeLogical.normalize(&mut snapshots);
        assert_eq!(scale, None);
        assert_eq!(snapshots[1].logical_bounds, original);
        assert_eq!(snapshots[1].scale_factor, 1.5);
        assert_eq!(
            capture_window_geometry(&snapshots, scale).unwrap().bounds,
            LogicalRect {
                x: -100.0,
                y: -100.0,
                width: 200.0,
                height: 200.0
            }
        );
    }

    #[test]
    fn uniform_geometry_keeps_negative_origins_and_mixed_dpi_displays_contiguous() {
        let mut snapshots = vec![
            monitor("primary", 0, 0, 3840, 2160, 2.0),
            monitor("right", 3840, 0, 1920, 1080, 1.0),
            monitor("left", -2560, 0, 2560, 1440, 1.25),
            monitor("top", 0, -1080, 1920, 1080, 1.5),
        ];
        let scale = CaptureCoordinatePolicy::PrimaryMonitorScale.normalize(&mut snapshots);
        assert_eq!(scale, Some(2.0));
        assert_eq!(
            snapshots[1].logical_bounds.x,
            snapshots[0].logical_bounds.width
        );
        assert_eq!(
            snapshots[2].logical_bounds.x + snapshots[2].logical_bounds.width,
            0.0
        );
        assert_eq!(
            snapshots[3].logical_bounds.y + snapshots[3].logical_bounds.height,
            0.0
        );
        assert!(snapshots.iter().all(|monitor| monitor.scale_factor == 2.0));
        assert_eq!(
            capture_window_geometry(&snapshots, scale),
            Some(CaptureWindowGeometry {
                bounds: LogicalRect {
                    x: -1280.0,
                    y: -540.0,
                    width: 4160.0,
                    height: 1620.0
                },
                desktop_scale: Some(2.0),
            })
        );
    }

    fn monitor(id: &str, x: i32, y: i32, width: u32, height: u32, scale: f64) -> MonitorSnapshot {
        MonitorSnapshot {
            id: id.into(),
            logical_bounds: LogicalRect {
                x: x as f64 / scale,
                y: y as f64 / scale,
                width: width as f64 / scale,
                height: height as f64 / scale,
            },
            physical_bounds: PhysicalRect {
                x,
                y,
                width,
                height,
            },
            scale_factor: scale,
            png_data: Vec::new(),
        }
    }
}

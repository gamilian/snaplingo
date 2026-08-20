#[cfg(any(target_os = "windows", target_os = "linux", test))]
use crate::domain::capture::LogicalPoint;
#[cfg(any(target_os = "windows", target_os = "linux", test))]
use crate::domain::capture::WindowCandidate;
use crate::domain::capture::{LogicalRect, MonitorLayout, MonitorSnapshot, PhysicalRect};

pub fn monitor_snapshot_from_physical_geometry(
    id: String,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    scale_factor: f64,
    png_data: Vec<u8>,
) -> MonitorSnapshot {
    let scale_factor = normalized_scale_factor(scale_factor);

    MonitorSnapshot {
        id,
        logical_bounds: logical_rect_from_physical(x, y, width, height, scale_factor),
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

pub fn monitor_layout_from_physical_geometry(
    id: String,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    scale_factor: f64,
) -> MonitorLayout {
    let scale_factor = normalized_scale_factor(scale_factor);

    MonitorLayout {
        id,
        logical_bounds: logical_rect_from_physical(x, y, width, height, scale_factor),
        physical_bounds: PhysicalRect {
            x,
            y,
            width,
            height,
        },
        scale_factor,
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
    Some(WindowCandidate {
        id,
        title,
        app_name,
        logical_bounds: LogicalRect {
            x: monitor.logical_bounds.x + (x - monitor.physical_bounds.x) as f64 / scale,
            y: monitor.logical_bounds.y + (y - monitor.physical_bounds.y) as f64 / scale,
            width: width as f64 / scale,
            height: height as f64 / scale,
        },
    })
}

#[cfg(any(target_os = "windows", target_os = "linux", test))]
pub fn logical_point_from_physical_geometry(
    x: i32,
    y: i32,
    monitors: &[MonitorSnapshot],
) -> Option<LogicalPoint> {
    let monitor = monitors.iter().find(|monitor| {
        let bounds = &monitor.physical_bounds;
        x >= bounds.x
            && y >= bounds.y
            && x < bounds.x.saturating_add_unsigned(bounds.width)
            && y < bounds.y.saturating_add_unsigned(bounds.height)
    })?;
    let scale = monitor.scale_factor.max(1.0);

    Some(LogicalPoint {
        x: monitor.logical_bounds.x + (x - monitor.physical_bounds.x) as f64 / scale,
        y: monitor.logical_bounds.y + (y - monitor.physical_bounds.y) as f64 / scale,
    })
}

fn normalized_scale_factor(scale_factor: f64) -> f64 {
    if scale_factor > 0.0 {
        scale_factor
    } else {
        1.0
    }
}

pub(crate) fn logical_rect_from_physical(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    scale_factor: f64,
) -> LogicalRect {
    LogicalRect {
        x: x as f64 / scale_factor,
        y: y as f64 / scale_factor,
        width: width as f64 / scale_factor,
        height: height as f64 / scale_factor,
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monitor_snapshot_from_physical_geometry_derives_logical_bounds() {
        let snapshot = monitor_snapshot_from_physical_geometry(
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
            LogicalRect {
                x: -1280.0,
                y: 0.0,
                width: 1280.0,
                height: 720.0,
            }
        );
        assert_eq!(snapshot.scale_factor, 2.0);
        assert_eq!(snapshot.png_data, vec![4, 5, 6]);
    }

    #[test]
    fn window_candidate_uses_matching_monitor_scale() {
        let monitors = vec![monitor_snapshot_from_physical_geometry(
            "retina".to_string(),
            -2560,
            0,
            2560,
            1440,
            2.0,
            vec![],
        )];

        let candidate = window_candidate_from_physical_geometry(
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
        assert_eq!(candidate.logical_bounds.x, -1180.0);
        assert_eq!(candidate.logical_bounds.width, 400.0);
    }

    #[test]
    fn window_candidate_rejects_offscreen_windows() {
        let monitors = vec![monitor_snapshot_from_physical_geometry(
            "primary".to_string(),
            0,
            0,
            1000,
            800,
            1.0,
            vec![],
        )];

        assert!(window_candidate_from_physical_geometry(
            "window-8".to_string(),
            "Hidden".to_string(),
            "Example".to_string(),
            1200,
            100,
            400,
            300,
            &monitors,
        )
        .is_none());
    }

    #[test]
    fn cursor_point_uses_the_scale_of_its_monitor() {
        let monitors = vec![
            monitor_snapshot_from_physical_geometry(
                "primary".to_string(),
                0,
                0,
                3840,
                2160,
                1.5,
                vec![],
            ),
            monitor_snapshot_from_physical_geometry(
                "secondary".to_string(),
                -1920,
                0,
                1920,
                1080,
                1.0,
                vec![],
            ),
        ];

        assert_eq!(
            logical_point_from_physical_geometry(-960, 540, &monitors),
            Some(LogicalPoint {
                x: -960.0,
                y: 540.0
            })
        );
    }
}

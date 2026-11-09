use crate::application::capture::ScrollingCaptureGeometry;
use crate::domain::capture::LogicalRect;

pub(super) fn preview_placement(
    geometry: &ScrollingCaptureGeometry,
    ui_scale: f64,
    controls: Option<&LogicalRect>,
) -> Option<LogicalRect> {
    let view = &geometry.viewport;
    let mut slots = Vec::new();
    for screen in &geometry.displays {
        for (left, right) in [
            (
                screen.x + 12.0,
                (view.x - 12.0).min(screen.x + screen.width - 12.0),
            ),
            (
                (view.x + view.width + 12.0).max(screen.x + 12.0),
                screen.x + screen.width - 12.0,
            ),
        ] {
            let available = right - left;
            if available < 96.0 * ui_scale || screen.height < 96.0 * ui_scale {
                continue;
            }
            let width = available.min(240.0 * ui_scale);
            let height = view.height.min(screen.height - 24.0).min(360.0 * ui_scale);
            let y = (view.y + view.height - height).clamp(
                screen.y + 12.0,
                (screen.y + screen.height - height - 12.0).max(screen.y + 12.0),
            );
            let mut rect = LogicalRect {
                x: if left > screen.x + 12.0 {
                    right - width
                } else {
                    left
                },
                y,
                width,
                height,
            };
            if let Some(controls) = controls.filter(|controls| overlaps(controls, &rect)) {
                let remaining = rect.y + rect.height - controls.y - controls.height - 12.0;
                if remaining < 96.0 * ui_scale {
                    continue;
                }
                rect.y += rect.height - remaining;
                rect.height = remaining;
            }
            slots.push((available, rect));
        }
    }
    slots
        .into_iter()
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, rect)| rect)
}

pub(super) fn overlaps(a: &LogicalRect, b: &LogicalRect) -> bool {
    a.x < b.x + b.width && a.x + a.width > b.x && a.y < b.y + b.height && a.y + a.height > b.y
}

pub(super) fn outside_position(
    geometry: &ScrollingCaptureGeometry,
    width: f64,
    height: f64,
) -> Option<(f64, f64)> {
    let view = &geometry.viewport;
    for screen in &geometry.displays {
        let clamp_x = |x: f64| {
            x.max(screen.x)
                .min((screen.x + screen.width - width).max(screen.x))
        };
        let clamp_y = |y: f64| {
            y.max(screen.y)
                .min((screen.y + screen.height - height).max(screen.y))
        };
        for (x, y) in [
            (clamp_x(view.x), view.y + view.height + 12.0),
            (clamp_x(view.x), view.y - height - 12.0),
            (view.x + view.width + 12.0, clamp_y(view.y)),
            (view.x - width - 12.0, clamp_y(view.y)),
        ] {
            let candidate = LogicalRect {
                x,
                y,
                width,
                height,
            };
            if x >= screen.x
                && y >= screen.y
                && x + width <= screen.x + screen.width
                && y + height <= screen.y + screen.height
                && !overlaps(&candidate, view)
            {
                return Some((x, y));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::capture::{MonitorSnapshot, PhysicalRect};

    fn geometry(viewport: LogicalRect) -> ScrollingCaptureGeometry {
        ScrollingCaptureGeometry {
            displays: vec![LogicalRect {
                x: -1920.0,
                y: -100.0,
                width: 1920.0,
                height: 1080.0,
            }],
            viewport,
            desktop_scale: Some(2.0),
            monitors: vec![MonitorSnapshot {
                id: "left".into(),
                logical_bounds: LogicalRect {
                    x: -1920.0,
                    y: -100.0,
                    width: 1920.0,
                    height: 1080.0,
                },
                physical_bounds: PhysicalRect {
                    x: -3840,
                    y: -200,
                    width: 3840,
                    height: 2160,
                },
                scale_factor: 2.0,
                png_data: Vec::new(),
            }],
        }
    }

    #[test]
    fn places_the_panel_outside_a_viewport_on_a_negative_coordinate_display() {
        let viewport = LogicalRect {
            x: -1700.0,
            y: 50.0,
            width: 800.0,
            height: 400.0,
        };
        let geometry = geometry(viewport.clone());
        assert_eq!(
            outside_position(&geometry, 360.0, 460.0),
            Some((-1700.0, 462.0))
        );
    }

    #[test]
    fn a_full_screen_viewport_requires_hiding_the_panel_during_sampling() {
        let geometry = geometry(LogicalRect {
            x: -1920.0,
            y: -100.0,
            width: 1920.0,
            height: 1080.0,
        });
        assert_eq!(outside_position(&geometry, 360.0, 460.0), None);
        assert_eq!(outside_position(&geometry, 360.0, 132.0), None);
        assert!(preview_placement(&geometry, 1.0, None).is_none());
    }

    #[test]
    fn preview_uses_the_wider_side_and_aligns_with_the_viewport_bottom() {
        let geometry = geometry(LogicalRect {
            x: -1700.0,
            y: 50.0,
            width: 800.0,
            height: 400.0,
        });
        assert_eq!(
            preview_placement(&geometry, 1.0, None),
            Some(LogicalRect {
                x: -252.0,
                y: 90.0,
                width: 240.0,
                height: 360.0,
            })
        );
    }

    #[test]
    fn preview_prefers_the_right_when_both_sides_have_equal_room() {
        let geometry = geometry(LogicalRect {
            x: -1360.0,
            y: 50.0,
            width: 800.0,
            height: 400.0,
        });
        assert_eq!(preview_placement(&geometry, 1.0, None).unwrap().x, -252.0);
    }
}

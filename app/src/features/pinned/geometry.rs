use super::position::{cascaded_center, union};
use snaplingo_core::capture::model::DesktopPosition;
use snaplingo_core::capture::model::MonitorSnapshot;
use snaplingo_core::capture::model::PhysicalPoint;
use snaplingo_core::capture::CaptureCoordinatePolicy;
use snaplingo_core::pinned::PinnedImagePlacement;

/// Windows capture logical origins use one primary-display desktop scale.
/// Keep the host boundary physical so winit cannot apply a second monitor DPI.
pub(super) fn position(
    placement: &PinnedImagePlacement,
    monitors: &[MonitorSnapshot],
    physical_size: (u32, u32),
    cascade: usize,
) -> Option<PhysicalPoint> {
    let mut monitors = monitors.to_vec();
    let scale = CaptureCoordinatePolicy::PrimaryMonitorScale
        .normalize(&mut monitors)
        .unwrap_or(1.);
    let point = if placement.centered {
        let bounds = union(
            &monitors
                .into_iter()
                .map(|monitor| monitor.logical_bounds)
                .collect::<Vec<_>>(),
        )
        .ok()?;
        cascaded_center(
            &bounds,
            (
                physical_size.0 as f64 / scale,
                physical_size.1 as f64 / scale,
            ),
            cascade,
        )
    } else {
        match placement.origin.as_ref()? {
            DesktopPosition::Physical(point) => return Some(point.clone()),
            DesktopPosition::Logical(point) => point.clone(),
        }
    };
    Some(PhysicalPoint {
        x: (point.x * scale).round() as i32,
        y: (point.y * scale).round() as i32,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use snaplingo_core::capture::model::LogicalPoint;
    use snaplingo_core::capture::model::LogicalRect;
    use snaplingo_core::capture::model::PhysicalRect;

    fn monitors() -> Vec<MonitorSnapshot> {
        [(0, 0, 3840, 2160, 2.), (-2560, -200, 2560, 1440, 1.25)]
            .into_iter()
            .enumerate()
            .map(|(index, (x, y, width, height, scale))| MonitorSnapshot {
                id: index.to_string(),
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
            })
            .collect()
    }
    fn physical(x: i32, y: i32) -> Option<PhysicalPoint> {
        Some(PhysicalPoint { x, y })
    }
    #[test]
    fn physical_capture_origin_stays_exact_on_a_negative_mixed_dpi_monitor() {
        let placement = PinnedImagePlacement {
            origin: physical(-2400, -150).map(DesktopPosition::Physical),
            centered: false,
        };
        assert_eq!(
            position(&placement, &monitors(), (800, 600), 0),
            physical(-2400, -150)
        );
    }
    #[test]
    fn capture_logical_origin_uses_the_primary_desktop_scale() {
        let placement = PinnedImagePlacement {
            origin: Some(DesktopPosition::Logical(LogicalPoint {
                x: -1200.,
                y: -75.,
            })),
            centered: false,
        };
        assert_eq!(
            position(&placement, &monitors(), (800, 600), 0),
            physical(-2400, -150)
        );
    }
    #[test]
    fn fractional_capture_logical_origins_round_at_the_physical_boundary() {
        let mut monitors = monitors();
        monitors[0].scale_factor = 1.5;
        let placement = PinnedImagePlacement {
            origin: Some(DesktopPosition::Logical(LogicalPoint {
                x: -100.5,
                y: 20.5,
            })),
            centered: false,
        };
        assert_eq!(
            position(&placement, &monitors, (800, 600), 0),
            physical(-151, 31)
        );
    }
    #[test]
    fn first_pin_centers_on_the_whole_virtual_desktop() {
        assert_eq!(
            position(&Default::default(), &monitors(), (800, 600), 0),
            physical(240, 680)
        );
    }
    #[test]
    fn centered_pin_cascade_clamps_within_virtual_desktop_edges() {
        assert_eq!(
            position(&Default::default(), &monitors(), (800, 600), 1),
            physical(280, 720)
        );
        assert_eq!(
            position(&Default::default(), &monitors(), (800, 600), 1000),
            physical(3032, 1552)
        );
    }
    #[test]
    fn centering_uses_the_actual_destination_window_extent_at_mixed_dpi() {
        let mut monitors = monitors();
        monitors[1].physical_bounds = PhysicalRect {
            x: -7680,
            y: 0,
            width: 7680,
            height: 2160,
        };
        // The virtual center lies on the 125% display. A 400x300 DIP window
        // occupies 500x375 pixels there, while the primary display uses 200%.
        assert_eq!(
            position(&Default::default(), &monitors, (500, 375), 0),
            physical(-2170, 893)
        );
    }
    #[test]
    fn oversized_pins_and_empty_display_lists_have_defined_bounds() {
        assert_eq!(
            position(&Default::default(), &monitors(), (7000, 3000), 0),
            physical(-2552, -192)
        );
        assert_eq!(position(&Default::default(), &[], (800, 600), 0), None);
        let placement = PinnedImagePlacement {
            origin: physical(-2400, -150).map(DesktopPosition::Physical),
            centered: false,
        };
        assert_eq!(
            position(&placement, &[], (800, 600), 0),
            physical(-2400, -150)
        );
    }
}

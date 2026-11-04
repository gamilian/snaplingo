#[cfg(not(target_os = "macos"))]
use crate::domain::capture::DesktopPosition;
use crate::domain::capture::LogicalRect;

#[cfg(not(target_os = "macos"))]
pub(crate) fn center_on_virtual_desktop(
    window: &tauri::WebviewWindow,
    cascade: usize,
) -> Result<(), String> {
    let (bounds, size) = {
        let screens: Vec<_> = window
            .available_monitors()
            .map_err(|error| error.to_string())?
            .into_iter()
            .map(|monitor| LogicalRect {
                x: monitor.position().x as f64,
                y: monitor.position().y as f64,
                width: monitor.size().width as f64,
                height: monitor.size().height as f64,
            })
            .collect();
        let size = window.outer_size().map_err(|error| error.to_string())?;
        (union(&screens)?, (size.width as f64, size.height as f64))
    };
    let point = cascaded_center(&bounds, size, cascade);
    let position = DesktopPosition::Physical(crate::domain::capture::PhysicalPoint {
        x: point.x.round() as i32,
        y: point.y.round() as i32,
    });
    set_desktop_position(window, position)
}

pub(super) fn union(rects: &[LogicalRect]) -> Result<LogicalRect, String> {
    let first = rects.first().ok_or("No desktop display is available")?;
    let (mut left, mut top, mut right, mut bottom) = (
        first.x,
        first.y,
        first.x + first.width,
        first.y + first.height,
    );
    for rect in &rects[1..] {
        left = left.min(rect.x);
        top = top.min(rect.y);
        right = right.max(rect.x + rect.width);
        bottom = bottom.max(rect.y + rect.height);
    }
    Ok(LogicalRect {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    })
}

pub(super) fn cascaded_center(
    bounds: &LogicalRect,
    size: (f64, f64),
    cascade: usize,
) -> crate::domain::capture::LogicalPoint {
    let offset = cascade as f64 * 20.0;
    crate::domain::capture::LogicalPoint {
        x: (bounds.x + (bounds.width - size.0) / 2.0 + offset).clamp(
            bounds.x + 4.0,
            (bounds.x + bounds.width - size.0 - 4.0).max(bounds.x + 4.0),
        ),
        y: (bounds.y + (bounds.height - size.1) / 2.0 + offset).clamp(
            bounds.y + 4.0,
            (bounds.y + bounds.height - size.1 - 4.0).max(bounds.y + 4.0),
        ),
    }
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn desktop_rect(
    window: &tauri::WebviewWindow,
    desktop_scale: Option<f64>,
) -> Result<LogicalRect, String> {
    let scale = desktop_scale.unwrap_or(window.scale_factor().map_err(|error| error.to_string())?);
    let position = window.outer_position().map_err(|error| error.to_string())?;
    let size = window.outer_size().map_err(|error| error.to_string())?;
    Ok(LogicalRect {
        x: position.x as f64 / scale,
        y: position.y as f64 / scale,
        width: size.width as f64 / scale,
        height: size.height as f64 / scale,
    })
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn set_desktop_position(
    window: &tauri::WebviewWindow,
    position: DesktopPosition,
) -> Result<(), String> {
    match position {
        DesktopPosition::Physical(point) => window
            .set_position(tauri::PhysicalPosition::new(point.x, point.y))
            .map_err(|error| error.to_string()),
        DesktopPosition::Logical(point) => window
            .set_position(tauri::LogicalPosition::new(point.x, point.y))
            .map_err(|error| error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn centers_on_the_virtual_desktop_and_cascades_within_its_edges() {
        let bounds = union(&[
            LogicalRect {
                x: -1920.0,
                y: -200.0,
                width: 1920.0,
                height: 1080.0,
            },
            LogicalRect {
                x: 0.0,
                y: 0.0,
                width: 2560.0,
                height: 1440.0,
            },
        ])
        .unwrap();
        let first = cascaded_center(&bounds, (400.0, 300.0), 0);
        let second = cascaded_center(&bounds, (400.0, 300.0), 1);
        assert_eq!((first.x, first.y), (120.0, 470.0));
        assert_eq!((second.x - first.x, second.y - first.y), (20.0, 20.0));
        let last = cascaded_center(&bounds, (400.0, 300.0), 1000);
        assert!(last.x + 400.0 < bounds.x + bounds.width);
        assert!(last.y + 300.0 < bounds.y + bounds.height);
    }
}

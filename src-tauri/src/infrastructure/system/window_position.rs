use crate::domain::capture::DesktopPosition;
use crate::domain::capture::LogicalRect;

pub(crate) fn center_on_virtual_desktop(
    window: &tauri::WebviewWindow,
    cascade: usize,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let (bounds, size) = {
        use core_graphics::display::CGDisplay;
        let screens: Vec<_> = CGDisplay::active_displays()
            .map_err(|error| format!("Display enumeration failed: {error}"))?
            .into_iter()
            .map(|id| {
                let bounds = CGDisplay::new(id).bounds();
                LogicalRect {
                    x: bounds.origin.x,
                    y: bounds.origin.y,
                    width: bounds.size.width,
                    height: bounds.size.height,
                }
            })
            .collect();
        let frame = desktop_rect(window, None)?;
        (union(&screens)?, (frame.width, frame.height))
    };
    #[cfg(not(target_os = "macos"))]
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
    #[cfg(target_os = "macos")]
    let position = DesktopPosition::Logical(point);
    #[cfg(not(target_os = "macos"))]
    let position = DesktopPosition::Physical(crate::domain::capture::PhysicalPoint {
        x: point.x.round() as i32,
        y: point.y.round() as i32,
    });
    set_desktop_position(window, position)
}

fn union(rects: &[LogicalRect]) -> Result<LogicalRect, String> {
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

fn cascaded_center(
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

pub(crate) fn desktop_rect(
    window: &tauri::WebviewWindow,
    desktop_scale: Option<f64>,
) -> Result<LogicalRect, String> {
    #[cfg(target_os = "macos")]
    if desktop_scale.is_none() {
        let pointer = window.ns_window().map_err(|error| error.to_string())?;
        let frame: objc2_foundation::NSRect =
            unsafe { objc2::msg_send![&*pointer.cast::<objc2::runtime::AnyObject>(), frame] };
        let height = core_graphics::display::CGDisplay::main()
            .bounds()
            .size
            .height;
        return Ok(LogicalRect {
            x: frame.origin.x,
            y: height - frame.origin.y - frame.size.height,
            width: frame.size.width,
            height: frame.size.height,
        });
    }
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

pub(crate) fn set_desktop_position(
    window: &tauri::WebviewWindow,
    position: DesktopPosition,
) -> Result<(), String> {
    match position {
        DesktopPosition::Physical(point) => window
            .set_position(tauri::PhysicalPosition::new(point.x, point.y))
            .map_err(|error| error.to_string()),
        DesktopPosition::Logical(point) => {
            #[cfg(target_os = "macos")]
            {
                // Core Graphics and the capture session use a top-left origin;
                // NSWindow uses unscaled desktop points with a bottom-left origin.
                let height = core_graphics::display::CGDisplay::main()
                    .bounds()
                    .size
                    .height;
                let pointer = window.ns_window().map_err(|error| error.to_string())?;
                let position = objc2_foundation::NSPoint::new(point.x, height - point.y);
                unsafe {
                    let object = &*pointer.cast::<objc2::runtime::AnyObject>();
                    let _: () = objc2::msg_send![object, setFrameTopLeftPoint: position];
                }
                Ok(())
            }
            #[cfg(not(target_os = "macos"))]
            {
                window
                    .set_position(tauri::LogicalPosition::new(point.x, point.y))
                    .map_err(|error| error.to_string())
            }
        }
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

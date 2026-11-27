use crate::domain::capture::DesktopPosition;
use crate::domain::capture::{LogicalPoint, LogicalRect};

#[derive(Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CursorWindowPlacement {
    Cursor,
    BelowCursor,
    Center,
}

pub(crate) fn position_near_cursor(
    window: &tauri::WebviewWindow,
    placement: CursorWindowPlacement,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let (screens, cursor, size, scale) = {
        use core_graphics::display::CGDisplay;
        // AppKit and Core Graphics use desktop points, regardless of Retina scale.
        let mouse = objc2_app_kit::NSEvent::mouseLocation();
        let cursor = LogicalPoint {
            x: mouse.x,
            y: CGDisplay::main().bounds().size.height - mouse.y,
        };
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
        (screens, cursor, (frame.width, frame.height), 1.0)
    };
    #[cfg(not(target_os = "macos"))]
    let (screens, cursor, size, scale) = {
        let cursor = window
            .cursor_position()
            .map_err(|error| error.to_string())?;
        let monitor = window
            .monitor_from_point(cursor.x, cursor.y)
            .map_err(|error| error.to_string())?
            .ok_or("No display is available under the cursor")?;
        let bounds = monitor.work_area();
        let size = window.outer_size().map_err(|error| error.to_string())?;
        let ratio =
            monitor.scale_factor() / window.scale_factor().map_err(|error| error.to_string())?;
        (
            vec![LogicalRect {
                x: bounds.position.x as f64,
                y: bounds.position.y as f64,
                width: bounds.size.width as f64,
                height: bounds.size.height as f64,
            }],
            LogicalPoint {
                x: cursor.x,
                y: cursor.y,
            },
            (size.width as f64 * ratio, size.height as f64 * ratio),
            monitor.scale_factor(),
        )
    };
    let point = cursor_window_position(&screens, &cursor, size, placement, scale)?;
    #[cfg(target_os = "macos")]
    let position = DesktopPosition::Logical(point);
    #[cfg(not(target_os = "macos"))]
    let position = DesktopPosition::Physical(crate::domain::capture::PhysicalPoint {
        x: point.x.round() as i32,
        y: point.y.round() as i32,
    });
    set_desktop_position(window, position)
}

fn cursor_window_position(
    screens: &[LogicalRect],
    cursor: &LogicalPoint,
    size: (f64, f64),
    placement: CursorWindowPlacement,
    scale: f64,
) -> Result<LogicalPoint, String> {
    let screen = screens
        .iter()
        .find(|screen| {
            cursor.x >= screen.x
                && cursor.x < screen.x + screen.width
                && cursor.y >= screen.y
                && cursor.y < screen.y + screen.height
        })
        .or_else(|| {
            screens.iter().min_by(|a, b| {
                let distance = |rect: &LogicalRect| {
                    let x = cursor.x.clamp(rect.x, rect.x + rect.width);
                    let y = cursor.y.clamp(rect.y, rect.y + rect.height);
                    (x - cursor.x).powi(2) + (y - cursor.y).powi(2)
                };
                distance(a).total_cmp(&distance(b))
            })
        })
        .ok_or("No desktop display is available")?;
    let inset = 12.0 * scale;
    let (x, y) = match placement {
        CursorWindowPlacement::Cursor => (cursor.x, cursor.y),
        CursorWindowPlacement::BelowCursor => (cursor.x, cursor.y + inset),
        CursorWindowPlacement::Center => (
            screen.x + (screen.width - size.0) / 2.0,
            screen.y + (screen.height - size.1) / 2.0,
        ),
    };
    Ok(LogicalPoint {
        x: x.clamp(
            screen.x + inset,
            (screen.x + screen.width - size.0 - inset).max(screen.x + inset),
        ),
        y: y.clamp(
            screen.y + inset,
            (screen.y + screen.height - size.1 - inset).max(screen.y + inset),
        ),
    })
}

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
    fn result_follows_the_cursor_between_displays_and_stays_inside_the_target() {
        let screens = [
            LogicalRect {
                x: -1920.0,
                y: -200.0,
                width: 1920.0,
                height: 1080.0,
            },
            LogicalRect {
                x: 0.0,
                y: 0.0,
                width: 1440.0,
                height: 900.0,
            },
        ];
        for (cursor, screen) in [
            (
                LogicalPoint {
                    x: -1800.0,
                    y: -100.0,
                },
                &screens[0],
            ),
            (LogicalPoint { x: -10.0, y: 870.0 }, &screens[0]),
            (LogicalPoint { x: 0.0, y: 0.0 }, &screens[1]),
            (
                LogicalPoint {
                    x: 1400.0,
                    y: 890.0,
                },
                &screens[1],
            ),
        ] {
            let point = cursor_window_position(
                &screens,
                &cursor,
                (660.0, 660.0),
                CursorWindowPlacement::Cursor,
                1.0,
            )
            .unwrap();
            assert!(point.x >= screen.x && point.y >= screen.y);
            assert!(point.x + 660.0 <= screen.x + screen.width);
            assert!(point.y + 660.0 <= screen.y + screen.height);
        }
    }

    #[test]
    fn centers_and_offsets_on_the_cursor_display() {
        let screen = LogicalRect {
            x: -1920.0,
            y: -200.0,
            width: 1920.0,
            height: 1080.0,
        };
        let cursor = LogicalPoint { x: -1800.0, y: 0.0 };
        let centered = cursor_window_position(
            &[screen.clone()],
            &cursor,
            (660.0, 660.0),
            CursorWindowPlacement::Center,
            1.0,
        )
        .unwrap();
        assert_eq!((centered.x, centered.y), (-1290.0, 10.0));
        let below = cursor_window_position(
            &[screen],
            &cursor,
            (660.0, 660.0),
            CursorWindowPlacement::BelowCursor,
            1.0,
        )
        .unwrap();
        assert_eq!((below.x, below.y), (-1800.0, 12.0));
    }

    #[test]
    fn cursor_placement_uses_target_display_pixels_for_a_scaled_window() {
        let screen = LogicalRect {
            x: 1920.0,
            y: -400.0,
            width: 2560.0,
            height: 1440.0,
        };
        let point = cursor_window_position(
            &[screen],
            &LogicalPoint {
                x: 4400.0,
                y: 1000.0,
            },
            (990.0, 990.0),
            CursorWindowPlacement::Cursor,
            1.5,
        )
        .unwrap();
        assert_eq!((point.x, point.y), (3472.0, 32.0));
    }

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

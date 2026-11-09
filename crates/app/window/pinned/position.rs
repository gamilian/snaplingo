use snaplingo_core::domain::capture::LogicalRect;

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
) -> snaplingo_core::domain::capture::LogicalPoint {
    let offset = cascade as f64 * 20.0;
    snaplingo_core::domain::capture::LogicalPoint {
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

use crate::capture::model::AnnotationCommand;
use crate::capture::model::LogicalPoint;
use crate::capture::model::LogicalRect;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnnotationHandle {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
    ArrowStart,
    ArrowEnd,
    Control1,
    Control2,
    Radius,
}

pub fn arrow_controls(annotation: &AnnotationCommand) -> Option<[LogicalPoint; 2]> {
    let AnnotationCommand::Arrow {
        start,
        end,
        control1,
        control2,
        ..
    } = annotation
    else {
        return None;
    };
    let between = |t: f64| LogicalPoint {
        x: start.x + (end.x - start.x) * t,
        y: start.y + (end.y - start.y) * t,
    };
    Some([
        control1.clone().unwrap_or_else(|| between(1.0 / 3.0)),
        control2.clone().unwrap_or_else(|| between(2.0 / 3.0)),
    ])
}

pub fn arrow_points(annotation: &AnnotationCommand) -> Vec<LogicalPoint> {
    let AnnotationCommand::Arrow { start, end, .. } = annotation else {
        return Vec::new();
    };
    let [c1, c2] = arrow_controls(annotation).unwrap();
    let distance = |a: &LogicalPoint, b: &LogicalPoint| (b.x - a.x).hypot(b.y - a.y);
    let steps = ((distance(start, &c1) + distance(&c1, &c2) + distance(&c2, end)) / 8.0)
        .ceil()
        .clamp(16.0, 256.0) as usize;
    (0..=steps)
        .map(|index| {
            let t = index as f64 / steps as f64;
            let u = 1.0 - t;
            LogicalPoint {
                x: u.powi(3) * start.x
                    + 3.0 * u.powi(2) * t * c1.x
                    + 3.0 * u * t.powi(2) * c2.x
                    + t.powi(3) * end.x,
                y: u.powi(3) * start.y
                    + 3.0 * u.powi(2) * t * c1.y
                    + 3.0 * u * t.powi(2) * c2.y
                    + t.powi(3) * end.y,
            }
        })
        .collect()
}

pub fn handles(annotation: &AnnotationCommand) -> Vec<(AnnotationHandle, LogicalPoint)> {
    use AnnotationHandle::*;
    if let AnnotationCommand::Arrow { start, end, .. } = annotation {
        let [c1, c2] = arrow_controls(annotation).unwrap();
        return vec![
            (ArrowStart, start.clone()),
            (ArrowEnd, end.clone()),
            (Control1, c1),
            (Control2, c2),
        ];
    }
    if !matches!(
        annotation,
        AnnotationCommand::Rectangle { .. }
            | AnnotationCommand::Ellipse { .. }
            | AnnotationCommand::RegionHighlight { .. }
            | AnnotationCommand::Blur { .. }
            | AnnotationCommand::Spotlight { .. }
            | AnnotationCommand::Text { .. }
    ) {
        return Vec::new();
    }
    let rect = bounds(annotation);
    let left = rect.x;
    let right = rect.x + rect.width;
    let top = rect.y;
    let bottom = rect.y + rect.height;
    let mid_x = (left + right) / 2.0;
    let mid_y = (top + bottom) / 2.0;
    let mut handles = vec![
        (NorthWest, LogicalPoint { x: left, y: top }),
        (North, LogicalPoint { x: mid_x, y: top }),
        (NorthEast, LogicalPoint { x: right, y: top }),
        (East, LogicalPoint { x: right, y: mid_y }),
        (
            SouthEast,
            LogicalPoint {
                x: right,
                y: bottom,
            },
        ),
        (
            South,
            LogicalPoint {
                x: mid_x,
                y: bottom,
            },
        ),
        (SouthWest, LogicalPoint { x: left, y: bottom }),
        (West, LogicalPoint { x: left, y: mid_y }),
    ];
    if let AnnotationCommand::Rectangle { corner_radius, .. }
    | AnnotationCommand::RegionHighlight { corner_radius, .. }
    | AnnotationCommand::Blur { corner_radius, .. }
    | AnnotationCommand::Spotlight { corner_radius, .. } = annotation
    {
        let radius = corner_radius.min(rect.width.min(rect.height) / 2.0);
        handles.push((
            Radius,
            LogicalPoint {
                x: right - 8.0 - radius / 2.0,
                y: top + 8.0 + radius / 2.0,
            },
        ));
    }
    handles
}

pub fn handle_at(annotation: &AnnotationCommand, point: &LogicalPoint) -> Option<AnnotationHandle> {
    let handles = handles(annotation);
    if let Some((handle, _)) = handles.iter().rev().find(|(_, position)| {
        (position.x - point.x).abs() <= 8.0 && (position.y - point.y).abs() <= 8.0
    }) {
        return Some(*handle);
    }
    if handles.len() < 8 {
        return None;
    }
    let rect = bounds(annotation);
    use AnnotationHandle::*;
    if point.x >= rect.x && point.x <= rect.x + rect.width {
        if (point.y - rect.y).abs() <= 6.0 {
            return Some(North);
        }
        if (point.y - rect.y - rect.height).abs() <= 6.0 {
            return Some(South);
        }
    }
    if point.y >= rect.y && point.y <= rect.y + rect.height {
        if (point.x - rect.x).abs() <= 6.0 {
            return Some(West);
        }
        if (point.x - rect.x - rect.width).abs() <= 6.0 {
            return Some(East);
        }
    }
    None
}

pub(super) fn rect_handle_at(
    rect: &LogicalRect,
    point: &LogicalPoint,
    tolerance: f64,
) -> Option<AnnotationHandle> {
    if point.x < rect.x - tolerance
        || point.x > rect.x + rect.width + tolerance
        || point.y < rect.y - tolerance
        || point.y > rect.y + rect.height + tolerance
    {
        return None;
    }
    let left = (point.x - rect.x).abs();
    let right = (point.x - rect.x - rect.width).abs();
    let top = (point.y - rect.y).abs();
    let bottom = (point.y - rect.y - rect.height).abs();
    let horizontal = if left.min(right) > tolerance {
        0
    } else if left <= right {
        -1
    } else {
        1
    };
    let vertical = if top.min(bottom) > tolerance {
        0
    } else if top <= bottom {
        -1
    } else {
        1
    };
    use AnnotationHandle::*;
    match (horizontal, vertical) {
        (-1, -1) => Some(NorthWest),
        (0, -1) => Some(North),
        (1, -1) => Some(NorthEast),
        (1, 0) => Some(East),
        (1, 1) => Some(SouthEast),
        (0, 1) => Some(South),
        (-1, 1) => Some(SouthWest),
        (-1, 0) => Some(West),
        _ => None,
    }
}

pub fn resized(
    annotation: &AnnotationCommand,
    handle: AnnotationHandle,
    delta: &LogicalPoint,
    selection: &LogicalRect,
    preserve_aspect: bool,
) -> AnnotationCommand {
    use AnnotationHandle::*;
    if delta.x == 0.0 && delta.y == 0.0 {
        return annotation.clone();
    }
    let mut result = annotation.clone();
    if handle == Radius {
        match &mut result {
            AnnotationCommand::Rectangle {
                rect,
                corner_radius,
                ..
            }
            | AnnotationCommand::RegionHighlight {
                rect,
                corner_radius,
                ..
            }
            | AnnotationCommand::Blur {
                rect,
                corner_radius,
                ..
            }
            | AnnotationCommand::Spotlight {
                rect,
                corner_radius,
                ..
            } => {
                *corner_radius = (*corner_radius + (delta.y - delta.x) / 2.0)
                    .max(0.0)
                    .min(rect.width.min(rect.height) / 2.0);
            }
            _ => {}
        }
        return result;
    }
    if let AnnotationCommand::Arrow {
        start,
        end,
        control1,
        control2,
        ..
    } = &mut result
    {
        let [mut c1, mut c2] = arrow_controls(annotation).unwrap();
        match handle {
            Control1 => {
                c1.x += delta.x;
                c1.y += delta.y;
            }
            Control2 => {
                c2.x += delta.x;
                c2.y += delta.y;
            }
            ArrowStart | ArrowEnd => {
                let (moving, anchor) = if handle == ArrowStart {
                    (&*start, &*end)
                } else {
                    (&*end, &*start)
                };
                let target = LogicalPoint {
                    x: moving.x + delta.x,
                    y: moving.y + delta.y,
                };
                let target = super::editor::constrain(
                    super::editor::AnnotationTool::Arrow,
                    anchor,
                    &target,
                    preserve_aspect,
                );
                let dx = moving.x - anchor.x;
                let dy = moving.y - anchor.y;
                let denominator = dx * dx + dy * dy;
                let (real, imaginary) = if denominator > 0.0 {
                    (
                        ((target.x - anchor.x) * dx + (target.y - anchor.y) * dy) / denominator,
                        ((target.y - anchor.y) * dx - (target.x - anchor.x) * dy) / denominator,
                    )
                } else {
                    (1.0, 0.0)
                };
                let transform = |point: &LogicalPoint| LogicalPoint {
                    x: anchor.x + real * (point.x - anchor.x) - imaginary * (point.y - anchor.y),
                    y: anchor.y + imaginary * (point.x - anchor.x) + real * (point.y - anchor.y),
                };
                c1 = transform(&c1);
                c2 = transform(&c2);
                if handle == ArrowStart {
                    *start = target;
                } else {
                    *end = target;
                }
            }
            _ => return result,
        }
        *control1 = Some(c1);
        *control2 = Some(c2);
        return result;
    }
    if matches!(handle, ArrowStart | ArrowEnd | Control1 | Control2) {
        return result;
    }
    match &mut result {
        AnnotationCommand::Rectangle { rect, .. }
        | AnnotationCommand::Ellipse { rect, .. }
        | AnnotationCommand::RegionHighlight { rect, .. }
        | AnnotationCommand::Blur { rect, .. }
        | AnnotationCommand::Spotlight { rect, .. } => {
            *rect = resized_rect(rect, handle, delta, selection, 4.0, preserve_aspect)
        }
        AnnotationCommand::Text {
            position,
            text,
            font_size,
            ..
        } => {
            let original = bounds(annotation);
            let target = resized_rect(&original, handle, delta, selection, 4.0, preserve_aspect);
            *font_size = ((*font_size as f64 * target.height / original.height).round() as u32)
                .clamp(8, 160);
            position.x = target.x;
            position.y = target.y + *font_size as f64 * 1.2 * text.split('\n').count() as f64;
        }
        _ => {}
    }
    result
}

pub(super) fn resized_rect(
    rect: &LogicalRect,
    handle: AnnotationHandle,
    delta: &LogicalPoint,
    selection: &LogicalRect,
    min_size: f64,
    preserve_aspect: bool,
) -> LogicalRect {
    use AnnotationHandle::*;
    let west = matches!(handle, West | NorthWest | SouthWest);
    let east = matches!(handle, East | NorthEast | SouthEast);
    let north = matches!(handle, North | NorthEast | NorthWest);
    let south = matches!(handle, South | SouthEast | SouthWest);
    let clamp = |value: f64, min: f64, max: f64| value.max(min).min(max);
    let original_right = rect.x + rect.width;
    let original_bottom = rect.y + rect.height;
    let right_bound = selection.x + selection.width;
    let bottom_bound = selection.y + selection.height;
    let mut left = if west {
        clamp(rect.x + delta.x, selection.x, original_right - min_size)
    } else {
        rect.x
    };
    let mut right = if east {
        clamp(original_right + delta.x, rect.x + min_size, right_bound)
    } else {
        original_right
    };
    let mut top = if north {
        clamp(rect.y + delta.y, selection.y, original_bottom - min_size)
    } else {
        rect.y
    };
    let mut bottom = if south {
        clamp(original_bottom + delta.y, rect.y + min_size, bottom_bound)
    } else {
        original_bottom
    };
    if preserve_aspect && (west || east) && (north || south) {
        let ratio = rect.width / rect.height;
        let free_width = right - left;
        let free_height = bottom - top;
        let use_width =
            (free_width - rect.width).abs() >= ((free_height - rect.height) * ratio).abs();
        let max_width = if west {
            original_right - selection.x
        } else {
            right_bound - rect.x
        };
        let max_height = if north {
            original_bottom - selection.y
        } else {
            bottom_bound - rect.y
        };
        let mut width = clamp(
            if use_width {
                free_width
            } else {
                free_height * ratio
            },
            min_size,
            max_width,
        );
        let mut height = width / ratio;
        if height > max_height {
            height = max_height;
            width = height * ratio;
        }
        if height < min_size {
            height = min_size;
            width = height * ratio;
        }
        left = if west { original_right - width } else { rect.x };
        right = left + width;
        top = if north {
            original_bottom - height
        } else {
            rect.y
        };
        bottom = top + height;
    }
    LogicalRect {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    }
}

fn point_bounds(points: &[LogicalPoint], padding: f64) -> LogicalRect {
    let first = &points[0];
    let (left, top, right, bottom) = points.iter().skip(1).fold(
        (first.x, first.y, first.x, first.y),
        |(left, top, right, bottom), point| {
            (
                left.min(point.x),
                top.min(point.y),
                right.max(point.x),
                bottom.max(point.y),
            )
        },
    );
    LogicalRect {
        x: left - padding,
        y: top - padding,
        width: right - left + padding * 2.0,
        height: bottom - top + padding * 2.0,
    }
}

pub fn bounds(annotation: &AnnotationCommand) -> LogicalRect {
    match annotation {
        AnnotationCommand::Rectangle { rect, .. }
        | AnnotationCommand::Ellipse { rect, .. }
        | AnnotationCommand::RegionHighlight { rect, .. }
        | AnnotationCommand::Blur { rect, .. }
        | AnnotationCommand::Spotlight { rect, .. } => rect.clone(),
        AnnotationCommand::Arrow { stroke_width, .. } => {
            point_bounds(&arrow_points(annotation), *stroke_width as f64 / 2.0)
        }
        AnnotationCommand::Line {
            start,
            end,
            stroke_width,
            ..
        } => point_bounds(&[start.clone(), end.clone()], *stroke_width as f64 / 2.0),
        AnnotationCommand::Freehand {
            points,
            stroke_width,
            ..
        }
        | AnnotationCommand::Highlight {
            points,
            stroke_width,
            ..
        }
        | AnnotationCommand::Mosaic {
            points,
            stroke_width,
            ..
        }
        | AnnotationCommand::Eraser {
            points,
            stroke_width,
            ..
        } => point_bounds(points, *stroke_width as f64 / 2.0),
        AnnotationCommand::Text {
            position,
            text,
            font_size,
            ..
        } => {
            let lines: Vec<_> = text.split('\n').collect();
            let width = lines
                .iter()
                .map(|line| line.encode_utf16().count())
                .max()
                .unwrap_or(1)
                .max(1) as f64
                * *font_size as f64
                * 0.6;
            let height = *font_size as f64 * 1.2 * lines.len() as f64;
            LogicalRect {
                x: position.x,
                y: position.y - height,
                width,
                height,
            }
        }
    }
}

pub fn hit(annotation: &AnnotationCommand, point: &LogicalPoint, tolerance: f64) -> bool {
    if matches!(
        annotation,
        AnnotationCommand::Arrow {
            control1: Some(_),
            ..
        } | AnnotationCommand::Arrow {
            control2: Some(_),
            ..
        }
    ) {
        let AnnotationCommand::Arrow { stroke_width, .. } = annotation else {
            unreachable!()
        };
        return arrow_points(annotation).windows(2).any(|pair| {
            let dx = pair[1].x - pair[0].x;
            let dy = pair[1].y - pair[0].y;
            let length = dx * dx + dy * dy;
            let t = if length > 0.0 {
                (((point.x - pair[0].x) * dx + (point.y - pair[0].y) * dy) / length).clamp(0.0, 1.0)
            } else {
                0.0
            };
            (point.x - pair[0].x - dx * t).hypot(point.y - pair[0].y - dy * t)
                <= tolerance + *stroke_width as f64 / 2.0
        });
    }
    let rect = bounds(annotation);
    point.x >= rect.x - tolerance
        && point.x <= rect.x + rect.width + tolerance
        && point.y >= rect.y - tolerance
        && point.y <= rect.y + rect.height + tolerance
}

pub fn moved(annotation: &AnnotationCommand, delta: &LogicalPoint) -> AnnotationCommand {
    let mut annotation = annotation.clone();
    let move_point = |point: &mut LogicalPoint| {
        point.x += delta.x;
        point.y += delta.y;
    };
    match &mut annotation {
        AnnotationCommand::Rectangle { rect, .. }
        | AnnotationCommand::Ellipse { rect, .. }
        | AnnotationCommand::RegionHighlight { rect, .. }
        | AnnotationCommand::Blur { rect, .. }
        | AnnotationCommand::Spotlight { rect, .. } => {
            rect.x += delta.x;
            rect.y += delta.y;
        }
        AnnotationCommand::Arrow {
            start,
            end,
            control1,
            control2,
            ..
        } => {
            move_point(start);
            move_point(end);
            if let Some(point) = control1 {
                move_point(point);
            }
            if let Some(point) = control2 {
                move_point(point);
            }
        }
        AnnotationCommand::Line { start, end, .. } => {
            move_point(start);
            move_point(end);
        }
        AnnotationCommand::Freehand { points, .. }
        | AnnotationCommand::Highlight { points, .. }
        | AnnotationCommand::Mosaic { points, .. }
        | AnnotationCommand::Eraser { points, .. } => points.iter_mut().for_each(move_point),
        AnnotationCommand::Text { position, .. } => move_point(position),
    }
    annotation
}

use std::path::Path;

use base64::Engine;

use super::{CaptureImageComposer, CaptureOutput, CaptureSessions, ImageAnnotation, PngPlacement};
use crate::application::providers::ocr::OcrCoordinator;
use crate::domain::capture::{
    AnnotationCommand, CaptureOutputAction, CaptureSessionId, CapturedCursor, LogicalPoint,
    LogicalRect, MonitorSnapshot, PhysicalPoint, PhysicalRect,
};
use crate::domain::ocr::OcrResult;
use crate::error::AppError;

const OCR_SELECTION_PADDING_LOGICAL_PX: f64 = 2.0;

#[derive(Debug, PartialEq)]
struct CaptureImageCompositionPlan {
    width: u32,
    height: u32,
    placements: Vec<CaptureImagePlacement>,
}

#[derive(Debug, PartialEq)]
struct CaptureImagePlacement {
    snapshot_index: usize,
    source_rect: PhysicalRect,
    destination_rect: PhysicalRect,
}

#[derive(Debug, PartialEq)]
struct CaptureCursorPlacement {
    source_rect: PhysicalRect,
    destination_rect: PhysicalRect,
}

pub enum CaptureSessionOutput {
    Completed,
    Pin(Vec<u8>),
}

pub fn render_capture_png_base64(
    capture_sessions: &CaptureSessions,
    image_composition: &CaptureImageComposer,
    session_id: &CaptureSessionId,
    rect: &LogicalRect,
    annotations: &[AnnotationCommand],
    include_cursor: bool,
) -> crate::error::Result<String> {
    let png_data = render_capture_png(
        capture_sessions,
        image_composition,
        session_id,
        rect,
        annotations,
        include_cursor,
    )?;

    Ok(base64::engine::general_purpose::STANDARD.encode(png_data))
}

pub async fn recognize_capture_selection_text(
    capture_sessions: &CaptureSessions,
    image_composition: &CaptureImageComposer,
    ocr: &OcrCoordinator,
    session_id: &CaptureSessionId,
    rect: &LogicalRect,
    language: Option<String>,
) -> crate::error::Result<OcrResult> {
    let session = capture_sessions.get_session(session_id)?;
    let ocr_rect = expanded_ocr_selection_rect(rect, &session.snapshots);
    let png_data = render_capture_png(
        capture_sessions,
        image_composition,
        session_id,
        &ocr_rect,
        &[],
        false,
    )?;

    ocr.recognize(&crate::domain::OcrRequest {
        image_data: png_data,
        language,
    })
    .await
}

pub async fn output_capture_selection(
    capture_sessions: &CaptureSessions,
    image_composition: &CaptureImageComposer,
    output: &CaptureOutput,
    session_id: &CaptureSessionId,
    rect: &LogicalRect,
    annotations: &[AnnotationCommand],
    include_cursor: bool,
    action: CaptureOutputAction,
) -> crate::error::Result<CaptureSessionOutput> {
    let png_data = render_capture_png(
        capture_sessions,
        image_composition,
        session_id,
        rect,
        annotations,
        include_cursor,
    )?;

    match action {
        CaptureOutputAction::Save {
            path,
            format,
            quality,
            copy_after_save,
        } => {
            output
                .save_image(&png_data, Path::new(&path), &format, quality)
                .await?;
            if copy_after_save {
                output.copy_png(&png_data).await?;
            }
            Ok(CaptureSessionOutput::Completed)
        }
        CaptureOutputAction::Copy => {
            output.copy_png(&png_data).await?;
            Ok(CaptureSessionOutput::Completed)
        }
        CaptureOutputAction::Pin => Ok(CaptureSessionOutput::Pin(png_data)),
    }
}

pub fn render_capture_png(
    capture_sessions: &CaptureSessions,
    image_composition: &CaptureImageComposer,
    session_id: &CaptureSessionId,
    rect: &LogicalRect,
    annotations: &[AnnotationCommand],
    include_cursor: bool,
) -> crate::error::Result<Vec<u8>> {
    let session = capture_sessions.get_session(session_id)?;

    let plan =
        capture_image_composition_plan(rect, &session.snapshots).map_err(AppError::System)?;
    let mut placements = plan
        .placements
        .iter()
        .map(|placement| PngPlacement {
            png_data: session.snapshots[placement.snapshot_index]
                .png_data
                .as_slice(),
            source_rect: placement.source_rect.clone(),
            destination_rect: placement.destination_rect.clone(),
        })
        .collect::<Vec<_>>();
    if include_cursor {
        if let Some(cursor) = session.captured_cursor.as_ref() {
            if let Some(placement) =
                captured_cursor_placement_for_selection(cursor, rect, plan.width)
                    .map_err(AppError::System)?
            {
                placements.push(PngPlacement {
                    png_data: cursor.png_data.as_slice(),
                    source_rect: placement.source_rect,
                    destination_rect: placement.destination_rect,
                });
            }
        }
    }

    let image_annotations =
        image_annotations_from_commands(annotations, rect, plan.width).map_err(AppError::System)?;

    image_composition.compose_png_with_annotations(
        plan.width,
        plan.height,
        &placements,
        &image_annotations,
    )
}

fn image_annotations_from_commands(
    annotations: &[AnnotationCommand],
    selection_rect: &LogicalRect,
    output_width: u32,
) -> Result<Vec<ImageAnnotation>, String> {
    if annotations.is_empty() {
        return Ok(Vec::new());
    }

    let output_scale = output_width as f64 / selection_rect.width;
    let annotation_origin = LogicalRect {
        x: 0.0,
        y: 0.0,
        width: selection_rect.width,
        height: selection_rect.height,
    };

    annotations
        .iter()
        .map(|annotation| match annotation {
            AnnotationCommand::Rectangle {
                rect,
                color,
                stroke_width,
                filled,
            } => Ok(ImageAnnotation::Rectangle {
                rect: scaled_logical_rect_relative_to(rect, &annotation_origin, output_scale)?,
                color: *color,
                stroke_width: ((*stroke_width).max(1) as f64 * output_scale).ceil() as u32,
                filled: *filled,
            }),
            AnnotationCommand::Ellipse {
                rect,
                color,
                stroke_width,
                filled,
            } => Ok(ImageAnnotation::Ellipse {
                rect: scaled_logical_rect_relative_to(rect, &annotation_origin, output_scale)?,
                color: *color,
                stroke_width: ((*stroke_width).max(1) as f64 * output_scale).ceil() as u32,
                filled: *filled,
            }),
            AnnotationCommand::Arrow {
                start,
                end,
                color,
                stroke_width,
            } => Ok(ImageAnnotation::Arrow {
                start: scaled_logical_point_relative_to(start, &annotation_origin, output_scale)?,
                end: scaled_logical_point_relative_to(end, &annotation_origin, output_scale)?,
                color: *color,
                stroke_width: ((*stroke_width).max(1) as f64 * output_scale).ceil() as u32,
            }),
            AnnotationCommand::Line {
                start,
                end,
                color,
                stroke_width,
            } => Ok(ImageAnnotation::Line {
                start: scaled_logical_point_relative_to(start, &annotation_origin, output_scale)?,
                end: scaled_logical_point_relative_to(end, &annotation_origin, output_scale)?,
                color: *color,
                stroke_width: ((*stroke_width).max(1) as f64 * output_scale).ceil() as u32,
            }),
            AnnotationCommand::Freehand {
                points,
                color,
                stroke_width,
            } => Ok(ImageAnnotation::Freehand {
                points: points
                    .iter()
                    .map(|point| {
                        scaled_logical_point_relative_to(point, &annotation_origin, output_scale)
                    })
                    .collect::<Result<Vec<_>, String>>()?,
                color: *color,
                stroke_width: ((*stroke_width).max(1) as f64 * output_scale).ceil() as u32,
            }),
            AnnotationCommand::Highlight {
                points,
                color,
                stroke_width,
            } => Ok(ImageAnnotation::Highlight {
                points: points
                    .iter()
                    .map(|point| {
                        scaled_logical_point_relative_to(point, &annotation_origin, output_scale)
                    })
                    .collect::<Result<Vec<_>, String>>()?,
                color: *color,
                stroke_width: ((*stroke_width).max(1) as f64 * output_scale).ceil() as u32,
            }),
            AnnotationCommand::Mosaic {
                points,
                stroke_width,
                block_size,
            } => Ok(ImageAnnotation::Mosaic {
                points: points
                    .iter()
                    .map(|point| {
                        scaled_logical_point_relative_to(point, &annotation_origin, output_scale)
                    })
                    .collect::<Result<Vec<_>, String>>()?,
                stroke_width: ((*stroke_width).max(1) as f64 * output_scale).ceil() as u32,
                block_size: ((*block_size).max(1) as f64 * output_scale).ceil() as u32,
            }),
            AnnotationCommand::Eraser {
                points,
                stroke_width,
            } => Ok(ImageAnnotation::Eraser {
                points: points
                    .iter()
                    .map(|point| {
                        scaled_logical_point_relative_to(point, &annotation_origin, output_scale)
                    })
                    .collect::<Result<Vec<_>, String>>()?,
                stroke_width: ((*stroke_width).max(1) as f64 * output_scale).ceil() as u32,
            }),
            AnnotationCommand::Text {
                position,
                text,
                color,
                font_size,
            } => Ok(ImageAnnotation::Text {
                position: scaled_logical_point_relative_to(
                    position,
                    &annotation_origin,
                    output_scale,
                )?,
                text: text.clone(),
                color: *color,
                font_size: ((*font_size).max(1) as f64 * output_scale).ceil() as u32,
            }),
        })
        .collect()
}

fn capture_image_composition_plan(
    rect: &LogicalRect,
    snapshots: &[MonitorSnapshot],
) -> Result<CaptureImageCompositionPlan, String> {
    if rect.width <= 0.0 || rect.height <= 0.0 {
        return Err("Selection has no area".to_string());
    }

    let intersections = snapshots
        .iter()
        .enumerate()
        .filter_map(|(snapshot_index, snapshot)| {
            logical_rect_intersection(rect, &snapshot.logical_bounds)
                .map(|intersection| (snapshot_index, snapshot, intersection))
        })
        .collect::<Vec<_>>();

    if intersections.is_empty() {
        return Err("Selection does not intersect any captured monitor".to_string());
    }

    let output_scale = intersections
        .iter()
        .map(|(_, snapshot, _)| snapshot.scale_factor)
        .fold(1.0_f64, f64::max);
    let output_width = scaled_extent(rect.width, output_scale)?;
    let output_height = scaled_extent(rect.height, output_scale)?;
    let placements = intersections
        .into_iter()
        .map(|(snapshot_index, snapshot, intersection)| {
            let source_rect = scaled_logical_rect_relative_to(
                &intersection,
                &snapshot.logical_bounds,
                snapshot.scale_factor,
            )?;
            let destination_rect =
                scaled_logical_rect_relative_to(&intersection, rect, output_scale)?;

            Ok(CaptureImagePlacement {
                snapshot_index,
                source_rect,
                destination_rect,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    Ok(CaptureImageCompositionPlan {
        width: output_width,
        height: output_height,
        placements,
    })
}

fn logical_rect_intersection(a: &LogicalRect, b: &LogicalRect) -> Option<LogicalRect> {
    let left = a.x.max(b.x);
    let top = a.y.max(b.y);
    let right = (a.x + a.width).min(b.x + b.width);
    let bottom = (a.y + a.height).min(b.y + b.height);

    if right <= left || bottom <= top {
        return None;
    }

    Some(LogicalRect {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    })
}

fn expanded_ocr_selection_rect(
    selection_rect: &LogicalRect,
    snapshots: &[MonitorSnapshot],
) -> LogicalRect {
    let expanded_rect = LogicalRect {
        x: selection_rect.x - OCR_SELECTION_PADDING_LOGICAL_PX,
        y: selection_rect.y - OCR_SELECTION_PADDING_LOGICAL_PX,
        width: selection_rect.width + OCR_SELECTION_PADDING_LOGICAL_PX * 2.0,
        height: selection_rect.height + OCR_SELECTION_PADDING_LOGICAL_PX * 2.0,
    };
    let Some(capture_bounds) = logical_bounds_for_snapshots(snapshots) else {
        return expanded_rect;
    };

    logical_rect_intersection(&expanded_rect, &capture_bounds)
        .unwrap_or_else(|| selection_rect.clone())
}

fn logical_bounds_for_snapshots(snapshots: &[MonitorSnapshot]) -> Option<LogicalRect> {
    let mut snapshots = snapshots.iter();
    let first = snapshots.next()?.logical_bounds.clone();

    Some(snapshots.fold(first, |bounds, snapshot| {
        let left = bounds.x.min(snapshot.logical_bounds.x);
        let top = bounds.y.min(snapshot.logical_bounds.y);
        let right = (bounds.x + bounds.width)
            .max(snapshot.logical_bounds.x + snapshot.logical_bounds.width);
        let bottom = (bounds.y + bounds.height)
            .max(snapshot.logical_bounds.y + snapshot.logical_bounds.height);

        LogicalRect {
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
        }
    }))
}

fn captured_cursor_placement_for_selection(
    cursor: &CapturedCursor,
    selection_rect: &LogicalRect,
    output_width: u32,
) -> Result<Option<CaptureCursorPlacement>, String> {
    if output_width == 0 || selection_rect.width <= 0.0 || selection_rect.height <= 0.0 {
        return Err("Selection has invalid cursor output size".to_string());
    }

    let cursor_scale = cursor.scale_factor.max(1.0);
    let cursor_rect = LogicalRect {
        x: cursor.logical_position.x - cursor.hotspot.x,
        y: cursor.logical_position.y - cursor.hotspot.y,
        width: cursor.image_width as f64 / cursor_scale,
        height: cursor.image_height as f64 / cursor_scale,
    };
    let Some(visible_rect) = logical_rect_intersection(&cursor_rect, selection_rect) else {
        return Ok(None);
    };

    let output_scale = output_width as f64 / selection_rect.width;
    Ok(Some(CaptureCursorPlacement {
        source_rect: scaled_logical_rect_relative_to(&visible_rect, &cursor_rect, cursor_scale)?,
        destination_rect: scaled_logical_rect_relative_to(
            &visible_rect,
            selection_rect,
            output_scale,
        )?,
    }))
}

fn scaled_logical_rect_relative_to(
    rect: &LogicalRect,
    origin: &LogicalRect,
    scale: f64,
) -> Result<PhysicalRect, String> {
    let left = ((rect.x - origin.x) * scale).floor();
    let top = ((rect.y - origin.y) * scale).floor();
    let right = ((rect.x + rect.width - origin.x) * scale).ceil();
    let bottom = ((rect.y + rect.height - origin.y) * scale).ceil();

    if left < 0.0 || top < 0.0 || right <= left || bottom <= top {
        return Err("Selection has invalid scaled capture bounds".to_string());
    }

    Ok(PhysicalRect {
        x: left as i32,
        y: top as i32,
        width: (right - left) as u32,
        height: (bottom - top) as u32,
    })
}

fn scaled_logical_point_relative_to(
    point: &LogicalPoint,
    origin: &LogicalRect,
    scale: f64,
) -> Result<PhysicalPoint, String> {
    let x = ((point.x - origin.x) * scale).round();
    let y = ((point.y - origin.y) * scale).round();

    if x < 0.0 || y < 0.0 {
        return Err("Annotation has invalid scaled capture point".to_string());
    }

    Ok(PhysicalPoint {
        x: x as i32,
        y: y as i32,
    })
}

fn scaled_extent(value: f64, scale: f64) -> Result<u32, String> {
    let scaled = (value * scale).ceil();
    if scaled <= 0.0 {
        return Err("Selection has invalid scaled capture size".to_string());
    }

    Ok(scaled as u32)
}

#[cfg(test)]
mod tests {
    use image::ImageEncoder;

    use super::{CaptureImageComposer, ImageAnnotation, PngPlacement};
    use crate::domain::capture::{
        AnnotationCommand, LogicalPoint, LogicalRect, PhysicalPoint, PhysicalRect,
    };
    use crate::domain::capture::{CapturedCursor, MonitorSnapshot};

    fn solid_png(width: u32, height: u32, rgba: [u8; 4]) -> Vec<u8> {
        let mut png = Vec::new();
        image::codecs::png::PngEncoder::new(&mut png)
            .write_image(
                &rgba.repeat((width * height) as usize),
                width,
                height,
                image::ExtendedColorType::Rgba8,
            )
            .unwrap();
        png
    }

    fn png_pixel(png: &[u8], x: u32, y: u32) -> [u8; 4] {
        image::load_from_memory(png)
            .unwrap()
            .to_rgba8()
            .get_pixel(x, y)
            .0
    }

    #[test]
    fn capture_image_placements_split_selection_across_monitors() {
        let snapshots = vec![
            MonitorSnapshot {
                id: "left".to_string(),
                logical_bounds: LogicalRect {
                    x: -4.0,
                    y: 0.0,
                    width: 4.0,
                    height: 2.0,
                },
                physical_bounds: PhysicalRect {
                    x: -4,
                    y: 0,
                    width: 4,
                    height: 2,
                },
                scale_factor: 1.0,
                png_data: Vec::new(),
            },
            MonitorSnapshot {
                id: "primary".to_string(),
                logical_bounds: LogicalRect {
                    x: 0.0,
                    y: 0.0,
                    width: 4.0,
                    height: 2.0,
                },
                physical_bounds: PhysicalRect {
                    x: 0,
                    y: 0,
                    width: 4,
                    height: 2,
                },
                scale_factor: 1.0,
                png_data: Vec::new(),
            },
        ];

        let plan = super::capture_image_composition_plan(
            &LogicalRect {
                x: -2.0,
                y: 0.0,
                width: 4.0,
                height: 2.0,
            },
            &snapshots,
        )
        .unwrap();

        assert_eq!(plan.width, 4);
        assert_eq!(plan.height, 2);
        assert_eq!(plan.placements.len(), 2);
        assert_eq!(plan.placements[0].snapshot_index, 0);
        assert_eq!(
            plan.placements[0].source_rect,
            PhysicalRect {
                x: 2,
                y: 0,
                width: 2,
                height: 2,
            }
        );
        assert_eq!(
            plan.placements[0].destination_rect,
            PhysicalRect {
                x: 0,
                y: 0,
                width: 2,
                height: 2,
            }
        );
        assert_eq!(plan.placements[1].snapshot_index, 1);
        assert_eq!(
            plan.placements[1].source_rect,
            PhysicalRect {
                x: 0,
                y: 0,
                width: 2,
                height: 2,
            }
        );
        assert_eq!(
            plan.placements[1].destination_rect,
            PhysicalRect {
                x: 2,
                y: 0,
                width: 2,
                height: 2,
            }
        );
    }

    #[test]
    fn renders_cross_display_selection_at_the_highest_pixel_density() {
        let snapshots = vec![
            MonitorSnapshot {
                id: "left".to_string(),
                logical_bounds: LogicalRect {
                    x: -2.0,
                    y: 0.0,
                    width: 2.0,
                    height: 2.0,
                },
                physical_bounds: PhysicalRect {
                    x: -2,
                    y: 0,
                    width: 2,
                    height: 2,
                },
                scale_factor: 1.0,
                png_data: solid_png(2, 2, [255, 0, 0, 255]),
            },
            MonitorSnapshot {
                id: "retina".to_string(),
                logical_bounds: LogicalRect {
                    x: 0.0,
                    y: 0.0,
                    width: 2.0,
                    height: 2.0,
                },
                physical_bounds: PhysicalRect {
                    x: 0,
                    y: 0,
                    width: 4,
                    height: 4,
                },
                scale_factor: 2.0,
                png_data: solid_png(4, 4, [0, 0, 255, 255]),
            },
        ];
        let plan = super::capture_image_composition_plan(
            &LogicalRect {
                x: -1.0,
                y: 0.0,
                width: 3.0,
                height: 2.0,
            },
            &snapshots,
        )
        .unwrap();
        let placements = plan
            .placements
            .iter()
            .map(|placement| PngPlacement {
                png_data: &snapshots[placement.snapshot_index].png_data,
                source_rect: placement.source_rect.clone(),
                destination_rect: placement.destination_rect.clone(),
            })
            .collect::<Vec<_>>();

        let output = CaptureImageComposer::new()
            .compose_png(plan.width, plan.height, &placements)
            .unwrap();
        let image = image::load_from_memory(&output).unwrap();

        assert_eq!((image.width(), image.height()), (6, 4));
        assert_eq!(png_pixel(&output, 0, 0), [255, 0, 0, 255]);
        assert_eq!(png_pixel(&output, 1, 3), [255, 0, 0, 255]);
        assert_eq!(png_pixel(&output, 2, 0), [0, 0, 255, 255]);
        assert_eq!(png_pixel(&output, 5, 3), [0, 0, 255, 255]);
    }

    #[test]
    fn plans_captured_cursor_placement_inside_selection() {
        let cursor = CapturedCursor {
            logical_position: LogicalPoint { x: 18.0, y: 27.0 },
            hotspot: LogicalPoint { x: 2.0, y: 3.0 },
            image_width: 20,
            image_height: 24,
            scale_factor: 2.0,
            png_data: Vec::new(),
        };

        let placement = super::captured_cursor_placement_for_selection(
            &cursor,
            &LogicalRect {
                x: 10.0,
                y: 20.0,
                width: 40.0,
                height: 30.0,
            },
            80,
        )
        .unwrap()
        .unwrap();

        assert_eq!(
            placement.source_rect,
            PhysicalRect {
                x: 0,
                y: 0,
                width: 20,
                height: 24,
            }
        );
        assert_eq!(
            placement.destination_rect,
            PhysicalRect {
                x: 12,
                y: 8,
                width: 20,
                height: 24,
            }
        );
    }

    #[test]
    fn clips_captured_cursor_placement_to_selection() {
        let cursor = CapturedCursor {
            logical_position: LogicalPoint { x: 11.0, y: 21.0 },
            hotspot: LogicalPoint { x: 4.0, y: 4.0 },
            image_width: 10,
            image_height: 10,
            scale_factor: 1.0,
            png_data: Vec::new(),
        };

        let placement = super::captured_cursor_placement_for_selection(
            &cursor,
            &LogicalRect {
                x: 10.0,
                y: 20.0,
                width: 20.0,
                height: 20.0,
            },
            20,
        )
        .unwrap()
        .unwrap();

        assert_eq!(
            placement.source_rect,
            PhysicalRect {
                x: 3,
                y: 3,
                width: 7,
                height: 7,
            }
        );
        assert_eq!(
            placement.destination_rect,
            PhysicalRect {
                x: 0,
                y: 0,
                width: 7,
                height: 7,
            }
        );
    }

    #[test]
    fn scales_selection_local_annotations_to_output_pixels() {
        let annotations = super::image_annotations_from_commands(
            &[
                AnnotationCommand::Rectangle {
                    rect: LogicalRect {
                        x: 1.0,
                        y: 2.0,
                        width: 3.0,
                        height: 4.0,
                    },
                    color: [255, 77, 79, 255],
                    stroke_width: 2,
                    filled: true,
                },
                AnnotationCommand::Arrow {
                    start: LogicalPoint { x: 2.0, y: 3.0 },
                    end: LogicalPoint { x: 5.0, y: 7.0 },
                    color: [255, 77, 79, 255],
                    stroke_width: 2,
                },
                AnnotationCommand::Line {
                    start: LogicalPoint { x: 3.0, y: 4.0 },
                    end: LogicalPoint { x: 6.0, y: 8.0 },
                    color: [250, 219, 20, 255],
                    stroke_width: 3,
                },
                AnnotationCommand::Freehand {
                    points: vec![
                        LogicalPoint { x: 1.0, y: 1.5 },
                        LogicalPoint { x: 3.0, y: 4.0 },
                    ],
                    color: [24, 144, 255, 255],
                    stroke_width: 1,
                },
                AnnotationCommand::Highlight {
                    points: vec![
                        LogicalPoint { x: 2.0, y: 2.5 },
                        LogicalPoint { x: 4.0, y: 5.0 },
                    ],
                    color: [250, 219, 20, 96],
                    stroke_width: 3,
                },
                AnnotationCommand::Mosaic {
                    points: vec![
                        LogicalPoint { x: 2.0, y: 3.0 },
                        LogicalPoint { x: 6.0, y: 8.0 },
                    ],
                    stroke_width: 12,
                    block_size: 3,
                },
                AnnotationCommand::Ellipse {
                    rect: LogicalRect {
                        x: 0.5,
                        y: 1.0,
                        width: 2.5,
                        height: 3.5,
                    },
                    color: [40, 167, 69, 255],
                    stroke_width: 2,
                    filled: false,
                },
                AnnotationCommand::Text {
                    position: LogicalPoint { x: 3.5, y: 4.5 },
                    text: "Snap".to_string(),
                    color: [255, 255, 255, 255],
                    font_size: 12,
                },
            ],
            &LogicalRect {
                x: 100.0,
                y: 200.0,
                width: 10.0,
                height: 10.0,
            },
            20,
        )
        .unwrap();

        assert_eq!(annotations.len(), 8);
        assert_eq!(
            annotations[0],
            ImageAnnotation::Rectangle {
                rect: PhysicalRect {
                    x: 2,
                    y: 4,
                    width: 6,
                    height: 8,
                },
                color: [255, 77, 79, 255],
                stroke_width: 4,
                filled: true,
            }
        );
        assert_eq!(
            annotations[1],
            ImageAnnotation::Arrow {
                start: PhysicalPoint { x: 4, y: 6 },
                end: PhysicalPoint { x: 10, y: 14 },
                color: [255, 77, 79, 255],
                stroke_width: 4,
            }
        );
        assert_eq!(
            annotations[2],
            ImageAnnotation::Line {
                start: PhysicalPoint { x: 6, y: 8 },
                end: PhysicalPoint { x: 12, y: 16 },
                color: [250, 219, 20, 255],
                stroke_width: 6,
            }
        );
        assert_eq!(
            annotations[3],
            ImageAnnotation::Freehand {
                points: vec![PhysicalPoint { x: 2, y: 3 }, PhysicalPoint { x: 6, y: 8 }],
                color: [24, 144, 255, 255],
                stroke_width: 2,
            }
        );
        assert_eq!(
            annotations[4],
            ImageAnnotation::Highlight {
                points: vec![PhysicalPoint { x: 4, y: 5 }, PhysicalPoint { x: 8, y: 10 }],
                color: [250, 219, 20, 96],
                stroke_width: 6,
            }
        );
        assert_eq!(
            annotations[5],
            ImageAnnotation::Mosaic {
                points: vec![PhysicalPoint { x: 4, y: 6 }, PhysicalPoint { x: 12, y: 16 }],
                stroke_width: 24,
                block_size: 6,
            }
        );
        assert_eq!(
            annotations[6],
            ImageAnnotation::Ellipse {
                rect: PhysicalRect {
                    x: 1,
                    y: 2,
                    width: 5,
                    height: 7,
                },
                color: [40, 167, 69, 255],
                stroke_width: 4,
                filled: false,
            }
        );
        assert_eq!(
            annotations[7],
            ImageAnnotation::Text {
                position: PhysicalPoint { x: 7, y: 9 },
                text: "Snap".to_string(),
                color: [255, 255, 255, 255],
                font_size: 24,
            }
        );
    }
}

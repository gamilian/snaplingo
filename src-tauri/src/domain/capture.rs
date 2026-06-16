use serde::{Deserialize, Serialize};

/// Screen capture mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaptureMode {
    Region,
    Window,
    Fullscreen,
}

/// Configuration for screen capture
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureConfig {
    pub mode: CaptureMode,
    pub region: Option<CaptureRegion>,
    pub format: ImageFormat,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            mode: CaptureMode::Region,
            region: None,
            format: ImageFormat::Png,
        }
    }
}

/// Screen region coordinates
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CaptureRegion {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Rectangle in frontend logical pixels.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogicalRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Point in frontend logical pixels.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogicalPoint {
    pub x: f64,
    pub y: f64,
}

/// Rectangle in physical image pixels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhysicalRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Point in physical image pixels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhysicalPoint {
    pub x: i32,
    pub y: i32,
}

/// Identifier for a frozen screenshot capture session.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CaptureSessionId(pub String);

/// Frontend-safe monitor snapshot metadata and image data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonitorSnapshotView {
    pub id: String,
    pub logical_bounds: LogicalRect,
    pub physical_bounds: PhysicalRect,
    pub scale_factor: f64,
    pub image_base64: String,
}

/// Frontend hover target candidate captured at session start.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaptureCandidateView {
    pub id: String,
    pub kind: String,
    pub rect: LogicalRect,
    pub priority: i32,
}

/// View returned to the frontend after a capture session is created.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaptureSessionView {
    pub id: CaptureSessionId,
    pub monitors: Vec<MonitorSnapshotView>,
    pub candidates: Vec<CaptureCandidateView>,
}

/// Frontend-safe pinned image metadata and image data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinnedImageView {
    pub id: String,
    pub image_base64: String,
    pub width: u32,
    pub height: u32,
}

/// Output action requested for a capture session selection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum CaptureOutputAction {
    Copy,
    Save { path: String },
    Pin,
}

/// Vector annotation command in selection-local logical pixels.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AnnotationCommand {
    Rectangle {
        rect: LogicalRect,
        color: [u8; 4],
        stroke_width: u32,
    },
    Ellipse {
        rect: LogicalRect,
        color: [u8; 4],
        stroke_width: u32,
    },
    Arrow {
        start: LogicalPoint,
        end: LogicalPoint,
        color: [u8; 4],
        stroke_width: u32,
    },
    Line {
        start: LogicalPoint,
        end: LogicalPoint,
        color: [u8; 4],
        stroke_width: u32,
    },
    Freehand {
        points: Vec<LogicalPoint>,
        color: [u8; 4],
        stroke_width: u32,
    },
    Highlight {
        points: Vec<LogicalPoint>,
        color: [u8; 4],
        stroke_width: u32,
    },
    Mosaic {
        rect: LogicalRect,
        block_size: u32,
    },
}

/// Supported image formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Webp,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_session_id_serializes_as_string() {
        let id = CaptureSessionId("session-1".to_string());

        let serialized = serde_json::to_string(&id).unwrap();

        assert_eq!(serialized, "\"session-1\"");
    }

    #[test]
    fn capture_output_action_serializes_with_type_tag() {
        let action = CaptureOutputAction::Save {
            path: "/tmp/snap.png".to_string(),
        };

        let serialized = serde_json::to_value(&action).unwrap();

        assert_eq!(serialized["type"], "save");
        assert_eq!(serialized["path"], "/tmp/snap.png");
    }

    #[test]
    fn rectangle_annotation_serializes_with_type_tag() {
        let annotation = AnnotationCommand::Rectangle {
            rect: LogicalRect {
                x: 1.0,
                y: 2.0,
                width: 3.0,
                height: 4.0,
            },
            color: [255, 0, 0, 255],
            stroke_width: 2,
        };

        let serialized = serde_json::to_value(&annotation).unwrap();

        assert_eq!(serialized["type"], "rectangle");
        assert_eq!(serialized["stroke_width"], 2);
    }

    #[test]
    fn arrow_annotation_serializes_with_type_tag() {
        let annotation = AnnotationCommand::Arrow {
            start: LogicalPoint { x: 1.0, y: 2.0 },
            end: LogicalPoint { x: 3.0, y: 4.0 },
            color: [255, 0, 0, 255],
            stroke_width: 2,
        };

        let serialized = serde_json::to_value(&annotation).unwrap();

        assert_eq!(serialized["type"], "arrow");
        assert_eq!(serialized["start"]["x"], 1.0);
        assert_eq!(serialized["end"]["y"], 4.0);
        assert_eq!(serialized["stroke_width"], 2);
    }

    #[test]
    fn line_annotation_serializes_with_type_tag() {
        let annotation = AnnotationCommand::Line {
            start: LogicalPoint { x: 1.0, y: 2.0 },
            end: LogicalPoint { x: 3.0, y: 4.0 },
            color: [255, 0, 0, 255],
            stroke_width: 2,
        };

        let serialized = serde_json::to_value(&annotation).unwrap();

        assert_eq!(serialized["type"], "line");
        assert_eq!(serialized["start"]["x"], 1.0);
        assert_eq!(serialized["end"]["y"], 4.0);
        assert_eq!(serialized["stroke_width"], 2);
    }

    #[test]
    fn freehand_annotation_serializes_with_type_tag() {
        let annotation = AnnotationCommand::Freehand {
            points: vec![
                LogicalPoint { x: 1.0, y: 2.0 },
                LogicalPoint { x: 3.0, y: 4.0 },
            ],
            color: [255, 0, 0, 255],
            stroke_width: 2,
        };

        let serialized = serde_json::to_value(&annotation).unwrap();

        assert_eq!(serialized["type"], "freehand");
        assert_eq!(serialized["points"][0]["x"], 1.0);
        assert_eq!(serialized["points"][1]["y"], 4.0);
        assert_eq!(serialized["stroke_width"], 2);
    }

    #[test]
    fn highlight_annotation_serializes_with_type_tag() {
        let annotation = AnnotationCommand::Highlight {
            points: vec![
                LogicalPoint { x: 1.0, y: 2.0 },
                LogicalPoint { x: 3.0, y: 4.0 },
            ],
            color: [255, 230, 0, 96],
            stroke_width: 6,
        };

        let serialized = serde_json::to_value(&annotation).unwrap();

        assert_eq!(serialized["type"], "highlight");
        assert_eq!(serialized["points"][0]["x"], 1.0);
        assert_eq!(serialized["points"][1]["y"], 4.0);
        assert_eq!(serialized["color"][3], 96);
        assert_eq!(serialized["stroke_width"], 6);
    }

    #[test]
    fn mosaic_annotation_serializes_with_type_tag() {
        let annotation = AnnotationCommand::Mosaic {
            rect: LogicalRect {
                x: 1.0,
                y: 2.0,
                width: 3.0,
                height: 4.0,
            },
            block_size: 6,
        };

        let serialized = serde_json::to_value(&annotation).unwrap();

        assert_eq!(serialized["type"], "mosaic");
        assert_eq!(serialized["rect"]["x"], 1.0);
        assert_eq!(serialized["block_size"], 6);
    }

    #[test]
    fn ellipse_annotation_serializes_with_type_tag() {
        let annotation = AnnotationCommand::Ellipse {
            rect: LogicalRect {
                x: 1.0,
                y: 2.0,
                width: 6.0,
                height: 4.0,
            },
            color: [255, 0, 0, 255],
            stroke_width: 2,
        };

        let serialized = serde_json::to_value(&annotation).unwrap();

        assert_eq!(serialized["type"], "ellipse");
        assert_eq!(serialized["rect"]["width"], 6.0);
        assert_eq!(serialized["stroke_width"], 2);
    }
}

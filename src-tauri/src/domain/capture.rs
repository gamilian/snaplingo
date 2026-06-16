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

/// Rectangle in physical image pixels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhysicalRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
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

/// View returned to the frontend after a capture session is created.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaptureSessionView {
    pub id: CaptureSessionId,
    pub monitors: Vec<MonitorSnapshotView>,
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

/// Placeholder for vector annotation commands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnnotationCommand {
    pub kind: String,
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
}

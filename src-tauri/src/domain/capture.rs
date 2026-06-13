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

/// Supported image formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Webp,
}

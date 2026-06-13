use crate::error::AppError;

/// Rectangle region for partial screenshot capture
#[derive(Debug, Clone, Copy)]
pub struct ScreenRegion {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Platform-agnostic screenshot backend trait
#[async_trait::async_trait]
pub trait ScreenshotBackend: Send + Sync {
    /// Capture the entire screen
    /// Returns PNG-encoded image data
    async fn capture_full_screen(&self) -> Result<Vec<u8>, AppError>;

    /// Capture a specific region of the screen
    /// Returns PNG-encoded image data
    async fn capture_region(&self, region: ScreenRegion) -> Result<Vec<u8>, AppError>;
}

use crate::error::AppError;
use super::backend::{ScreenshotBackend, ScreenRegion};

pub struct WindowsScreenshotBackend;

impl WindowsScreenshotBackend {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl ScreenshotBackend for WindowsScreenshotBackend {
    async fn capture_full_screen(&self) -> Result<Vec<u8>, AppError> {
        // Placeholder implementation
        // TODO: Implement actual screenshot capture in Phase 4
        Ok(vec![])
    }

    async fn capture_region(&self, _region: ScreenRegion) -> Result<Vec<u8>, AppError> {
        // Placeholder implementation
        // TODO: Implement actual screenshot capture in Phase 4
        Ok(vec![])
    }
}

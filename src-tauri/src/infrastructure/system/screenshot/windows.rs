use crate::error::AppError;
use super::backend::{ScreenshotBackend, ScreenRegion};
use super::xcap_common;

/// Windows screenshot backend using the cross-platform XCap crate.
pub struct WindowsScreenshotBackend;

impl WindowsScreenshotBackend {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl ScreenshotBackend for WindowsScreenshotBackend {
    async fn capture_full_screen(&self) -> Result<Vec<u8>, AppError> {
        xcap_common::capture_full_screen_png()
    }

    async fn capture_region(&self, region: ScreenRegion) -> Result<Vec<u8>, AppError> {
        xcap_common::capture_region_png(region)
    }
}

use super::backend::{ScreenRegion, ScreenshotBackend};
use super::xcap_common;
use crate::error::AppError;

/// Windows screenshot backend using the cross-platform XCap crate.
pub struct WindowsScreenshotBackend;

impl WindowsScreenshotBackend {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl ScreenshotBackend for WindowsScreenshotBackend {
    async fn capture_monitor_snapshots(
        &self,
    ) -> Result<Vec<super::backend::MonitorSnapshot>, AppError> {
        xcap_common::capture_all_monitor_snapshots()
    }

    async fn capture_full_screen(&self) -> Result<Vec<u8>, AppError> {
        xcap_common::capture_full_screen_png()
    }

    async fn capture_region(&self, region: ScreenRegion) -> Result<Vec<u8>, AppError> {
        xcap_common::capture_region_png(region)
    }
}

use super::backend::{ScreenRegion, ScreenshotBackend};
use super::xcap_common;
use crate::error::AppError;

/// Linux screenshot backend using the cross-platform XCap crate.
///
/// XCap handles X11 and Wayland differences internally. On Wayland, the
/// compositor must support a screenshot protocol (e.g. via xdg-desktop-portal).
pub struct LinuxScreenshotBackend;

impl LinuxScreenshotBackend {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl ScreenshotBackend for LinuxScreenshotBackend {
    async fn capture_monitor_snapshots(
        &self,
    ) -> Result<Vec<super::backend::MonitorSnapshot>, AppError> {
        Ok(vec![xcap_common::capture_primary_monitor_snapshot()?])
    }

    async fn capture_full_screen(&self) -> Result<Vec<u8>, AppError> {
        xcap_common::capture_full_screen_png()
    }

    async fn capture_region(&self, region: ScreenRegion) -> Result<Vec<u8>, AppError> {
        xcap_common::capture_region_png(region)
    }
}

use crate::error::AppError;
use super::backend::{ScreenshotBackend, ScreenRegion};
use super::xcap_common;

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
    async fn capture_full_screen(&self) -> Result<Vec<u8>, AppError> {
        xcap_common::capture_full_screen_png()
    }

    async fn capture_region(&self, region: ScreenRegion) -> Result<Vec<u8>, AppError> {
        xcap_common::capture_region_png(region)
    }
}

use super::backend::{
    MonitorLayout, MonitorSnapshot, ScreenRegion, ScreenshotBackend, WindowCandidate,
};
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
    async fn capture_monitor_snapshots(&self) -> Result<Vec<MonitorSnapshot>, AppError> {
        xcap_common::capture_all_monitor_snapshots()
    }

    async fn capture_monitor_layouts(&self) -> Result<Vec<MonitorLayout>, AppError> {
        xcap_common::capture_all_monitor_layouts()
    }

    async fn capture_window_candidates(
        &self,
        monitors: &[MonitorSnapshot],
    ) -> Result<Vec<WindowCandidate>, AppError> {
        xcap_common::capture_window_candidates(monitors)
    }

    async fn capture_region(&self, region: ScreenRegion) -> Result<Vec<u8>, AppError> {
        xcap_common::capture_region_png(region)
    }
}

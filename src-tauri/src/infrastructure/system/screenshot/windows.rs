use super::xcap_common;
use crate::application::CaptureSessionSource;
use crate::domain::capture::{MonitorLayout, MonitorSnapshot, ScreenRegion, WindowCandidate};
use crate::error::AppError;

/// Windows screenshot backend using the cross-platform XCap crate.
pub struct WindowsCaptureSessionSource;

impl WindowsCaptureSessionSource {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl CaptureSessionSource for WindowsCaptureSessionSource {
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

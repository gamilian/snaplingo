use crate::domain::capture::{
    CapturedCursor, LogicalPoint, MonitorLayout, MonitorSnapshot, ScreenRegion, WindowCandidate,
};
use crate::error::AppError;

/// Supplies portable desktop data required to create and hydrate Capture Sessions.
#[async_trait::async_trait]
pub trait CaptureSessionSource: Send + Sync {
    async fn capture_monitor_snapshots(&self) -> Result<Vec<MonitorSnapshot>, AppError>;

    async fn capture_monitor_layouts(&self) -> Result<Vec<MonitorLayout>, AppError>;

    async fn capture_window_candidates(
        &self,
        _monitors: &[MonitorSnapshot],
    ) -> Result<Vec<WindowCandidate>, AppError> {
        Ok(Vec::new())
    }

    async fn capture_cursor(
        &self,
        _monitors: &[MonitorSnapshot],
    ) -> Result<Option<CapturedCursor>, AppError> {
        Ok(None)
    }

    fn current_cursor_position(
        &self,
        _monitors: &[MonitorSnapshot],
    ) -> Result<Option<LogicalPoint>, AppError> {
        Ok(None)
    }

    async fn capture_region(&self, region: ScreenRegion) -> Result<Vec<u8>, AppError>;
}

use super::CaptureCoordinatePolicy;
use crate::domain::capture::{
    CapturedCursor, ControlCandidate, LogicalPoint, MonitorLayout, MonitorSnapshot, ScreenRegion,
    WindowCandidate,
};
use crate::error::AppError;

/// Supplies portable desktop data required to create and hydrate Capture Sessions.
#[async_trait::async_trait]
pub trait CaptureSessionSource: Send + Sync {
    /// A platform may require the selection window to stay on one display.
    /// Sample this before capturing pixels so the trigger's display is retained.
    fn selection_monitor_id(&self) -> Result<Option<String>, AppError> {
        Ok(None)
    }

    fn coordinate_policy(&self) -> CaptureCoordinatePolicy {
        CaptureCoordinatePolicy::NativeLogical
    }

    async fn capture_monitor_snapshots(&self) -> Result<Vec<MonitorSnapshot>, AppError>;

    async fn capture_monitor_snapshot(
        &self,
        monitor_id: &str,
    ) -> Result<MonitorSnapshot, AppError> {
        self.capture_monitor_snapshots()
            .await?
            .into_iter()
            .find(|snapshot| snapshot.id == monitor_id)
            .ok_or_else(|| AppError::System(format!("Capture monitor not found: {monitor_id}")))
    }

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

    async fn capture_control_candidate(
        &self,
        _point: &LogicalPoint,
        _monitors: &[MonitorSnapshot],
    ) -> Result<Option<ControlCandidate>, AppError> {
        Err(AppError::System("当前平台暂不支持界面元素检测".to_string()))
    }

    fn current_cursor_position(
        &self,
        _monitors: &[MonitorSnapshot],
    ) -> Result<Option<LogicalPoint>, AppError> {
        Ok(None)
    }

    async fn capture_region(&self, region: ScreenRegion) -> Result<Vec<u8>, AppError>;
}

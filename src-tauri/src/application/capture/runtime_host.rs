use async_trait::async_trait;

use crate::domain::capture::{LogicalRect, MonitorSnapshotView};
use crate::Result;

pub trait CaptureCursorMover: Send + Sync {
    fn move_relative(&self, delta_x: i32, delta_y: i32);
}

#[async_trait]
pub(crate) trait CaptureSessionRuntimeHost: Send + Sync {
    async fn begin_capture_presentation(&self) -> Result<()>;
    async fn end_capture_presentation(&self) -> Result<()>;
    async fn prepare_capture_window_for_reveal(&self) -> Result<()>;
    async fn reveal_capture_window(&self) -> Result<()>;
    async fn hide_capture_window(&self) -> Result<()>;
    async fn set_capture_window_cursor_passthrough(&self, _enabled: bool) -> Result<()> {
        Ok(())
    }
    async fn destroy_inactive_capture_window(&self) -> Result<()>;
    async fn open_capture_window_for_session(
        &self,
        mode: &str,
        session_id: &str,
        bounds: &LogicalRect,
    ) -> Result<()>;
    async fn restore_capture_snapshot_windows(
        &self,
        hidden_window_labels: Vec<String>,
    ) -> Result<()>;
    fn capture_window_bounds(&self, monitors: &[MonitorSnapshotView]) -> Option<LogicalRect>;
}

pub(crate) struct UnconfiguredCaptureSessionRuntimeHost;

#[async_trait]
impl CaptureSessionRuntimeHost for UnconfiguredCaptureSessionRuntimeHost {
    async fn begin_capture_presentation(&self) -> Result<()> {
        Err("Capture session host is not configured".into())
    }

    async fn end_capture_presentation(&self) -> Result<()> {
        Err("Capture session host is not configured".into())
    }

    async fn prepare_capture_window_for_reveal(&self) -> Result<()> {
        Err("Capture session host is not configured".into())
    }

    async fn reveal_capture_window(&self) -> Result<()> {
        Err("Capture session host is not configured".into())
    }

    async fn hide_capture_window(&self) -> Result<()> {
        Err("Capture session host is not configured".into())
    }

    async fn destroy_inactive_capture_window(&self) -> Result<()> {
        Err("Capture session host is not configured".into())
    }

    async fn open_capture_window_for_session(
        &self,
        _mode: &str,
        _session_id: &str,
        _bounds: &LogicalRect,
    ) -> Result<()> {
        Err("Capture session host is not configured".into())
    }

    async fn restore_capture_snapshot_windows(
        &self,
        _hidden_window_labels: Vec<String>,
    ) -> Result<()> {
        Err("Capture session host is not configured".into())
    }

    fn capture_window_bounds(&self, _monitors: &[MonitorSnapshotView]) -> Option<LogicalRect> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeCaptureSessionRuntimeHost;

    #[async_trait]
    impl CaptureSessionRuntimeHost for FakeCaptureSessionRuntimeHost {
        async fn begin_capture_presentation(&self) -> Result<()> {
            Ok(())
        }

        async fn end_capture_presentation(&self) -> Result<()> {
            Ok(())
        }

        async fn prepare_capture_window_for_reveal(&self) -> Result<()> {
            Ok(())
        }

        async fn reveal_capture_window(&self) -> Result<()> {
            Ok(())
        }

        async fn hide_capture_window(&self) -> Result<()> {
            Ok(())
        }

        async fn destroy_inactive_capture_window(&self) -> Result<()> {
            Ok(())
        }

        async fn open_capture_window_for_session(
            &self,
            _mode: &str,
            _session_id: &str,
            _bounds: &LogicalRect,
        ) -> Result<()> {
            Ok(())
        }

        async fn restore_capture_snapshot_windows(
            &self,
            _hidden_window_labels: Vec<String>,
        ) -> Result<()> {
            Ok(())
        }

        fn capture_window_bounds(&self, _monitors: &[MonitorSnapshotView]) -> Option<LogicalRect> {
            Some(LogicalRect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 100.0,
            })
        }
    }

    #[tokio::test]
    async fn fake_capture_session_runtime_host_can_drive_runtime_port() {
        let host = FakeCaptureSessionRuntimeHost;
        host.begin_capture_presentation().await.unwrap();

        assert!(host.capture_window_bounds(&[]).is_some());
    }
}

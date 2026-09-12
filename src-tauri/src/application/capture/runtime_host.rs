use async_trait::async_trait;

use super::ocr_completion::{CaptureOcrStatus, CaptureOcrTarget};
use super::CaptureWindowGeometry;
use crate::domain::capture::CaptureSessionId;
use crate::domain::ocr::OcrResult;
use crate::Result;

pub trait CaptureCursorMover: Send + Sync {
    fn move_relative(&self, delta_x: i32, delta_y: i32);
}

#[async_trait]
pub(crate) trait CapturePinOutput: Send + Sync {
    async fn pin_png(&self, png_data: Vec<u8>) -> Result<()>;
}

pub(crate) struct UnconfiguredCapturePinOutput;

#[async_trait]
impl CapturePinOutput for UnconfiguredCapturePinOutput {
    async fn pin_png(&self, _png_data: Vec<u8>) -> Result<()> {
        Err("Capture pin output is not configured".into())
    }
}

#[async_trait]
pub(crate) trait CaptureSessionRuntimeHost: Send + Sync {
    fn clipboard_revision(&self) -> u64 {
        0
    }
    fn reserve_capture_ocr_result(&self, _target: CaptureOcrTarget) -> Result<u64> {
        Ok(0)
    }
    fn set_capture_ocr_status(&self, _status: Option<CaptureOcrStatus>) {}
    async fn publish_capture_ocr(
        &self,
        _target: CaptureOcrTarget,
        _result_id: u64,
        _result: OcrResult,
        _preview: Option<String>,
    ) -> Result<()> {
        Err("Capture OCR output is not configured".into())
    }
    async fn report_capture_ocr_error(
        &self,
        _target: CaptureOcrTarget,
        _result_id: u64,
        error: &str,
    ) {
        log::error!("Capture OCR failed: {error}");
    }
    async fn begin_capture_presentation(&self) -> Result<()>;
    async fn end_capture_presentation(&self) -> Result<()>;
    /// No geometry is supplied when revealing a session-load error.
    async fn prepare_capture_window_for_reveal(
        &self,
        session_id: Option<&CaptureSessionId>,
        geometry: Option<&CaptureWindowGeometry>,
    ) -> Result<()>;
    async fn reveal_capture_window(&self, geometry: Option<&CaptureWindowGeometry>) -> Result<()>;
    /// Hiding an absent or already hidden overlay succeeds.
    async fn hide_capture_window(&self) -> Result<()>;
    async fn set_capture_window_cursor_passthrough(&self, _enabled: bool) -> Result<()> {
        Ok(())
    }
    async fn destroy_inactive_capture_window(&self) -> Result<()>;
    async fn open_capture_window_for_session(
        &self,
        mode: &str,
        session_id: &str,
        geometry: &CaptureWindowGeometry,
    ) -> Result<()>;
    async fn restore_capture_snapshot_windows(
        &self,
        hidden_window_labels: Vec<String>,
    ) -> Result<()>;
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

    async fn prepare_capture_window_for_reveal(
        &self,
        _session_id: Option<&CaptureSessionId>,
        _geometry: Option<&CaptureWindowGeometry>,
    ) -> Result<()> {
        Err("Capture session host is not configured".into())
    }

    async fn reveal_capture_window(&self, _geometry: Option<&CaptureWindowGeometry>) -> Result<()> {
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
        _geometry: &CaptureWindowGeometry,
    ) -> Result<()> {
        Err("Capture session host is not configured".into())
    }

    async fn restore_capture_snapshot_windows(
        &self,
        _hidden_window_labels: Vec<String>,
    ) -> Result<()> {
        Err("Capture session host is not configured".into())
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

        async fn prepare_capture_window_for_reveal(
            &self,
            _session_id: Option<&CaptureSessionId>,
            _geometry: Option<&CaptureWindowGeometry>,
        ) -> Result<()> {
            Ok(())
        }

        async fn reveal_capture_window(
            &self,
            _geometry: Option<&CaptureWindowGeometry>,
        ) -> Result<()> {
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
            _geometry: &CaptureWindowGeometry,
        ) -> Result<()> {
            Ok(())
        }

        async fn restore_capture_snapshot_windows(
            &self,
            _hidden_window_labels: Vec<String>,
        ) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn fake_capture_session_runtime_host_can_drive_runtime_port() {
        let host = FakeCaptureSessionRuntimeHost;
        host.begin_capture_presentation().await.unwrap();

        host.open_capture_window_for_session(
            "screenshot",
            "session",
            &CaptureWindowGeometry {
                bounds: crate::domain::capture::LogicalRect {
                    x: -100.0,
                    y: 0.0,
                    width: 200.0,
                    height: 100.0,
                },
                desktop_scale: Some(2.0),
            },
        )
        .await
        .unwrap();
    }
}

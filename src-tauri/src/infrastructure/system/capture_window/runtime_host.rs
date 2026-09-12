use async_trait::async_trait;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

use crate::application::capture::{CaptureOcrStatus, CaptureOcrTarget};
use crate::application::capture::{CaptureSessionRuntimeHost, CaptureWindowGeometry};
use crate::application::result_window::{ResultWindowOpenRequest, ResultWindowRequestId};
use crate::domain::capture::CaptureSessionId;
use crate::domain::ocr::OcrResult;
use crate::Result;

use super::tauri::{bind_capture_window_session, set_capture_window_cursor_passthrough};
use super::{
    begin_capture_presentation, destroy_inactive_capture_window, end_capture_presentation,
    hide_capture_window, open_capture_window_for_session, prepare_capture_window_for_reveal,
    restore_capture_snapshot_windows, reveal_capture_window,
};

pub(crate) struct TauriCaptureSessionRuntimeHost {
    app: AppHandle,
}

impl TauriCaptureSessionRuntimeHost {
    pub(crate) fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

#[async_trait]
impl CaptureSessionRuntimeHost for TauriCaptureSessionRuntimeHost {
    fn clipboard_revision(&self) -> u64 {
        #[cfg(target_os = "macos")]
        {
            objc2_app_kit::NSPasteboard::generalPasteboard().changeCount() as u64
        }
        #[cfg(not(target_os = "macos"))]
        {
            0
        }
    }

    fn reserve_capture_ocr_result(&self, target: CaptureOcrTarget) -> Result<u64> {
        if target == CaptureOcrTarget::Clipboard {
            return Ok(0);
        }
        self.app
            .state::<crate::AppState>()
            .result_window
            .reserve_request()
            .map(|id| id.0)
    }

    fn set_capture_ocr_status(&self, status: Option<CaptureOcrStatus>) {
        crate::app_shell::set_capture_ocr_status(&self.app, status);
    }

    async fn publish_capture_ocr(
        &self,
        target: CaptureOcrTarget,
        result_id: u64,
        result: OcrResult,
        preview: Option<String>,
    ) -> Result<()> {
        if target == CaptureOcrTarget::Clipboard {
            let mut clipboard = arboard::Clipboard::new()
                .map_err(|error| format!("Failed to open clipboard: {error}"))?;
            return clipboard
                .set_text(result.text)
                .map_err(|error| format!("Failed to copy OCR text: {error}").into());
        }
        let request = if target == CaptureOcrTarget::TranslationWindow {
            ResultWindowOpenRequest::screenshot_translation(result.text, result.detected_language)
        } else {
            ResultWindowOpenRequest::capture_ocr(result.text, preview, result.confidence)
        };
        self.app
            .state::<crate::AppState>()
            .result_window
            .open_reserved(ResultWindowRequestId(result_id), request)
            .await
    }

    async fn report_capture_ocr_error(
        &self,
        target: CaptureOcrTarget,
        result_id: u64,
        error: &str,
    ) {
        log::error!("Capture OCR failed: {error}");
        if target != CaptureOcrTarget::Clipboard
            && !self
                .app
                .state::<crate::AppState>()
                .result_window
                .is_current(ResultWindowRequestId(result_id))
                .unwrap_or(false)
        {
            return;
        }
        self.app
            .dialog()
            .message(error)
            .title("SnapLingo — OCR")
            .kind(MessageDialogKind::Error)
            .show(|_| {});
    }

    async fn begin_capture_presentation(&self) -> Result<()> {
        run_on_main_thread(&self.app, "begin capture presentation", |app| {
            begin_capture_presentation(&app)
        })
        .await
    }

    async fn end_capture_presentation(&self) -> Result<()> {
        run_on_main_thread(&self.app, "end capture presentation", |app| {
            end_capture_presentation(&app)
        })
        .await
    }

    async fn prepare_capture_window_for_reveal(
        &self,
        session_id: Option<&CaptureSessionId>,
        geometry: Option<&CaptureWindowGeometry>,
    ) -> Result<()> {
        let geometry = geometry.cloned();
        let session_id = session_id.cloned();
        run_on_main_thread(&self.app, "prepare capture window for reveal", move |app| {
            if let Some(session_id) = session_id {
                bind_capture_window_session(&app, &session_id.0);
            }
            prepare_capture_window_for_reveal(&app, geometry.as_ref())
        })
        .await
    }

    async fn reveal_capture_window(&self, geometry: Option<&CaptureWindowGeometry>) -> Result<()> {
        let geometry = geometry.cloned();
        run_on_main_thread(&self.app, "reveal capture window", move |app| {
            reveal_capture_window(&app, geometry.as_ref())
        })
        .await
    }

    async fn hide_capture_window(&self) -> Result<()> {
        run_on_main_thread(&self.app, "hide capture window", |app| {
            hide_capture_window(&app)
        })
        .await
    }

    async fn set_capture_window_cursor_passthrough(&self, enabled: bool) -> Result<()> {
        run_on_main_thread(
            &self.app,
            "set capture window cursor passthrough",
            move |app| set_capture_window_cursor_passthrough(&app, enabled),
        )
        .await
    }

    async fn destroy_inactive_capture_window(&self) -> Result<()> {
        run_on_main_thread(&self.app, "destroy inactive capture window", |app| {
            destroy_inactive_capture_window(&app)
        })
        .await
    }

    async fn open_capture_window_for_session(
        &self,
        mode: &str,
        session_id: &str,
        geometry: &CaptureWindowGeometry,
    ) -> Result<()> {
        let mode = mode.to_string();
        let session_id = session_id.to_string();
        let geometry = geometry.clone();

        run_on_main_thread(&self.app, "open capture window", move |app| {
            open_capture_window_for_session(&app, &mode, &session_id, &geometry)
        })
        .await
    }

    async fn restore_capture_snapshot_windows(
        &self,
        hidden_window_labels: Vec<String>,
    ) -> Result<()> {
        run_on_main_thread(&self.app, "restore capture snapshot windows", move |app| {
            restore_capture_snapshot_windows(&app, &hidden_window_labels)
        })
        .await
    }
}

async fn run_on_main_thread<T, F>(
    app: &AppHandle,
    operation_name: &'static str,
    operation: F,
) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce(AppHandle) -> std::result::Result<T, String> + Send + 'static,
{
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let app_for_operation = app.clone();
    app.run_on_main_thread(move || {
        let _ = sender.send(operation(app_for_operation));
    })
    .map_err(|error| format!("Failed to dispatch {operation_name}: {error}"))?;

    receiver
        .await
        .map_err(|error| format!("Failed to receive {operation_name} result: {error}"))?
        .map_err(Into::into)
}

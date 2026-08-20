use async_trait::async_trait;
use tauri::AppHandle;

use crate::application::capture::CaptureSessionRuntimeHost;
use crate::domain::capture::LogicalRect;
use crate::Result;

use super::tauri::set_capture_window_cursor_passthrough;
use super::{
    begin_capture_presentation, capture_window_bounds, destroy_inactive_capture_window,
    end_capture_presentation, hide_capture_window, open_capture_window_for_session,
    prepare_capture_window_for_reveal, restore_capture_snapshot_windows, reveal_capture_window,
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

    async fn prepare_capture_window_for_reveal(&self) -> Result<()> {
        run_on_main_thread(&self.app, "prepare capture window for reveal", |app| {
            prepare_capture_window_for_reveal(&app)
        })
        .await
    }

    async fn reveal_capture_window(&self) -> Result<()> {
        run_on_main_thread(&self.app, "reveal capture window", |app| {
            reveal_capture_window(&app)
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
        bounds: &LogicalRect,
    ) -> Result<()> {
        let mode = mode.to_string();
        let session_id = session_id.to_string();
        let bounds = bounds.clone();

        run_on_main_thread(&self.app, "open capture window", move |app| {
            open_capture_window_for_session(&app, &mode, &session_id, &bounds)
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

    fn capture_window_bounds(
        &self,
        monitors: &[crate::domain::capture::MonitorSnapshotView],
    ) -> Option<LogicalRect> {
        capture_window_bounds(monitors)
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

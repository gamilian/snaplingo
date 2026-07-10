use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use tauri::AppHandle;

use crate::application::providers::ocr::OcrCoordinator;
use crate::application::services::capture_session_render::{
    output_capture_selection, recognize_capture_selection_text, render_capture_png_base64,
};
use crate::application::services::{
    CaptureOutputService, CaptureSessionOutput, CaptureSessionService, ImageCompositionService,
};
use crate::domain::capture::{
    AnnotationCommand, CaptureOutputAction, CaptureSessionId, CaptureSessionView, LogicalRect,
};
use crate::domain::ocr::OcrResult;
use crate::infrastructure::system::capture_window::{
    begin_capture_presentation, capture_window_bounds, destroy_inactive_capture_window,
    end_capture_presentation, hide_capture_window, open_capture_window_for_session,
    restore_capture_snapshot_windows,
};
use crate::Result;

#[async_trait]
pub(crate) trait CaptureSessionRuntimeHost: Send + Sync {
    async fn begin_capture_presentation(&self) -> Result<()>;
    async fn end_capture_presentation(&self) -> Result<()>;
    async fn hide_capture_window(&self) -> Result<()>;
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
}

struct UnconfiguredCaptureSessionRuntimeHost;

#[async_trait]
impl CaptureSessionRuntimeHost for UnconfiguredCaptureSessionRuntimeHost {
    async fn begin_capture_presentation(&self) -> Result<()> {
        Err("Capture session host is not configured".into())
    }

    async fn end_capture_presentation(&self) -> Result<()> {
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
}

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

    async fn hide_capture_window(&self) -> Result<()> {
        run_on_main_thread(&self.app, "hide capture window", |app| {
            hide_capture_window(&app)
        })
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
}

/// Coordinates Capture Session operations that need several application services.
pub struct CaptureSessionRuntime {
    sessions: Arc<CaptureSessionService>,
    image_composition: Arc<ImageCompositionService>,
    output: Arc<CaptureOutputService>,
    ocr: Arc<OcrCoordinator>,
    host: Arc<dyn CaptureSessionRuntimeHost>,
}

impl CaptureSessionRuntime {
    pub fn new(
        sessions: Arc<CaptureSessionService>,
        image_composition: Arc<ImageCompositionService>,
        output: Arc<CaptureOutputService>,
        ocr: Arc<OcrCoordinator>,
    ) -> Self {
        Self::with_host(
            sessions,
            image_composition,
            output,
            ocr,
            Arc::new(UnconfiguredCaptureSessionRuntimeHost),
        )
    }

    pub(crate) fn with_host(
        sessions: Arc<CaptureSessionService>,
        image_composition: Arc<ImageCompositionService>,
        output: Arc<CaptureOutputService>,
        ocr: Arc<OcrCoordinator>,
        host: Arc<dyn CaptureSessionRuntimeHost>,
    ) -> Self {
        Self {
            sessions,
            image_composition,
            output,
            ocr,
            host,
        }
    }

    pub async fn create_session_from_visible_desktop(&self) -> Result<CaptureSessionView> {
        let total_start = Instant::now();
        let begin_start = Instant::now();
        self.host.begin_capture_presentation().await?;
        let begin_ms = elapsed_ms(begin_start);

        let hide_overlay_start = Instant::now();
        if let Err(err) = self.host.hide_capture_window().await {
            if err.to_string() != "Capture window is not open" {
                let presentation_result = self.host.end_capture_presentation().await;
                return match presentation_result {
                    Ok(()) => Err(err),
                    Err(presentation_err) => Err(format!(
                        "{}; also failed to end capture presentation: {}",
                        err, presentation_err
                    )
                    .into()),
                };
            }
        }
        let hide_overlay_ms = elapsed_ms(hide_overlay_start);

        let session_start = Instant::now();
        let session_result = self.sessions.create_session_without_monitor_images().await;
        let session_ms = elapsed_ms(session_start);

        log::info!(
            "[capture-perf] create_visible_desktop_frozen_session begin_ms={:.1} hide_overlay_ms={:.1} capture_session_ms={:.1} total_ms={:.1} success={}",
            begin_ms,
            hide_overlay_ms,
            session_ms,
            elapsed_ms(total_start),
            session_result.is_ok(),
        );

        match session_result {
            Ok(session) => Ok(session),
            Err(session_err) => {
                let presentation_result = self.host.end_capture_presentation().await;

                match presentation_result {
                    Ok(()) => Err(session_err),
                    Err(presentation_err) => Err(format!(
                        "{}; also failed to end capture presentation: {}",
                        session_err, presentation_err
                    )
                    .into()),
                }
            }
        }
    }

    pub async fn open_capture_window_for_mode(&self, mode: &str) -> Result<()> {
        let total_start = Instant::now();
        let session_start = Instant::now();
        let session = self.create_session_from_visible_desktop().await?;
        let session_ms = elapsed_ms(session_start);
        let monitor_count = session.monitors.len();
        let candidate_count = session.candidates.len();
        let view_base64_bytes = capture_session_view_base64_bytes(&session);

        let open_start = Instant::now();
        let open_result = match capture_window_bounds(&session.monitors) {
            Some(bounds) => {
                self.host
                    .open_capture_window_for_session(mode, &session.id.0, &bounds)
                    .await
            }
            None => Err("Cannot open capture window without monitor bounds".into()),
        };
        let open_ms = elapsed_ms(open_start);

        log::info!(
            "[capture-perf] open_capture_window mode={} monitors={} candidates={} view_base64_bytes={} create_session_ms={:.1} open_overlay_ms={:.1} total_ms={:.1} success={}",
            mode,
            monitor_count,
            candidate_count,
            view_base64_bytes,
            session_ms,
            open_ms,
            elapsed_ms(total_start),
            open_result.is_ok(),
        );

        if let Err(open_err) = open_result {
            let restore_result = self
                .restore_capture_snapshot_windows_for_session_id(&session.id)
                .await;
            let _ = self.sessions.cancel_session(&session.id);
            let presentation_result = self.host.end_capture_presentation().await;

            if let Err(restore_err) = restore_result {
                return Err(format!(
                    "{}; also failed to restore hidden windows: {}",
                    open_err, restore_err
                )
                .into());
            }

            if let Err(presentation_err) = presentation_result {
                return Err(format!(
                    "{}; also failed to end capture presentation: {}",
                    open_err, presentation_err
                )
                .into());
            }

            return Err(open_err);
        }

        Ok(())
    }

    pub async fn cancel_capture_session(&self, session_id: &CaptureSessionId) -> Result<()> {
        let restore_result = self
            .restore_capture_snapshot_windows_for_session_id(session_id)
            .await;
        let cancel_result = self.sessions.cancel_session(session_id);
        let presentation_result = self.host.end_capture_presentation().await;
        let destroy_window_result = self.host.destroy_inactive_capture_window().await;

        match (
            restore_result,
            cancel_result,
            presentation_result,
            destroy_window_result,
        ) {
            (Ok(()), Ok(()), Ok(()), Ok(())) => Ok(()),
            (Err(err), _, _, _) => Err(err),
            (_, Err(err), _, _) => Err(err),
            (_, _, Err(err), _) => Err(err),
            (_, _, _, Err(err)) => Err(err),
        }
    }

    pub async fn restore_capture_snapshot_windows_for_session(
        &self,
        session_id: &CaptureSessionId,
    ) -> Result<()> {
        let restore_result = self
            .restore_capture_snapshot_windows_for_session_id(session_id)
            .await;
        let presentation_result = self.host.end_capture_presentation().await;
        let destroy_window_result = self.host.destroy_inactive_capture_window().await;

        match (restore_result, presentation_result, destroy_window_result) {
            (Ok(()), Ok(()), Ok(())) => Ok(()),
            (Err(err), Ok(()), Ok(())) => Err(err),
            (Ok(()), Err(err), Ok(())) => Err(err),
            (Ok(()), Ok(()), Err(err)) => Err(err),
            (Err(restore_err), Err(presentation_err), _) => Err(format!(
                "{}; also failed to end capture presentation: {}",
                restore_err, presentation_err
            )
            .into()),
            (Err(restore_err), _, Err(destroy_err)) => Err(format!(
                "{}; also failed to destroy inactive capture window: {}",
                restore_err, destroy_err
            )
            .into()),
            (_, Err(presentation_err), Err(destroy_err)) => Err(format!(
                "{}; also failed to destroy inactive capture window: {}",
                presentation_err, destroy_err
            )
            .into()),
        }
    }

    pub fn render_png_base64(
        &self,
        session_id: &CaptureSessionId,
        rect: &LogicalRect,
        annotations: &[AnnotationCommand],
        include_cursor: bool,
    ) -> Result<String> {
        self.ensure_selection_snapshots_ready(session_id, rect)?;

        render_capture_png_base64(
            &self.sessions,
            &self.image_composition,
            session_id,
            rect,
            annotations,
            include_cursor,
        )
    }

    pub async fn recognize_selection_text(
        &self,
        session_id: &CaptureSessionId,
        rect: &LogicalRect,
    ) -> Result<OcrResult> {
        self.ensure_selection_snapshots_ready(session_id, rect)?;

        recognize_capture_selection_text(
            &self.sessions,
            &self.image_composition,
            &self.ocr,
            session_id,
            rect,
        )
        .await
    }

    pub async fn output_selection(
        &self,
        session_id: &CaptureSessionId,
        rect: &LogicalRect,
        annotations: &[AnnotationCommand],
        include_cursor: bool,
        action: CaptureOutputAction,
    ) -> Result<CaptureSessionOutput> {
        self.ensure_selection_snapshots_ready(session_id, rect)?;

        output_capture_selection(
            &self.sessions,
            &self.image_composition,
            &self.output,
            session_id,
            rect,
            annotations,
            include_cursor,
            action,
        )
        .await
    }

    fn ensure_selection_snapshots_ready(
        &self,
        session_id: &CaptureSessionId,
        rect: &LogicalRect,
    ) -> Result<()> {
        if self
            .sessions
            .session_selection_needs_freeze(session_id, rect)?
        {
            return Err("Capture session snapshots are not ready for the selected area".into());
        }

        Ok(())
    }

    async fn restore_capture_snapshot_windows_for_session_id(
        &self,
        session_id: &CaptureSessionId,
    ) -> Result<()> {
        let hidden_window_labels = self.sessions.take_hidden_window_labels(session_id)?;

        self.host
            .restore_capture_snapshot_windows(hidden_window_labels)
            .await
    }
}

fn capture_session_view_base64_bytes(session: &CaptureSessionView) -> usize {
    session
        .monitors
        .iter()
        .map(|monitor| monitor.image_base64.len())
        .sum::<usize>()
        + session
            .captured_cursor
            .as_ref()
            .map(|cursor| cursor.image_base64.len())
            .unwrap_or_default()
}

fn elapsed_ms(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
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
    .map_err(|e| format!("Failed to dispatch {operation_name}: {e}"))?;

    receiver
        .await
        .map_err(|e| format!("Failed to receive {operation_name} result: {e}"))?
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use image::ImageEncoder;

    use super::{CaptureSessionOutput, CaptureSessionRuntime, CaptureSessionRuntimeHost};
    use crate::application::providers::ocr::OcrCoordinator;
    use crate::application::services::{
        CaptureOutputService, CaptureSessionService, CaptureSessionSource, ImageCompositionService,
    };
    use crate::domain::capture::{
        CaptureOutputAction, CaptureSessionId, LogicalPoint, LogicalRect, MonitorLayout,
        MonitorSnapshot, PhysicalRect, ScreenRegion, WindowCandidate,
    };
    use crate::error::AppError;
    use crate::infrastructure::storage::ConfigFile;

    #[derive(Debug, Clone, PartialEq)]
    enum HostCall {
        BeginPresentation,
        HideCaptureWindow,
        OpenCaptureWindow {
            mode: String,
            session_id: String,
            bounds: LogicalRect,
        },
        RestoreSnapshotWindows(Vec<String>),
        EndPresentation,
    }

    struct RecordingRuntimeHost {
        calls: Arc<Mutex<Vec<HostCall>>>,
        hide_result: Result<(), String>,
        open_result: Result<(), String>,
        restore_result: Result<(), String>,
        end_result: Result<(), String>,
        destroy_result: Result<(), String>,
    }

    impl RecordingRuntimeHost {
        fn succeeds() -> Self {
            Self {
                calls: Arc::new(Mutex::new(Vec::new())),
                hide_result: Ok(()),
                open_result: Ok(()),
                restore_result: Ok(()),
                end_result: Ok(()),
                destroy_result: Ok(()),
            }
        }

        fn with_hide_error(message: &str) -> Self {
            Self {
                hide_result: Err(message.to_string()),
                ..Self::succeeds()
            }
        }

        fn with_open_error(message: &str) -> Self {
            Self {
                open_result: Err(message.to_string()),
                ..Self::succeeds()
            }
        }

        fn calls(&self) -> Vec<HostCall> {
            self.calls.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl CaptureSessionRuntimeHost for RecordingRuntimeHost {
        async fn begin_capture_presentation(&self) -> crate::Result<()> {
            self.calls.lock().unwrap().push(HostCall::BeginPresentation);
            Ok(())
        }

        async fn end_capture_presentation(&self) -> crate::Result<()> {
            self.calls.lock().unwrap().push(HostCall::EndPresentation);
            self.end_result.clone().map_err(AppError::from)
        }

        async fn hide_capture_window(&self) -> crate::Result<()> {
            self.calls.lock().unwrap().push(HostCall::HideCaptureWindow);
            self.hide_result.clone().map_err(AppError::from)
        }

        async fn destroy_inactive_capture_window(&self) -> crate::Result<()> {
            self.destroy_result.clone().map_err(AppError::from)
        }

        async fn open_capture_window_for_session(
            &self,
            mode: &str,
            session_id: &str,
            bounds: &LogicalRect,
        ) -> crate::Result<()> {
            self.calls
                .lock()
                .unwrap()
                .push(HostCall::OpenCaptureWindow {
                    mode: mode.to_string(),
                    session_id: session_id.to_string(),
                    bounds: bounds.clone(),
                });
            self.open_result.clone().map_err(AppError::from)
        }

        async fn restore_capture_snapshot_windows(
            &self,
            hidden_window_labels: Vec<String>,
        ) -> crate::Result<()> {
            self.calls
                .lock()
                .unwrap()
                .push(HostCall::RestoreSnapshotWindows(hidden_window_labels));
            self.restore_result.clone().map_err(AppError::from)
        }
    }

    struct MockCaptureSessionSource {
        snapshots: Vec<MonitorSnapshot>,
    }

    #[async_trait]
    impl CaptureSessionSource for MockCaptureSessionSource {
        async fn capture_monitor_snapshots(&self) -> Result<Vec<MonitorSnapshot>, AppError> {
            Ok(self.snapshots.clone())
        }

        async fn capture_monitor_layouts(&self) -> Result<Vec<MonitorLayout>, AppError> {
            Ok(self
                .snapshots
                .iter()
                .map(|snapshot| MonitorLayout {
                    id: snapshot.id.clone(),
                    logical_bounds: snapshot.logical_bounds.clone(),
                    physical_bounds: snapshot.physical_bounds.clone(),
                    scale_factor: snapshot.scale_factor,
                })
                .collect())
        }

        async fn capture_window_candidates(
            &self,
            _monitors: &[MonitorSnapshot],
        ) -> Result<Vec<WindowCandidate>, AppError> {
            Ok(Vec::new())
        }

        async fn capture_region(&self, _region: ScreenRegion) -> Result<Vec<u8>, AppError> {
            Ok(vec![1, 2, 3])
        }

        fn current_cursor_position(
            &self,
            _monitors: &[MonitorSnapshot],
        ) -> Result<Option<LogicalPoint>, AppError> {
            Ok(None)
        }
    }

    fn make_snapshot() -> MonitorSnapshot {
        MonitorSnapshot {
            id: "primary".to_string(),
            logical_bounds: LogicalRect {
                x: -20.0,
                y: 10.0,
                width: 80.0,
                height: 40.0,
            },
            physical_bounds: PhysicalRect {
                x: -20,
                y: 10,
                width: 80,
                height: 40,
            },
            scale_factor: 1.0,
            png_data: vec![1, 2, 3],
        }
    }

    fn make_renderable_snapshot() -> MonitorSnapshot {
        MonitorSnapshot {
            id: "primary".to_string(),
            logical_bounds: LogicalRect {
                x: 0.0,
                y: 0.0,
                width: 4.0,
                height: 4.0,
            },
            physical_bounds: PhysicalRect {
                x: 0,
                y: 0,
                width: 4,
                height: 4,
            },
            scale_factor: 1.0,
            png_data: make_solid_png(4, 4, [10, 20, 30, 255]),
        }
    }

    fn make_solid_png(width: u32, height: u32, rgba: [u8; 4]) -> Vec<u8> {
        let pixels = rgba.repeat((width * height) as usize);
        let mut png = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut png);
        encoder
            .write_image(&pixels, width, height, image::ExtendedColorType::Rgba8)
            .unwrap();
        png
    }

    fn make_runtime(
        host: Arc<dyn CaptureSessionRuntimeHost>,
        snapshots: Vec<MonitorSnapshot>,
    ) -> (CaptureSessionRuntime, Arc<CaptureSessionService>) {
        let sessions = Arc::new(CaptureSessionService::new(Arc::new(
            MockCaptureSessionSource { snapshots },
        )));
        let runtime = CaptureSessionRuntime::with_host(
            sessions.clone(),
            Arc::new(ImageCompositionService::new()),
            Arc::new(CaptureOutputService::new()),
            Arc::new(OcrCoordinator::new(Arc::new(ConfigFile::new_temp()))),
            host,
        );

        (runtime, sessions)
    }

    #[tokio::test]
    async fn create_session_from_visible_desktop_ignores_missing_capture_window_when_hiding() {
        let host = Arc::new(RecordingRuntimeHost::with_hide_error(
            "Capture window is not open",
        ));
        let (runtime, _) = make_runtime(host.clone(), vec![make_snapshot()]);

        let session = runtime.create_session_from_visible_desktop().await.unwrap();

        assert_eq!(session.monitors.len(), 1);
        assert_eq!(
            host.calls(),
            vec![HostCall::BeginPresentation, HostCall::HideCaptureWindow]
        );
    }

    #[tokio::test]
    async fn create_session_from_visible_desktop_ends_presentation_when_session_creation_fails() {
        let host = Arc::new(RecordingRuntimeHost::succeeds());
        let (runtime, _) = make_runtime(host.clone(), Vec::new());

        let err = runtime
            .create_session_from_visible_desktop()
            .await
            .unwrap_err()
            .to_string();

        assert_eq!(
            err,
            "System error: Cannot create capture session without monitor snapshots"
        );
        assert_eq!(
            host.calls(),
            vec![
                HostCall::BeginPresentation,
                HostCall::HideCaptureWindow,
                HostCall::EndPresentation,
            ]
        );
    }

    #[tokio::test]
    async fn open_capture_window_for_mode_rolls_back_session_when_window_open_fails() {
        let host = Arc::new(RecordingRuntimeHost::with_open_error("open failed"));
        let (runtime, sessions) = make_runtime(host.clone(), vec![make_snapshot()]);

        let err = runtime
            .open_capture_window_for_mode("screenshot")
            .await
            .unwrap_err()
            .to_string();

        let calls = host.calls();
        let session_id = match &calls[2] {
            HostCall::OpenCaptureWindow { session_id, .. } => session_id.clone(),
            other => panic!("expected open call, got {other:?}"),
        };

        assert_eq!(err, "open failed");
        assert_eq!(
            calls,
            vec![
                HostCall::BeginPresentation,
                HostCall::HideCaptureWindow,
                HostCall::OpenCaptureWindow {
                    mode: "screenshot".to_string(),
                    session_id: session_id.clone(),
                    bounds: LogicalRect {
                        x: -20.0,
                        y: 10.0,
                        width: 80.0,
                        height: 40.0,
                    },
                },
                HostCall::RestoreSnapshotWindows(Vec::new()),
                HostCall::EndPresentation,
            ]
        );
        assert!(sessions.get_session(&CaptureSessionId(session_id)).is_err());
    }

    #[tokio::test]
    async fn render_png_base64_rejects_selection_before_snapshots_are_hydrated() {
        let host = Arc::new(RecordingRuntimeHost::succeeds());
        let (runtime, sessions) = make_runtime(host, vec![make_snapshot()]);
        let session = sessions.create_layout_session().await.unwrap();

        let err = runtime
            .render_png_base64(
                &session.id,
                &LogicalRect {
                    x: -10.0,
                    y: 20.0,
                    width: 20.0,
                    height: 10.0,
                },
                &[],
                false,
            )
            .unwrap_err()
            .to_string();

        assert_eq!(
            err,
            "Capture session snapshots are not ready for the selected area"
        );
    }

    #[tokio::test]
    async fn output_selection_pin_returns_rendered_png_through_runtime() {
        let host = Arc::new(RecordingRuntimeHost::succeeds());
        let (runtime, sessions) = make_runtime(host, vec![make_renderable_snapshot()]);
        let session = sessions.create_session().await.unwrap();

        let output = runtime
            .output_selection(
                &session.id,
                &LogicalRect {
                    x: 1.0,
                    y: 1.0,
                    width: 2.0,
                    height: 2.0,
                },
                &[],
                false,
                CaptureOutputAction::Pin,
            )
            .await
            .unwrap();

        let CaptureSessionOutput::Pin(png_data) = output else {
            panic!("expected pin output");
        };
        let decoded = image::load_from_memory(&png_data).unwrap().to_rgba8();

        assert_eq!((decoded.width(), decoded.height()), (2, 2));
        assert!(decoded.pixels().all(|pixel| pixel.0 == [10, 20, 30, 255]));
    }
}

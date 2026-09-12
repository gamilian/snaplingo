use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use super::ocr_completion::{CaptureOcrStatus, CaptureOcrTarget, PendingCaptureOcr};
use super::render::{
    output_capture_selection, recognize_capture_selection_text, render_capture_png_base64,
};
use super::runtime_host::{
    CapturePinOutput, CaptureSessionRuntimeHost, UnconfiguredCapturePinOutput,
    UnconfiguredCaptureSessionRuntimeHost,
};
use super::{CaptureImageComposer, CaptureOutput, CaptureSessionOutput, CaptureSessions};
use crate::application::providers::ocr::OcrCoordinator;
use crate::domain::capture::{
    AnnotationCommand, CaptureOutputAction, CaptureSessionId, CaptureSessionView, LogicalRect,
};
use crate::domain::ocr::OcrResult;
use crate::Result;

/// Coordinates Capture Session operations that span several application modules.
pub struct CaptureSessionRuntime {
    sessions: Arc<CaptureSessions>,
    image_composition: Arc<CaptureImageComposer>,
    output: Arc<CaptureOutput>,
    ocr: Arc<OcrCoordinator>,
    host: Arc<dyn CaptureSessionRuntimeHost>,
    pin_output: Arc<dyn CapturePinOutput>,
    startup: tokio::sync::Mutex<()>,
    presentations: tokio::sync::Mutex<HashMap<CaptureSessionId, u64>>,
    ocr_generation: tokio::sync::watch::Sender<u64>,
    pending_ocr: tokio::sync::Mutex<Option<PendingCaptureOcr>>,
}

impl CaptureSessionRuntime {
    pub fn new(
        sessions: Arc<CaptureSessions>,
        image_composition: Arc<CaptureImageComposer>,
        output: Arc<CaptureOutput>,
        ocr: Arc<OcrCoordinator>,
    ) -> Self {
        Self::with_host(
            sessions,
            image_composition,
            output,
            ocr,
            Arc::new(UnconfiguredCaptureSessionRuntimeHost),
            Arc::new(UnconfiguredCapturePinOutput),
        )
    }

    pub(crate) fn with_host(
        sessions: Arc<CaptureSessions>,
        image_composition: Arc<CaptureImageComposer>,
        output: Arc<CaptureOutput>,
        ocr: Arc<OcrCoordinator>,
        host: Arc<dyn CaptureSessionRuntimeHost>,
        pin_output: Arc<dyn CapturePinOutput>,
    ) -> Self {
        Self {
            sessions,
            image_composition,
            output,
            ocr,
            host,
            pin_output,
            startup: tokio::sync::Mutex::new(()),
            presentations: tokio::sync::Mutex::new(HashMap::new()),
            ocr_generation: tokio::sync::watch::channel(0).0,
            pending_ocr: tokio::sync::Mutex::new(None),
        }
    }

    pub async fn create_session_from_visible_desktop(&self) -> Result<CaptureSessionView> {
        let _startup = self.startup.try_lock().map_err(|_| {
            crate::AppError::System("Capture session startup is already in progress".into())
        })?;
        self.create_visible_desktop_session().await
    }

    async fn create_visible_desktop_session(&self) -> Result<CaptureSessionView> {
        self.cancel_background_ocr();
        *self.pending_ocr.lock().await = None;
        let generation = *self.ocr_generation.borrow();
        self.host.set_capture_ocr_status(None);
        let total_start = Instant::now();
        let begin_start = Instant::now();
        self.host.begin_capture_presentation().await?;
        let begin_ms = elapsed_ms(begin_start);

        let hide_overlay_start = Instant::now();
        if let Err(err) = self.host.hide_capture_window().await {
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
            Ok(session) => {
                self.presentations
                    .lock()
                    .await
                    .insert(session.id.clone(), generation);
                Ok(session)
            }
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
        let Ok(_startup) = self.startup.try_lock() else {
            log::info!("Ignoring capture request while a capture window is already opening");
            return Ok(());
        };
        let total_start = Instant::now();
        let session_start = Instant::now();
        let session = self.create_visible_desktop_session().await?;
        let session_ms = elapsed_ms(session_start);
        let monitor_count = session.monitors.len();
        let candidate_count = session.candidates.len();
        let view_base64_bytes = capture_session_view_base64_bytes(&session);

        let open_start = Instant::now();
        let open_result = match self.sessions.window_geometry(&session.id) {
            Ok(geometry) => {
                self.host
                    .open_capture_window_for_session(mode, &session.id.0, &geometry)
                    .await
            }
            Err(error) => Err(error),
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
            let presentation_result = self.end_session_presentation(&session.id).await;

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

    pub async fn prepare_capture_window_for_reveal(
        &self,
        session_id: Option<&CaptureSessionId>,
    ) -> Result<()> {
        let geometry = session_id
            .map(|id| self.sessions.window_geometry(id))
            .transpose()?;
        self.host
            .prepare_capture_window_for_reveal(session_id, geometry.as_ref())
            .await
    }

    pub async fn reveal_capture_window(&self, session_id: Option<&CaptureSessionId>) -> Result<()> {
        let geometry = session_id
            .map(|id| self.sessions.window_geometry(id))
            .transpose()?;
        self.host.reveal_capture_window(geometry.as_ref()).await
    }

    pub async fn hide_capture_window(&self) -> Result<()> {
        self.host.hide_capture_window().await
    }

    pub async fn control_candidate_at(
        &self,
        id: &CaptureSessionId,
        point: &crate::domain::capture::LogicalPoint,
    ) -> Result<Option<crate::domain::capture::CaptureCandidateView>> {
        self.host
            .set_capture_window_cursor_passthrough(true)
            .await?;
        let result = self.sessions.control_candidate_at(id, point).await;
        self.host
            .set_capture_window_cursor_passthrough(false)
            .await?;
        result
    }

    pub async fn cancel_capture_session(&self, session_id: &CaptureSessionId) -> Result<()> {
        let _startup = self.startup.lock().await;
        self.cancel_capture_session_inner(session_id).await
    }

    async fn cancel_capture_session_inner(&self, session_id: &CaptureSessionId) -> Result<()> {
        let mut pending = self.pending_ocr.lock().await;
        if pending
            .as_ref()
            .is_some_and(|job| &job.session_id == session_id)
        {
            *pending = None;
        }
        drop(pending);
        let restore_result = self
            .restore_capture_snapshot_windows_for_session_id(session_id)
            .await;
        let cancel_result = self.sessions.cancel_session(session_id);
        let presentation_result = self.end_session_presentation(session_id).await;
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
        let presentation_result = self.end_session_presentation(session_id).await;
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

    pub fn render_png(
        &self,
        session_id: &CaptureSessionId,
        rect: &LogicalRect,
        annotations: &[AnnotationCommand],
        include_cursor: bool,
    ) -> Result<Vec<u8>> {
        self.ensure_selection_snapshots_ready(session_id, rect)?;

        super::render::render_capture_png(
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
        language: Option<String>,
    ) -> Result<OcrResult> {
        self.ensure_selection_snapshots_ready(session_id, rect)?;

        recognize_capture_selection_text(
            &self.sessions,
            &self.image_composition,
            &self.ocr,
            session_id,
            rect,
            language,
        )
        .await
    }

    /// Own the selected pixels before the WebView finishes its selection workflow.
    pub(crate) async fn prepare_capture_ocr(
        &self,
        session_id: &CaptureSessionId,
        rect: &LogicalRect,
        annotations: &[AnnotationCommand],
        target: CaptureOcrTarget,
        settings: crate::domain::OcrSettings,
    ) -> Result<()> {
        let _startup = self.startup.lock().await;
        let generation = *self.ocr_generation.borrow();
        if self.presentations.lock().await.get(session_id) != Some(&generation) {
            return Err("Capture OCR session is no longer current".into());
        }
        let clipboard_revision = self.host.clipboard_revision();
        self.ensure_selection_snapshots_ready(session_id, rect)?;
        let language = (settings.recognition_language != "auto")
            .then(|| settings.recognition_language.clone());
        let request = super::render::capture_ocr_request(
            &self.sessions,
            &self.image_composition,
            session_id,
            rect,
            language,
        )?;
        let preview = if target == CaptureOcrTarget::OcrWindow {
            Some(self.render_png_base64(session_id, rect, annotations, false)?)
        } else {
            None
        };
        *self.pending_ocr.lock().await = Some(PendingCaptureOcr {
            session_id: session_id.clone(),
            generation,
            request,
            preview,
            target,
            settings,
            clipboard_revision,
        });
        Ok(())
    }

    /// Commit the handoff. Destruction of the old WebView cannot cancel this owned job.
    pub(crate) async fn complete_capture_ocr(
        self: &Arc<Self>,
        session_id: &CaptureSessionId,
    ) -> Result<()> {
        let _startup = self.startup.lock().await;
        let mut pending = self.pending_ocr.lock().await;
        if !pending
            .as_ref()
            .is_some_and(|job| &job.session_id == session_id)
        {
            return Err("No prepared OCR selection for this session".into());
        }
        let job = pending.take().unwrap();
        drop(pending);
        if job.generation != *self.ocr_generation.borrow() {
            return Err("Capture OCR session is no longer current".into());
        }
        let result_id = self.host.reserve_capture_ocr_result(job.target)?;
        let hide_result = self.host.hide_capture_window().await;
        let cancel_result = self.cancel_capture_session_inner(session_id).await;
        if let Err(error) = hide_result.and(cancel_result) {
            self.host
                .report_capture_ocr_error(job.target, result_id, &error.to_string())
                .await;
            return Err(error);
        }
        if job.shows_status() {
            self.host
                .set_capture_ocr_status(Some(CaptureOcrStatus::Recognizing));
        }
        let runtime = Arc::clone(self);
        let receiver = self.ocr_generation.subscribe();
        tokio::spawn(async move {
            runtime.run_capture_ocr_job(job, result_id, receiver).await;
        });
        Ok(())
    }

    pub(crate) fn cancel_background_ocr(&self) {
        self.ocr_generation
            .send_modify(|generation| *generation = generation.wrapping_add(1));
    }

    async fn run_capture_ocr_job(
        &self,
        mut job: PendingCaptureOcr,
        result_id: u64,
        mut generation: tokio::sync::watch::Receiver<u64>,
    ) {
        if *generation.borrow() != job.generation {
            return;
        }
        let result = tokio::select! {
            biased;
            _ = generation.changed() => return,
            result = self.ocr.recognize(&job.request) => result,
        };
        // Serialize delivery with a new capture's startup, including native window calls.
        let startup = self.startup.lock().await;
        if *self.ocr_generation.borrow() != job.generation {
            return;
        }
        if job.target == CaptureOcrTarget::Clipboard
            && self.host.clipboard_revision() != job.clipboard_revision
        {
            // A newer explicit copy wins over this earlier background OCR request.
            self.host.set_capture_ocr_status(None);
            return;
        }
        let result = match result {
            Ok(mut result) if !result.text.trim().is_empty() => {
                if job.target == CaptureOcrTarget::Clipboard {
                    result.text =
                        super::ocr_completion::clipboard_text(&result.text, &job.settings);
                }
                self.host
                    .publish_capture_ocr(job.target, result_id, result, job.preview.take())
                    .await
            }
            Ok(_) => Err("No text was recognized in the selected area".into()),
            Err(error) => Err(error),
        };
        let status = match result {
            Ok(()) => {
                (job.target == CaptureOcrTarget::Clipboard).then_some(CaptureOcrStatus::Copied)
            }
            Err(error) => {
                self.host
                    .report_capture_ocr_error(job.target, result_id, &error.to_string())
                    .await;
                Some(CaptureOcrStatus::Failed)
            }
        };
        if job.shows_status() {
            self.host.set_capture_ocr_status(status);
        }
        drop(startup);
        if status.is_some() && job.shows_status() {
            tokio::select! {
                _ = generation.changed() => return,
                _ = tokio::time::sleep(std::time::Duration::from_millis(1500)) => {}
            }
            let _startup = self.startup.lock().await;
            if *self.ocr_generation.borrow() == job.generation {
                self.host.set_capture_ocr_status(None);
            }
        }
    }

    pub async fn output_selection(
        &self,
        session_id: &CaptureSessionId,
        rect: &LogicalRect,
        annotations: &[AnnotationCommand],
        include_cursor: bool,
        action: CaptureOutputAction,
    ) -> Result<()> {
        self.ensure_selection_snapshots_ready(session_id, rect)?;

        let output = output_capture_selection(
            &self.sessions,
            &self.image_composition,
            &self.output,
            session_id,
            rect,
            annotations,
            include_cursor,
            action,
        )
        .await?;
        match output {
            CaptureSessionOutput::Completed => Ok(()),
            CaptureSessionOutput::Pin(png_data) => self.pin_output.pin_png(png_data).await,
        }
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

    async fn end_session_presentation(&self, session_id: &CaptureSessionId) -> Result<()> {
        // A late IPC response or a destroyed old window may cancel the same ID again.
        // Only its owner can decrement the native presentation depth.
        if self.presentations.lock().await.remove(session_id).is_some() {
            self.host.end_capture_presentation().await?;
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

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use image::ImageEncoder;

    use super::super::{
        CaptureCoordinatePolicy, CaptureImageComposer, CaptureOutput, CaptureSessionSource,
        CaptureSessions, CaptureWindowGeometry,
    };
    use super::{CapturePinOutput, CaptureSessionRuntime, CaptureSessionRuntimeHost};
    use crate::application::providers::ocr::OcrCoordinator;
    use crate::domain::capture::{
        CaptureOutputAction, CaptureSessionId, LogicalPoint, LogicalRect, MonitorLayout,
        MonitorSnapshot, PhysicalRect, ScreenRegion, WindowCandidate,
    };
    use crate::error::AppError;
    use crate::infrastructure::storage::SqliteConfigStore;

    #[derive(Debug, Clone, PartialEq)]
    enum HostCall {
        BeginPresentation,
        CaptureSnapshots,
        PrepareCaptureWindowForReveal(Option<CaptureWindowGeometry>),
        RevealCaptureWindow(Option<CaptureWindowGeometry>),
        HideCaptureWindow,
        OpenCaptureWindow {
            mode: String,
            session_id: String,
            geometry: CaptureWindowGeometry,
        },
        RestoreSnapshotWindows(Vec<String>),
        EndPresentation,
    }

    struct RecordingRuntimeHost {
        calls: Arc<Mutex<Vec<HostCall>>>,
        hide_result: Result<(), String>,
        open_result: Mutex<Result<(), String>>,
        open_barrier: Option<Arc<StartupBarrier>>,
        restore_result: Result<(), String>,
        end_result: Result<(), String>,
        destroy_result: Result<(), String>,
        destroy_calls: std::sync::atomic::AtomicUsize,
        ocr_results: Mutex<
            Vec<(
                super::CaptureOcrTarget,
                crate::domain::ocr::OcrResult,
                Option<String>,
            )>,
        >,
        ocr_errors: Mutex<Vec<String>>,
        ocr_statuses: Mutex<Vec<Option<super::CaptureOcrStatus>>>,
        ocr_completed: tokio::sync::Notify,
        clipboard_revision: std::sync::atomic::AtomicU64,
    }

    impl RecordingRuntimeHost {
        fn succeeds() -> Self {
            Self {
                calls: Arc::new(Mutex::new(Vec::new())),
                hide_result: Ok(()),
                open_result: Mutex::new(Ok(())),
                open_barrier: None,
                restore_result: Ok(()),
                end_result: Ok(()),
                destroy_result: Ok(()),
                destroy_calls: std::sync::atomic::AtomicUsize::new(0),
                ocr_results: Mutex::new(Vec::new()),
                ocr_errors: Mutex::new(Vec::new()),
                ocr_statuses: Mutex::new(Vec::new()),
                ocr_completed: tokio::sync::Notify::new(),
                clipboard_revision: std::sync::atomic::AtomicU64::new(0),
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
                open_result: Mutex::new(Err(message.to_string())),
                ..Self::succeeds()
            }
        }

        fn calls(&self) -> Vec<HostCall> {
            self.calls.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl CaptureSessionRuntimeHost for RecordingRuntimeHost {
        fn clipboard_revision(&self) -> u64 {
            self.clipboard_revision
                .load(std::sync::atomic::Ordering::Relaxed)
        }
        fn set_capture_ocr_status(&self, status: Option<super::CaptureOcrStatus>) {
            self.ocr_statuses.lock().unwrap().push(status);
        }

        async fn publish_capture_ocr(
            &self,
            target: super::CaptureOcrTarget,
            _: u64,
            result: crate::domain::ocr::OcrResult,
            preview: Option<String>,
        ) -> crate::Result<()> {
            self.ocr_results
                .lock()
                .unwrap()
                .push((target, result, preview));
            self.ocr_completed.notify_one();
            Ok(())
        }

        async fn report_capture_ocr_error(&self, _: super::CaptureOcrTarget, _: u64, error: &str) {
            self.ocr_errors.lock().unwrap().push(error.into());
            self.ocr_completed.notify_one();
        }

        async fn begin_capture_presentation(&self) -> crate::Result<()> {
            self.calls.lock().unwrap().push(HostCall::BeginPresentation);
            Ok(())
        }

        async fn end_capture_presentation(&self) -> crate::Result<()> {
            self.calls.lock().unwrap().push(HostCall::EndPresentation);
            self.end_result.clone().map_err(AppError::from)
        }

        async fn prepare_capture_window_for_reveal(
            &self,
            _session_id: Option<&CaptureSessionId>,
            geometry: Option<&CaptureWindowGeometry>,
        ) -> crate::Result<()> {
            self.calls
                .lock()
                .unwrap()
                .push(HostCall::PrepareCaptureWindowForReveal(geometry.cloned()));
            Ok(())
        }

        async fn reveal_capture_window(
            &self,
            geometry: Option<&CaptureWindowGeometry>,
        ) -> crate::Result<()> {
            self.calls
                .lock()
                .unwrap()
                .push(HostCall::RevealCaptureWindow(geometry.cloned()));
            Ok(())
        }

        async fn hide_capture_window(&self) -> crate::Result<()> {
            self.calls.lock().unwrap().push(HostCall::HideCaptureWindow);
            self.hide_result.clone().map_err(AppError::from)
        }

        async fn destroy_inactive_capture_window(&self) -> crate::Result<()> {
            self.destroy_calls
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            self.destroy_result.clone().map_err(AppError::from)
        }

        async fn open_capture_window_for_session(
            &self,
            mode: &str,
            session_id: &str,
            geometry: &CaptureWindowGeometry,
        ) -> crate::Result<()> {
            self.calls
                .lock()
                .unwrap()
                .push(HostCall::OpenCaptureWindow {
                    mode: mode.to_string(),
                    session_id: session_id.to_string(),
                    geometry: geometry.clone(),
                });
            if let Some(barrier) = &self.open_barrier {
                barrier.entered.notify_one();
                barrier.release.notified().await;
            }
            self.open_result
                .lock()
                .unwrap()
                .clone()
                .map_err(AppError::from)
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

    #[derive(Default)]
    struct StartupBarrier {
        entered: tokio::sync::Notify,
        release: tokio::sync::Notify,
    }

    #[derive(Default)]
    struct RecordingPinOutput {
        pngs: Mutex<Vec<Vec<u8>>>,
        error: Mutex<Option<String>>,
    }

    #[async_trait]
    impl CapturePinOutput for RecordingPinOutput {
        async fn pin_png(&self, png_data: Vec<u8>) -> crate::Result<()> {
            self.pngs.lock().unwrap().push(png_data);
            match self.error.lock().unwrap().clone() {
                Some(error) => Err(AppError::from(error)),
                None => Ok(()),
            }
        }
    }

    struct MockCaptureSessionSource {
        snapshots: Mutex<Vec<MonitorSnapshot>>,
        coordinate_policy: CaptureCoordinatePolicy,
    }

    struct OrderedCaptureSessionSource {
        capture_barrier: Option<Arc<StartupBarrier>>,
        calls: Arc<Mutex<Vec<HostCall>>>,
        snapshots: Vec<MonitorSnapshot>,
    }

    #[async_trait]
    impl CaptureSessionSource for MockCaptureSessionSource {
        fn coordinate_policy(&self) -> CaptureCoordinatePolicy {
            self.coordinate_policy
        }

        async fn capture_monitor_snapshots(&self) -> Result<Vec<MonitorSnapshot>, AppError> {
            Ok(self.snapshots.lock().unwrap().clone())
        }

        async fn capture_monitor_layouts(&self) -> Result<Vec<MonitorLayout>, AppError> {
            Ok(self
                .snapshots
                .lock()
                .unwrap()
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

    #[async_trait]
    impl CaptureSessionSource for OrderedCaptureSessionSource {
        async fn capture_monitor_snapshots(&self) -> Result<Vec<MonitorSnapshot>, AppError> {
            self.calls.lock().unwrap().push(HostCall::CaptureSnapshots);
            if let Some(barrier) = &self.capture_barrier {
                barrier.entered.notify_one();
                barrier.release.notified().await;
            }
            Ok(self.snapshots.clone())
        }

        async fn capture_monitor_layouts(&self) -> Result<Vec<MonitorLayout>, AppError> {
            Ok(Vec::new())
        }

        async fn capture_region(&self, _region: ScreenRegion) -> Result<Vec<u8>, AppError> {
            unreachable!("capture sessions must use frozen monitor snapshots")
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
    ) -> (
        CaptureSessionRuntime,
        Arc<CaptureSessions>,
        Arc<RecordingPinOutput>,
    ) {
        let sessions = Arc::new(CaptureSessions::new(Arc::new(MockCaptureSessionSource {
            snapshots: Mutex::new(snapshots),
            coordinate_policy: CaptureCoordinatePolicy::NativeLogical,
        })));
        let pin_output = Arc::new(RecordingPinOutput::default());
        let runtime = CaptureSessionRuntime::with_host(
            sessions.clone(),
            Arc::new(CaptureImageComposer::new()),
            Arc::new(CaptureOutput::new()),
            Arc::new(OcrCoordinator::new(Arc::new(SqliteConfigStore::new_temp()))),
            host,
            pin_output.clone(),
        );

        (runtime, sessions, pin_output)
    }

    #[tokio::test]
    async fn capture_window_visibility_uses_runtime_host_seam() {
        let host = Arc::new(RecordingRuntimeHost::succeeds());
        let (runtime, sessions, _) = make_runtime(host.clone(), vec![make_snapshot()]);
        let session = sessions.create_session().await.unwrap();
        let geometry = sessions.window_geometry(&session.id).unwrap();

        runtime
            .prepare_capture_window_for_reveal(Some(&session.id))
            .await
            .unwrap();
        runtime
            .reveal_capture_window(Some(&session.id))
            .await
            .unwrap();
        runtime.hide_capture_window().await.unwrap();

        assert_eq!(
            host.calls(),
            vec![
                HostCall::PrepareCaptureWindowForReveal(Some(geometry.clone())),
                HostCall::RevealCaptureWindow(Some(geometry)),
                HostCall::HideCaptureWindow,
            ]
        );
    }

    #[tokio::test]
    async fn session_load_errors_can_be_revealed_without_session_geometry() {
        let host = Arc::new(RecordingRuntimeHost::succeeds());
        let (runtime, _, _) = make_runtime(host.clone(), Vec::new());

        runtime
            .prepare_capture_window_for_reveal(None)
            .await
            .unwrap();
        runtime.reveal_capture_window(None).await.unwrap();

        assert_eq!(
            host.calls(),
            vec![
                HostCall::PrepareCaptureWindowForReveal(None),
                HostCall::RevealCaptureWindow(None),
            ]
        );
    }

    #[tokio::test]
    async fn window_reveal_uses_the_requested_frozen_session_after_refresh() {
        let mut initial = make_snapshot();
        initial.scale_factor = 2.0;
        let source = Arc::new(MockCaptureSessionSource {
            snapshots: Mutex::new(vec![initial]),
            coordinate_policy: CaptureCoordinatePolicy::PrimaryMonitorScale,
        });
        let sessions = Arc::new(CaptureSessions::new(source.clone()));
        let host = Arc::new(RecordingRuntimeHost::succeeds());
        let runtime = CaptureSessionRuntime::with_host(
            sessions.clone(),
            Arc::new(CaptureImageComposer::new()),
            Arc::new(CaptureOutput::new()),
            Arc::new(OcrCoordinator::new(Arc::new(SqliteConfigStore::new_temp()))),
            host.clone(),
            Arc::new(RecordingPinOutput::default()),
        );

        runtime
            .open_capture_window_for_mode("screenshot")
            .await
            .unwrap();
        let original_id = host
            .calls()
            .into_iter()
            .find_map(|call| match call {
                HostCall::OpenCaptureWindow { session_id, .. } => {
                    Some(CaptureSessionId(session_id))
                }
                _ => None,
            })
            .unwrap();
        let original_geometry = sessions.window_geometry(&original_id).unwrap();
        {
            let mut snapshots = source.snapshots.lock().unwrap();
            snapshots[0].physical_bounds.x = -300;
            snapshots[0].physical_bounds.width = 900;
            snapshots[0].scale_factor = 1.5;
        }
        let refreshed = runtime.create_session_from_visible_desktop().await.unwrap();
        let refreshed_geometry = sessions.window_geometry(&refreshed.id).unwrap();
        assert_ne!(original_geometry.bounds, refreshed_geometry.bounds);
        assert_eq!(original_geometry.desktop_scale, Some(2.0));
        assert_eq!(refreshed_geometry.desktop_scale, Some(1.5));
        source.snapshots.lock().unwrap()[0].scale_factor = 4.0;
        host.calls.lock().unwrap().clear();

        runtime
            .prepare_capture_window_for_reveal(Some(&refreshed.id))
            .await
            .unwrap();
        runtime
            .reveal_capture_window(Some(&refreshed.id))
            .await
            .unwrap();
        runtime
            .prepare_capture_window_for_reveal(Some(&original_id))
            .await
            .unwrap();
        runtime
            .reveal_capture_window(Some(&original_id))
            .await
            .unwrap();

        assert_eq!(
            host.calls(),
            vec![
                HostCall::PrepareCaptureWindowForReveal(Some(refreshed_geometry.clone())),
                HostCall::RevealCaptureWindow(Some(refreshed_geometry)),
                HostCall::PrepareCaptureWindowForReveal(Some(original_geometry.clone())),
                HostCall::RevealCaptureWindow(Some(original_geometry)),
            ]
        );
    }

    #[tokio::test]
    async fn create_session_from_visible_desktop_accepts_an_idempotent_hide() {
        let host = Arc::new(RecordingRuntimeHost::succeeds());
        let (runtime, _, _) = make_runtime(host.clone(), vec![make_snapshot()]);

        let session = runtime.create_session_from_visible_desktop().await.unwrap();

        assert_eq!(session.monitors.len(), 1);
        assert_eq!(
            host.calls(),
            vec![HostCall::BeginPresentation, HostCall::HideCaptureWindow]
        );
    }

    #[tokio::test]
    async fn cancelling_an_old_session_twice_does_not_end_a_new_presentation() {
        let host = Arc::new(RecordingRuntimeHost::succeeds());
        let (runtime, sessions, _) = make_runtime(host.clone(), vec![make_snapshot()]);
        let old = runtime.create_session_from_visible_desktop().await.unwrap();
        let current = runtime.create_session_from_visible_desktop().await.unwrap();

        runtime.cancel_capture_session(&old.id).await.unwrap();
        runtime.cancel_capture_session(&old.id).await.unwrap();
        runtime
            .cancel_capture_session(&CaptureSessionId("unknown".into()))
            .await
            .unwrap();

        assert!(!sessions.has_session(&old.id));
        assert!(sessions.has_session(&current.id));
        assert_eq!(
            host.calls()
                .iter()
                .filter(|call| **call == HostCall::EndPresentation)
                .count(),
            1
        );

        runtime.cancel_capture_session(&current.id).await.unwrap();
        assert_eq!(
            host.calls()
                .iter()
                .filter(|call| **call == HostCall::EndPresentation)
                .count(),
            2
        );
    }

    #[tokio::test]
    async fn restoring_then_cancelling_a_session_releases_its_presentation_once() {
        let host = Arc::new(RecordingRuntimeHost::succeeds());
        let (runtime, sessions, _) = make_runtime(host.clone(), vec![make_snapshot()]);
        let session = runtime.create_session_from_visible_desktop().await.unwrap();

        runtime
            .restore_capture_snapshot_windows_for_session(&session.id)
            .await
            .unwrap();
        assert!(sessions.has_session(&session.id));
        runtime.cancel_capture_session(&session.id).await.unwrap();

        assert!(!sessions.has_session(&session.id));
        assert_eq!(
            host.calls()
                .iter()
                .filter(|call| **call == HostCall::EndPresentation)
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn create_session_hides_the_overlay_before_freezing_monitor_pixels() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let host = Arc::new(RecordingRuntimeHost {
            calls: calls.clone(),
            ..RecordingRuntimeHost::succeeds()
        });
        let sessions = Arc::new(CaptureSessions::new(Arc::new(
            OrderedCaptureSessionSource {
                capture_barrier: None,
                calls: calls.clone(),
                snapshots: vec![make_snapshot()],
            },
        )));
        let runtime = CaptureSessionRuntime::with_host(
            sessions,
            Arc::new(CaptureImageComposer::new()),
            Arc::new(CaptureOutput::new()),
            Arc::new(OcrCoordinator::new(Arc::new(SqliteConfigStore::new_temp()))),
            host,
            Arc::new(RecordingPinOutput::default()),
        );

        runtime.create_session_from_visible_desktop().await.unwrap();

        assert_eq!(
            calls.lock().unwrap().as_slice(),
            &[
                HostCall::BeginPresentation,
                HostCall::HideCaptureWindow,
                HostCall::CaptureSnapshots,
            ]
        );
    }

    #[tokio::test]
    async fn create_session_from_visible_desktop_ends_presentation_when_session_creation_fails() {
        let host = Arc::new(RecordingRuntimeHost::succeeds());
        let (runtime, _, _) = make_runtime(host.clone(), Vec::new());

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
        let (runtime, sessions, _) = make_runtime(host.clone(), vec![make_snapshot()]);

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
                    geometry: CaptureWindowGeometry {
                        bounds: LogicalRect {
                            x: -20.0,
                            y: 10.0,
                            width: 80.0,
                            height: 40.0
                        },
                        desktop_scale: None,
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
        let (runtime, sessions, _) = make_runtime(host, vec![make_snapshot()]);
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
    async fn output_selection_delivers_frozen_png_to_pin_output_once() {
        let host = Arc::new(RecordingRuntimeHost::succeeds());
        let (runtime, sessions, pin_output) = make_runtime(host, vec![make_renderable_snapshot()]);
        let session = sessions.create_session().await.unwrap();

        runtime
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

        let pngs = pin_output.pngs.lock().unwrap();
        assert_eq!(pngs.len(), 1);
        let expected = runtime
            .render_png(
                &session.id,
                &LogicalRect {
                    x: 1.0,
                    y: 1.0,
                    width: 2.0,
                    height: 2.0,
                },
                &[],
                false,
            )
            .unwrap();
        assert_eq!(pngs[0], expected);
        let decoded = image::load_from_memory(&pngs[0]).unwrap().to_rgba8();

        assert_eq!((decoded.width(), decoded.height()), (2, 2));
        assert!(decoded.pixels().all(|pixel| pixel.0 == [10, 20, 30, 255]));
    }
    #[tokio::test]
    async fn every_open_entry_ignores_overlap_until_native_open_finishes() {
        let barrier = Arc::new(StartupBarrier::default());
        let host = Arc::new(RecordingRuntimeHost {
            open_barrier: Some(barrier.clone()),
            ..RecordingRuntimeHost::succeeds()
        });
        let (runtime, _, _) = make_runtime(host.clone(), vec![make_snapshot()]);
        let runtime = Arc::new(runtime);
        let pending = {
            let runtime = runtime.clone();
            tokio::spawn(async move { runtime.open_capture_window_for_mode("screenshot").await })
        };
        tokio::time::timeout(
            std::time::Duration::from_secs(2),
            barrier.entered.notified(),
        )
        .await
        .unwrap();
        runtime
            .open_capture_window_for_mode("screenshot-ocr")
            .await
            .unwrap();
        let error = runtime
            .create_session_from_visible_desktop()
            .await
            .unwrap_err();
        assert!(error.to_string().contains("startup is already in progress"));
        assert_eq!(
            host.calls()
                .iter()
                .filter(|call| matches!(call, HostCall::BeginPresentation))
                .count(),
            1
        );
        barrier.release.notify_one();
        pending.await.unwrap().unwrap();
        barrier.release.notify_one();
        runtime
            .open_capture_window_for_mode("screenshot-ocr")
            .await
            .unwrap();
        assert_eq!(
            host.calls()
                .iter()
                .filter(|call| matches!(call, HostCall::BeginPresentation))
                .count(),
            2
        );
    }

    #[tokio::test]
    async fn direct_session_creation_and_window_open_share_startup_exclusion() {
        let barrier = Arc::new(StartupBarrier::default());
        let calls = Arc::new(Mutex::new(Vec::new()));
        let host = Arc::new(RecordingRuntimeHost {
            calls: calls.clone(),
            ..RecordingRuntimeHost::succeeds()
        });
        let sessions = Arc::new(CaptureSessions::new(Arc::new(
            OrderedCaptureSessionSource {
                capture_barrier: Some(barrier.clone()),
                calls: calls.clone(),
                snapshots: vec![make_snapshot()],
            },
        )));
        let runtime = Arc::new(CaptureSessionRuntime::with_host(
            sessions,
            Arc::new(CaptureImageComposer::new()),
            Arc::new(CaptureOutput::new()),
            Arc::new(OcrCoordinator::new(Arc::new(SqliteConfigStore::new_temp()))),
            host,
            Arc::new(RecordingPinOutput::default()),
        ));
        let pending = {
            let runtime = runtime.clone();
            tokio::spawn(async move { runtime.create_session_from_visible_desktop().await })
        };
        tokio::time::timeout(
            std::time::Duration::from_secs(2),
            barrier.entered.notified(),
        )
        .await
        .unwrap();
        assert!(runtime.create_session_from_visible_desktop().await.is_err());
        runtime
            .open_capture_window_for_mode("screenshot")
            .await
            .unwrap();
        assert_eq!(
            calls.lock().unwrap().as_slice(),
            &[
                HostCall::BeginPresentation,
                HostCall::HideCaptureWindow,
                HostCall::CaptureSnapshots,
            ]
        );
        barrier.release.notify_one();
        pending.await.unwrap().unwrap();
        barrier.release.notify_one();
        runtime.create_session_from_visible_desktop().await.unwrap();
    }

    #[tokio::test]
    async fn failed_startup_releases_exclusion_and_allows_retry() {
        let host = Arc::new(RecordingRuntimeHost::with_open_error("open failed"));
        let (runtime, sessions, _) = make_runtime(host.clone(), vec![make_snapshot()]);
        assert!(runtime
            .open_capture_window_for_mode("screenshot")
            .await
            .is_err());
        let first_id = host
            .calls()
            .iter()
            .find_map(|call| match call {
                HostCall::OpenCaptureWindow { session_id, .. } => Some(session_id.clone()),
                _ => None,
            })
            .unwrap();
        assert!(!sessions.has_session(&CaptureSessionId(first_id)));
        *host.open_result.lock().unwrap() = Ok(());
        runtime
            .open_capture_window_for_mode("screenshot")
            .await
            .unwrap();
        assert_eq!(
            host.calls()
                .iter()
                .filter(|call| matches!(call, HostCall::BeginPresentation))
                .count(),
            2
        );
        assert_eq!(
            host.calls()
                .iter()
                .filter(|call| matches!(call, HostCall::EndPresentation))
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn hide_failures_end_presentation_without_interpreting_adapter_error_text() {
        let host = Arc::new(RecordingRuntimeHost::with_hide_error(
            "Capture window is not open",
        ));
        let (runtime, _, _) = make_runtime(host.clone(), vec![make_snapshot()]);
        let error = runtime
            .create_session_from_visible_desktop()
            .await
            .unwrap_err();
        assert_eq!(error.to_string(), "Capture window is not open");
        assert_eq!(
            host.calls(),
            vec![
                HostCall::BeginPresentation,
                HostCall::HideCaptureWindow,
                HostCall::EndPresentation
            ]
        );
    }

    #[tokio::test]
    async fn failed_pin_delivery_is_not_repeated_and_keeps_the_session_available() {
        let host = Arc::new(RecordingRuntimeHost::succeeds());
        let (runtime, sessions, pin_output) = make_runtime(host, vec![make_renderable_snapshot()]);
        let session = sessions.create_session().await.unwrap();
        *pin_output.error.lock().unwrap() = Some("pin window failed".into());
        let error = runtime
            .output_selection(
                &session.id,
                &LogicalRect {
                    x: 0.0,
                    y: 0.0,
                    width: 2.0,
                    height: 2.0,
                },
                &[],
                false,
                CaptureOutputAction::Pin,
            )
            .await
            .unwrap_err();
        assert_eq!(error.to_string(), "pin window failed");
        assert_eq!(pin_output.pngs.lock().unwrap().len(), 1);
        assert!(sessions.has_session(&session.id));
    }

    mod ocr_completion_tests {
        use super::*;
        use crate::application::capture::{CaptureOcrStatus, CaptureOcrTarget};
        use crate::application::providers::{common::Provider, ocr::OcrProvider};
        use crate::domain::{OcrRequest, OcrResult, OcrSettings};
        use tokio::sync::Notify;

        struct SuspendedOcr {
            barrier: Arc<StartupBarrier>,
            request: Arc<Mutex<Option<OcrRequest>>>,
            stopped: Arc<Notify>,
            result: Result<&'static str, &'static str>,
        }

        impl Provider for SuspendedOcr {
            fn id(&self) -> &str {
                "suspended"
            }
            fn name(&self) -> &str {
                "Suspended OCR"
            }
            fn is_configured(&self) -> bool {
                true
            }
            fn requires_api_key(&self) -> bool {
                false
            }
            fn reconfigure_credentials(
                &mut self,
                _: &std::collections::HashMap<String, String>,
            ) -> crate::Result<()> {
                Ok(())
            }
        }

        struct NotifyOnDrop(Arc<Notify>);
        impl Drop for NotifyOnDrop {
            fn drop(&mut self) {
                self.0.notify_one();
            }
        }

        #[async_trait]
        impl OcrProvider for SuspendedOcr {
            async fn recognize(&self, request: &OcrRequest) -> crate::Result<OcrResult> {
                let _stopped = NotifyOnDrop(self.stopped.clone());
                *self.request.lock().unwrap() = Some(request.clone());
                self.barrier.entered.notify_one();
                self.barrier.release.notified().await;
                self.result
                    .map(|text| OcrResult::from_text(text, Some(0.9)))
                    .map_err(Into::into)
            }
        }

        struct Fixture {
            runtime: Arc<CaptureSessionRuntime>,
            host: Arc<RecordingRuntimeHost>,
            session_id: CaptureSessionId,
            barrier: Arc<StartupBarrier>,
            request: Arc<Mutex<Option<OcrRequest>>>,
            stopped: Arc<Notify>,
        }

        async fn prepared(
            target: CaptureOcrTarget,
            result: Result<&'static str, &'static str>,
            hide_status: bool,
        ) -> Fixture {
            let host = Arc::new(RecordingRuntimeHost::succeeds());
            let (runtime, _, _) = make_runtime(host.clone(), vec![make_renderable_snapshot()]);
            let runtime = Arc::new(runtime);
            let barrier = Arc::new(StartupBarrier::default());
            let request = Arc::new(Mutex::new(None));
            let stopped = Arc::new(Notify::new());
            runtime
                .ocr
                .register(SuspendedOcr {
                    barrier: barrier.clone(),
                    request: request.clone(),
                    stopped: stopped.clone(),
                    result,
                })
                .unwrap();
            runtime.ocr.activate("suspended").unwrap();
            let session_id = runtime
                .create_session_from_visible_desktop()
                .await
                .unwrap()
                .id;
            runtime
                .prepare_capture_ocr(
                    &session_id,
                    &LogicalRect {
                        x: 1.0,
                        y: 1.0,
                        width: 2.0,
                        height: 2.0,
                    },
                    &[],
                    target,
                    OcrSettings {
                        recognition_language: "ja".into(),
                        hide_silent_status: hide_status,
                        ..OcrSettings::default()
                    },
                )
                .await
                .unwrap();
            assert!(
                request.lock().unwrap().is_none(),
                "preparation must not start recognition"
            );
            Fixture {
                runtime,
                host,
                session_id,
                barrier,
                request,
                stopped,
            }
        }

        async fn notified(notify: &Notify) {
            tokio::time::timeout(std::time::Duration::from_secs(2), notify.notified())
                .await
                .expect("OCR did not reach the expected stage");
        }

        #[tokio::test]
        async fn ocr_job_survives_window_cleanup_and_uses_owned_pixels() {
            for target in [
                CaptureOcrTarget::OcrWindow,
                CaptureOcrTarget::TranslationWindow,
                CaptureOcrTarget::Clipboard,
            ] {
                let f = prepared(target, Ok("你 好\n世 界"), false).await;
                f.runtime.complete_capture_ocr(&f.session_id).await.unwrap();
                assert!(!f.runtime.sessions.has_session(&f.session_id));
                assert_eq!(
                    f.host
                        .destroy_calls
                        .load(std::sync::atomic::Ordering::Relaxed),
                    1
                );
                notified(&f.barrier.entered).await;
                assert!(f.host.ocr_results.lock().unwrap().is_empty());
                let request = f.request.lock().unwrap().clone().unwrap();
                assert_eq!(request.language.as_deref(), Some("ja"));
                let image = image::load_from_memory(&request.image_data)
                    .unwrap()
                    .to_rgba8();
                assert_eq!(image.dimensions(), (4, 4));
                assert_eq!(image.get_pixel(0, 0).0, [10, 20, 30, 255]);

                // This models the late Destroyed event from the closed WebView.
                f.runtime
                    .cancel_capture_session(&f.session_id)
                    .await
                    .unwrap();
                assert!(f.runtime.complete_capture_ocr(&f.session_id).await.is_err());
                f.barrier.release.notify_one();
                notified(&f.host.ocr_completed).await;
                let results = f.host.ocr_results.lock().unwrap();
                assert_eq!(results.len(), 1);
                assert_eq!(results[0].0, target);
                assert_eq!(
                    results[0].1.text,
                    if target == CaptureOcrTarget::Clipboard {
                        "你好\n世界"
                    } else {
                        "你 好\n世 界"
                    }
                );
                assert_eq!(
                    results[0].2.is_some(),
                    target == CaptureOcrTarget::OcrWindow
                );
            }
        }

        #[tokio::test]
        async fn cancellation_before_handoff_drops_prepared_ocr_without_running_it() {
            let f = prepared(CaptureOcrTarget::Clipboard, Ok("cancelled"), false).await;
            f.runtime
                .cancel_capture_session(&f.session_id)
                .await
                .unwrap();
            assert!(f.runtime.complete_capture_ocr(&f.session_id).await.is_err());
            assert!(f.request.lock().unwrap().is_none());
            assert!(f.host.ocr_results.lock().unwrap().is_empty());
        }

        #[tokio::test]
        async fn new_capture_cancels_in_flight_ocr_and_prevents_late_delivery() {
            let f = prepared(CaptureOcrTarget::TranslationWindow, Ok("stale"), false).await;
            f.runtime.complete_capture_ocr(&f.session_id).await.unwrap();
            notified(&f.barrier.entered).await;
            let new_session = f
                .runtime
                .create_session_from_visible_desktop()
                .await
                .unwrap();
            notified(&f.stopped).await;
            f.barrier.release.notify_one();
            assert!(f.host.ocr_results.lock().unwrap().is_empty());
            assert!(f.host.ocr_errors.lock().unwrap().is_empty());
            assert!(f.runtime.sessions.has_session(&new_session.id));
            assert_eq!(f.host.ocr_statuses.lock().unwrap().last(), Some(&None));
        }

        #[tokio::test]
        async fn failures_and_empty_results_report_after_the_overlay_has_closed() {
            for result in [Err("recognition failed"), Ok("  \n ")] {
                let f = prepared(CaptureOcrTarget::Clipboard, result, false).await;
                f.runtime.complete_capture_ocr(&f.session_id).await.unwrap();
                notified(&f.barrier.entered).await;
                f.barrier.release.notify_one();
                notified(&f.host.ocr_completed).await;
                assert!(!f.runtime.sessions.has_session(&f.session_id));
                assert!(f.host.ocr_results.lock().unwrap().is_empty());
                assert_eq!(f.host.ocr_errors.lock().unwrap().len(), 1);
            }
        }

        #[tokio::test]
        async fn silent_status_preference_does_not_suppress_copying() {
            let f = prepared(CaptureOcrTarget::Clipboard, Ok("recognized"), true).await;
            f.runtime.complete_capture_ocr(&f.session_id).await.unwrap();
            notified(&f.barrier.entered).await;
            f.barrier.release.notify_one();
            notified(&f.host.ocr_completed).await;
            assert_eq!(f.host.ocr_results.lock().unwrap().len(), 1);
            assert!(!f
                .host
                .ocr_statuses
                .lock()
                .unwrap()
                .contains(&Some(CaptureOcrStatus::Recognizing)));
            assert!(!f
                .host
                .ocr_statuses
                .lock()
                .unwrap()
                .contains(&Some(CaptureOcrStatus::Copied)));
        }

        #[tokio::test]
        async fn background_ocr_does_not_overwrite_a_newer_clipboard_copy() {
            let f = prepared(CaptureOcrTarget::Clipboard, Ok("older OCR text"), false).await;
            f.runtime.complete_capture_ocr(&f.session_id).await.unwrap();
            notified(&f.barrier.entered).await;
            f.host
                .clipboard_revision
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            f.barrier.release.notify_one();
            notified(&f.stopped).await;
            assert!(f.host.ocr_results.lock().unwrap().is_empty());
            assert!(f.host.ocr_errors.lock().unwrap().is_empty());
            assert_eq!(f.host.ocr_statuses.lock().unwrap().last(), Some(&None));
        }
    }
}

//! Native Windows capture host; all session/output rules remain in core.
mod actions;
mod view;

use crate::application::capture::{
    merge_selection_history, remember_selection, CaptureOcrDelivery, CaptureOcrEffects,
    CaptureOcrStatus, CaptureOcrTarget, CaptureSelectionMode, CaptureSessionRuntimeHost,
    CaptureWindowGeometry,
};
use crate::domain::capture::{CaptureSessionId, LogicalRect};
use crate::domain::ocr::OcrResult;
use crate::infrastructure::ui_thread;
use crate::{
    CaptureCursorMover, CaptureOutput, CaptureSessionRuntime, CaptureSessions, Result,
    SettingsConfiguration,
};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock, Weak};

#[derive(Clone)]
pub(super) struct Services {
    runtime: Weak<CaptureSessionRuntime>,
    sessions: Arc<CaptureSessions>,
    settings: Arc<SettingsConfiguration>,
    output: Arc<CaptureOutput>,
    cursor: Arc<dyn CaptureCursorMover>,
    favorites: Weak<crate::application::ScreenshotFavoriteCapture>,
}
impl Services {
    fn runtime(&self) -> Result<Arc<CaptureSessionRuntime>> {
        self.runtime
            .upgrade()
            .ok_or_else(|| "Capture runtime has stopped".into())
    }
}
pub(crate) struct WindowsCaptureSessionRuntimeHost {
    services: RwLock<Option<Services>>,
    delivery: Arc<CaptureOcrDelivery>,
    history_path: PathBuf,
    history: Mutex<()>,
}
impl WindowsCaptureSessionRuntimeHost {
    pub(crate) fn new(delivery: Arc<CaptureOcrDelivery>, history_path: PathBuf) -> Self {
        Self {
            services: RwLock::new(None),
            delivery,
            history_path,
            history: Mutex::new(()),
        }
    }
    pub(crate) fn bind(&self, state: &crate::AppState) {
        *self.services.write().unwrap() = Some(Services {
            runtime: Arc::downgrade(&state.capture.runtime),
            sessions: state.capture.sessions.clone(),
            settings: state.settings.configuration.clone(),
            output: state.capture.output.clone(),
            cursor: state.capture.cursor.clone(),
            favorites: Arc::downgrade(&state.screenshot_favorites.capture),
        });
    }
    fn services(&self) -> Result<Services> {
        self.services
            .read()
            .unwrap()
            .clone()
            .ok_or_else(|| "Native capture services are not initialized".into())
    }
    fn read_history(&self) -> Vec<LogicalRect> {
        std::fs::read(&self.history_path)
            .ok()
            .and_then(|data| serde_json::from_slice(&data).ok())
            .unwrap_or_default()
    }
    fn write_history(&self, history: &[LogicalRect]) -> Result<()> {
        if let Some(parent) = self.history_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.history_path, serde_json::to_vec(history)?)?;
        Ok(())
    }
    pub(crate) fn settings_changed(&self) {
        if let Ok(services) = self.services() {
            if let Ok(settings) = services.settings.snapshot() {
                ui_thread::dispatch(move || view::settings_changed(settings));
            }
        }
    }
}
#[async_trait::async_trait]
impl CaptureSessionRuntimeHost for WindowsCaptureSessionRuntimeHost {
    async fn request_capture_selection_pin(&self, id: &CaptureSessionId) -> Result<bool> {
        let id = id.0.clone();
        ui_thread::run("pin capture selection", move || Ok(view::request_pin(&id))).await
    }
    fn clipboard_revision(&self) -> u64 {
        unsafe { windows::Win32::System::DataExchange::GetClipboardSequenceNumber() as u64 }
    }
    fn reserve_capture_ocr_result(&self, target: CaptureOcrTarget) -> Result<u64> {
        self.delivery.reserve_result(target)
    }
    fn set_capture_ocr_status(&self, status: Option<CaptureOcrStatus>) {
        crate::app_shell::set_capture_ocr_status_native(status);
    }
    async fn publish_capture_ocr(
        &self,
        target: CaptureOcrTarget,
        result_id: u64,
        result: OcrResult,
        preview: Option<String>,
    ) -> Result<()> {
        self.delivery
            .publish(target, result_id, result, preview)
            .await
    }
    async fn report_capture_ocr_error(
        &self,
        target: CaptureOcrTarget,
        result_id: u64,
        error: &str,
    ) {
        self.delivery.report_error(target, result_id, error);
    }
    async fn begin_capture_presentation(&self) -> Result<()> {
        view::set_active(true);
        Ok(())
    }
    async fn end_capture_presentation(&self) -> Result<()> {
        view::set_active(false);
        Ok(())
    }
    async fn prepare_capture_window(&self) -> Result<()> {
        let old = ui_thread::run("prepare capture overlay", || Ok(view::close())).await?;
        if let Some(id) = old {
            if let Ok(runtime) = self.services().and_then(|s| s.runtime()) {
                crate::infrastructure::runtime::spawn(async move {
                    let _ = runtime.cancel_capture_session(&CaptureSessionId(id)).await;
                });
            }
        }
        Ok(())
    }
    async fn prepare_capture_window_for_reveal(
        &self,
        id: Option<&CaptureSessionId>,
        _: Option<&CaptureWindowGeometry>,
    ) -> Result<()> {
        let id = id.cloned();
        ui_thread::run("validate capture overlay", move || {
            if id.as_ref().is_none_or(|id| view::is_current(&id.0)) {
                Ok(())
            } else {
                Err("Capture session is no longer current".into())
            }
        })
        .await
    }
    async fn reveal_capture_window(&self, _: Option<&CaptureWindowGeometry>) -> Result<()> {
        ui_thread::run("reveal capture overlay", view::reveal).await
    }
    async fn hide_capture_window(&self) -> Result<()> {
        ui_thread::run("hide capture overlay", || {
            view::hide();
            Ok(())
        })
        .await
    }
    async fn set_capture_window_cursor_passthrough(&self, enabled: bool) -> Result<()> {
        ui_thread::run("set capture cursor passthrough", move || {
            view::passthrough(enabled)
        })
        .await
    }
    async fn destroy_inactive_capture_window(&self) -> Result<()> {
        ui_thread::run("close inactive capture", || {
            if !view::is_active() {
                view::close();
            }
            Ok(())
        })
        .await
    }
    async fn open_capture_window_for_session(
        &self,
        mode: &str,
        id: &str,
        geometry: &CaptureWindowGeometry,
    ) -> Result<()> {
        let services = self.services()?;
        let session = services
            .sessions
            .get_session(&CaptureSessionId(id.into()))?;
        let preferences = services.settings.snapshot()?;
        let mode = CaptureSelectionMode::parse(mode).ok_or("Unknown capture mode")?;
        let history = self.capture_selection_history(vec![])?;
        let geometry = geometry.clone();
        ui_thread::run_with_event_loop("open capture overlay", move |event_loop| {
            view::open(
                event_loop,
                services,
                session,
                mode,
                geometry,
                preferences,
                history,
            )
        })
        .await
    }
    async fn restore_capture_snapshot_windows(&self, _: Vec<String>) -> Result<()> {
        Ok(())
    }
    async fn capture_editor_owns_focus(&self, id: &CaptureSessionId) -> Result<bool> {
        let id = id.0.clone();
        ui_thread::run("check capture editor focus", move || {
            Ok(view::owns_focus(&id))
        })
        .await
    }
    fn capture_selection_history(&self, legacy: Vec<LogicalRect>) -> Result<Vec<LogicalRect>> {
        let _lock = self.history.lock().unwrap();
        let history = merge_selection_history(self.read_history(), legacy);
        self.write_history(&history)?;
        Ok(history)
    }
    fn remember_capture_selection(&self, rect: &LogicalRect) -> Result<()> {
        let _lock = self.history.lock().unwrap();
        self.write_history(&remember_selection(self.read_history(), rect.clone()))
    }
}
pub(crate) struct WindowsCaptureOcrEffects;
impl CaptureOcrEffects for WindowsCaptureOcrEffects {
    fn copy_text(&self, text: String) -> Result<()> {
        crate::infrastructure::system::clipboard_text::copy_text(text).map_err(Into::into)
    }
    fn show_error(&self, error: &str) {
        super::super::dialogs::show_native_error("SnapLingo — OCR", error);
    }
}
pub(crate) use view::{
    accessibility_event as windows_accessibility, close_all as windows_close_all,
    window_event as windows_window_event,
};
pub(crate) fn yield_capture_window_focus() {
    view::hide();
}
pub(crate) fn windows_scrolling_ended(id: &CaptureSessionId, completed: bool) {
    if completed {
        view::close_session(&id.0);
    } else {
        view::action_finished(&id.0, None);
    }
}

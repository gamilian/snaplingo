use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use objc2::{rc::Retained, AnyThread};
use objc2_foundation::{NSString, NSUserDefaults};
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

use crate::application::capture::{
    merge_selection_history, remember_selection, CaptureOcrDelivery, CaptureOcrStatus,
    CaptureOcrTarget, CaptureSelectionMode, CaptureSessionRuntimeHost, CaptureWindowGeometry,
};
use crate::domain::capture::{CaptureOutputAction, CaptureSessionId, LogicalPoint, LogicalRect};
use crate::domain::ocr::OcrResult;
use crate::Result;

use super::native_view::{self as native, Action, Operation};
use super::runtime_host::{run_on_main_thread, TauriCaptureSessionRuntimeHost};

pub(crate) struct NativeCaptureSessionRuntimeHost {
    app: AppHandle,
    legacy: TauriCaptureSessionRuntimeHost,
    history_lock: Mutex<()>,
    history_suite: String,
}

const HISTORY_KEY: &str = "SnapLingoCaptureSelectionHistory";

impl NativeCaptureSessionRuntimeHost {
    pub(crate) fn new(app: AppHandle, delivery: Arc<CaptureOcrDelivery>) -> Self {
        Self {
            legacy: TauriCaptureSessionRuntimeHost::new(app.clone(), delivery),
            history_suite: app.config().identifier.clone(),
            app,
            history_lock: Mutex::new(()),
        }
    }
    fn defaults(&self) -> Retained<NSUserDefaults> {
        NSUserDefaults::initWithSuiteName(
            NSUserDefaults::alloc(),
            Some(&NSString::from_str(&self.history_suite)),
        )
        .unwrap()
    }
    fn read_history(&self) -> Vec<LogicalRect> {
        self.defaults()
            .stringForKey(&NSString::from_str(HISTORY_KEY))
            .and_then(|json| serde_json::from_str(&json.to_string()).ok())
            .unwrap_or_default()
    }
    fn write_history(&self, history: &[LogicalRect]) -> Result<()> {
        let json = serde_json::to_string(history)?;
        // NSString is an immutable property-list value accepted by user defaults.
        unsafe {
            self.defaults().setObject_forKey(
                Some(&NSString::from_str(&json)),
                &NSString::from_str(HISTORY_KEY),
            );
        }
        Ok(())
    }
}

#[async_trait]
impl CaptureSessionRuntimeHost for NativeCaptureSessionRuntimeHost {
    async fn request_capture_selection_pin(&self, session_id: &CaptureSessionId) -> Result<bool> {
        let id = session_id.0.clone();
        if run_on_main_thread(&self.app, "pin native capture selection", move |_| {
            Ok(native::request_pin(&id))
        })
        .await?
        {
            return Ok(true);
        }
        self.legacy.request_capture_selection_pin(session_id).await
    }
    fn clipboard_revision(&self) -> u64 {
        self.legacy.clipboard_revision()
    }
    fn reserve_capture_ocr_result(&self, target: CaptureOcrTarget) -> Result<u64> {
        self.legacy.reserve_capture_ocr_result(target)
    }
    fn set_capture_ocr_status(&self, status: Option<CaptureOcrStatus>) {
        self.legacy.set_capture_ocr_status(status);
    }
    async fn publish_capture_ocr(
        &self,
        target: CaptureOcrTarget,
        result_id: u64,
        result: OcrResult,
        preview: Option<String>,
    ) -> Result<()> {
        self.legacy
            .publish_capture_ocr(target, result_id, result, preview)
            .await
    }
    async fn report_capture_ocr_error(
        &self,
        target: CaptureOcrTarget,
        result_id: u64,
        error: &str,
    ) {
        self.legacy
            .report_capture_ocr_error(target, result_id, error)
            .await;
    }
    async fn begin_capture_presentation(&self) -> Result<()> {
        self.legacy.begin_capture_presentation().await
    }
    async fn end_capture_presentation(&self) -> Result<()> {
        self.legacy.end_capture_presentation().await
    }
    async fn prepare_capture_window(&self) -> Result<()> {
        self.legacy.prepare_capture_window().await
    }
    async fn prepare_capture_window_for_mode(&self, mode: &str) -> Result<()> {
        if CaptureSelectionMode::parse(mode).is_none() {
            return self.legacy.prepare_capture_window().await;
        }
        // Initial selection does not need the annotation editor's dormant WebView.
        let closing = run_on_main_thread(&self.app, "release capture editor", |app| {
            super::tauri::release_capture_editor(&app)
        })
        .await?;
        if closing {
            super::tauri::wait_for_capture_window_label_release(|| {
                self.app
                    .get_webview_window(super::backend::CAPTURE_WINDOW_LABEL)
                    .is_some()
            })
            .await?;
        }
        Ok(())
    }
    async fn open_capture_editor_window(
        &self,
        selection: &crate::application::capture::CaptureEditorSelection,
        geometry: &CaptureWindowGeometry,
    ) -> Result<()> {
        let id = selection.session_id.clone();
        if !run_on_main_thread(&self.app, "check native editor selection", move |_| {
            Ok(native::is_current(&id))
        })
        .await?
        {
            return Err("Capture editor handoff was canceled".into());
        }
        self.legacy.prepare_capture_window().await?;
        self.legacy
            .open_capture_window_for_session("screenshot", &selection.session_id, geometry)
            .await
    }
    async fn finish_capture_editor_transfer(&self, session_id: &CaptureSessionId) -> Result<()> {
        let id = session_id.0.clone();
        run_on_main_thread(&self.app, "finish native editor handoff", move |_| {
            native::close_session(&id);
            Ok(())
        })
        .await
    }
    async fn discard_capture_editor(&self) -> Result<()> {
        let closing = run_on_main_thread(&self.app, "discard capture editor", |app| {
            super::tauri::release_capture_editor(&app)
        })
        .await?;
        if closing {
            super::tauri::wait_for_capture_window_label_release(|| {
                self.app
                    .get_webview_window(super::backend::CAPTURE_WINDOW_LABEL)
                    .is_some()
            })
            .await?;
        }
        Ok(())
    }
    async fn update_capture_editor_toolbar(
        &self,
        id: &CaptureSessionId,
        presentation: Option<&crate::application::capture::CaptureEditorToolbarPresentation>,
    ) -> Result<bool> {
        let id = id.0.clone();
        let presentation = presentation.cloned();
        run_on_main_thread(&self.app, "update native editor toolbar", move |app| {
            super::editor_toolbar_macos::update(&app, &id, presentation.as_ref())
        })
        .await
    }
    async fn capture_editor_owns_focus(&self, id: &CaptureSessionId) -> Result<bool> {
        let id = id.0.clone();
        run_on_main_thread(&self.app, "check native editor focus", move |_| {
            Ok(super::editor_toolbar_macos::owns_focus(&id))
        })
        .await
    }
    fn capture_selection_history(
        &self,
        legacy_history: Vec<LogicalRect>,
    ) -> Result<Vec<LogicalRect>> {
        let _history = self.history_lock.lock().unwrap();
        let history = merge_selection_history(self.read_history(), legacy_history);
        self.write_history(&history)?;
        Ok(history)
    }
    fn remember_capture_selection(&self, rect: &LogicalRect) -> Result<()> {
        let _history = self.history_lock.lock().unwrap();
        self.write_history(&remember_selection(self.read_history(), rect.clone()))
    }
    async fn prepare_capture_window_for_reveal(
        &self,
        session_id: Option<&CaptureSessionId>,
        geometry: Option<&CaptureWindowGeometry>,
    ) -> Result<()> {
        self.legacy
            .prepare_capture_window_for_reveal(session_id, geometry)
            .await
    }
    async fn reveal_capture_window(&self, geometry: Option<&CaptureWindowGeometry>) -> Result<()> {
        self.legacy.reveal_capture_window(geometry).await
    }
    async fn hide_capture_window(&self) -> Result<()> {
        let old_id = run_on_main_thread(&self.app, "hide native capture", |_| {
            let id = native::session_id();
            native::close();
            Ok(id)
        })
        .await?;
        if let Some(old_id) = old_id {
            if let Some(state) = self.app.try_state::<crate::AppState>() {
                let runtime = state.capture.runtime.clone();
                // As with the legacy window's destruction listener, clean up the
                // replaced ID after startup releases its lock, never a newer session.
                tauri::async_runtime::spawn(async move {
                    if let Err(error) = runtime
                        .cancel_capture_session(&CaptureSessionId(old_id))
                        .await
                    {
                        log::warn!("Failed to release native capture session: {error}");
                    }
                });
            }
        }
        self.legacy.hide_capture_window().await
    }
    async fn set_capture_window_cursor_passthrough(&self, enabled: bool) -> Result<()> {
        self.legacy
            .set_capture_window_cursor_passthrough(enabled)
            .await
    }
    async fn destroy_inactive_capture_window(&self) -> Result<()> {
        if super::is_capture_presentation_active() {
            return Ok(());
        }
        self.discard_capture_editor().await
    }
    async fn open_capture_window_for_session(
        &self,
        mode: &str,
        session_id: &str,
        geometry: &CaptureWindowGeometry,
    ) -> Result<()> {
        let Some(mode) = CaptureSelectionMode::parse(mode) else {
            return self
                .legacy
                .open_capture_window_for_session(mode, session_id, geometry)
                .await;
        };
        let state = self.app.state::<crate::AppState>();
        let session = state
            .capture
            .sessions
            .get_session(&CaptureSessionId(session_id.to_owned()))?;
        let preferences = state.settings.configuration.snapshot()?.screenshot;
        let history = self.capture_selection_history(Vec::new())?;
        let geometry = geometry.clone();
        run_on_main_thread(&self.app, "open native quick capture", move |app| {
            let id = session.id.clone();
            let controls = Arc::new(Mutex::new(ControlRequests::default()));
            native::open(
                session,
                mode,
                geometry,
                preferences,
                history,
                move |action| {
                    dispatch(app.clone(), id.clone(), mode, controls.clone(), action);
                },
            )
        })
        .await
    }
    async fn restore_capture_snapshot_windows(
        &self,
        hidden_window_labels: Vec<String>,
    ) -> Result<()> {
        self.legacy
            .restore_capture_snapshot_windows(hidden_window_labels)
            .await
    }
}

#[derive(Default)]
struct ControlRequests {
    running: bool,
    pending: Option<(LogicalPoint, u64)>,
}

fn dispatch(
    app: AppHandle,
    id: CaptureSessionId,
    mode: CaptureSelectionMode,
    controls: Arc<Mutex<ControlRequests>>,
    action: Action,
) {
    if matches!(action, Action::Cancel) {
        app.state::<crate::AppState>()
            .capture
            .runtime
            .abort_capture_editor(&id);
    }
    if let Action::Inspect(point, revision) = action {
        let mut queue = controls.lock().unwrap();
        queue.pending = Some((point, revision));
        if queue.running {
            return;
        }
        queue.running = true;
        drop(queue);
        tauri::async_runtime::spawn(async move {
            loop {
                let request = {
                    let mut queue = controls.lock().unwrap();
                    match queue.pending.take() {
                        Some(request) => request,
                        None => {
                            queue.running = false;
                            return;
                        }
                    }
                };
                let runtime = app.state::<crate::AppState>().capture.runtime.clone();
                let result = runtime
                    .control_candidate_at(&id, &request.0)
                    .await
                    .map(|candidate| candidate.map(|candidate| candidate.rect))
                    .map_err(|error| error.to_string());
                let id = id.0.clone();
                let _ = run_on_main_thread(&app, "update capture control candidate", move |_| {
                    native::control_result(&id, request.1, result);
                    Ok(())
                })
                .await;
            }
        });
        return;
    }
    let terminal = matches!(
        action,
        Action::Complete(_, _, _) | Action::Cancel | Action::Refresh
    );
    let refreshing = matches!(action, Action::Refresh);
    tauri::async_runtime::spawn(async move {
        if let Err(error) = run_action(&app, &id, mode, action).await {
            let error = error.to_string();
            let _ = run_on_main_thread(&app, "report capture action error", move |app| {
                if refreshing
                    && native::session_id().is_none()
                    && !super::is_capture_presentation_active()
                {
                    app.dialog()
                        .message(error)
                        .title("截图失败")
                        .kind(MessageDialogKind::Error)
                        .show(|_| {});
                } else if terminal {
                    native::action_finished(&id.0, Some(error));
                } else {
                    native::report_error(&id.0, &error);
                }
                Ok(())
            })
            .await;
        }
    });
}

async fn run_action(
    app: &AppHandle,
    id: &CaptureSessionId,
    mode: CaptureSelectionMode,
    action: Action,
) -> Result<()> {
    let state = app.state::<crate::AppState>();
    let runtime = state.capture.runtime.clone();
    match action {
        Action::Cancel => {
            let close_id = id.0.clone();
            run_on_main_thread(app, "cancel native capture", move |_| {
                native::close_session(&close_id);
                Ok(())
            })
            .await?;
            runtime.cancel_capture_session(id).await
        }
        Action::Refresh => {
            let check_id = id.0.clone();
            if !run_on_main_thread(app, "check capture refresh", move |_| {
                Ok(native::is_current(&check_id))
            })
            .await?
            {
                return Ok(());
            }
            let close_id = id.0.clone();
            run_on_main_thread(app, "refresh native capture", move |_| {
                native::close_session(&close_id);
                Ok(())
            })
            .await?;
            runtime.cancel_capture_session(id).await?;
            runtime.open_capture_window_for_mode(mode.as_str()).await
        }
        Action::MoveCursor(dx, dy) => {
            state.capture.cursor.move_relative(dx, dy);
            Ok(())
        }
        Action::CopyText(text) => {
            crate::infrastructure::system::clipboard_text::copy_text(text).map_err(Into::into)
        }
        Action::Complete(operation, rect, include_cursor) => {
            let check_id = id.0.clone();
            if !run_on_main_thread(app, "check native capture session", move |_| {
                Ok(native::is_current(&check_id))
            })
            .await?
            {
                return Ok(());
            }
            if operation == Operation::Edit {
                return runtime.open_capture_editor(id, &rect, include_cursor).await;
            }
            let screenshot = state.settings.configuration.snapshot()?.screenshot;
            if operation == Operation::Primary
                && matches!(
                    mode,
                    CaptureSelectionMode::Ocr
                        | CaptureSelectionMode::SilentOcr
                        | CaptureSelectionMode::Translate
                )
            {
                let target = match mode {
                    CaptureSelectionMode::Ocr => CaptureOcrTarget::OcrWindow,
                    CaptureSelectionMode::SilentOcr => CaptureOcrTarget::Clipboard,
                    CaptureSelectionMode::Translate => CaptureOcrTarget::TranslationWindow,
                    CaptureSelectionMode::Copy | CaptureSelectionMode::Edit => unreachable!(),
                };
                runtime
                    .prepare_capture_ocr(
                        id,
                        &rect,
                        &[],
                        target,
                        state.settings.configuration.snapshot()?.ocr,
                    )
                    .await?;
                return runtime.complete_capture_ocr(id).await;
            }
            let output = match operation {
                Operation::Edit => unreachable!(),
                Operation::Primary | Operation::Copy => CaptureOutputAction::Copy,
                Operation::Pin => CaptureOutputAction::Pin,
                Operation::QuickSave => {
                    let path = state.capture.output.quick_capture_save_path(
                        Some(&screenshot.save_path),
                        &screenshot.format,
                        &screenshot.naming_rule,
                        &screenshot.custom_file_name,
                    );
                    save_action(path, &screenshot, false)
                }
                Operation::Save => {
                    let suspend_id = id.0.clone();
                    run_on_main_thread(app, "show capture save dialog", move |_| {
                        native::suspend(&suspend_id);
                        Ok(())
                    })
                    .await?;
                    let Some(path) =
                        crate::infrastructure::system::image_save_dialog::select_capture_save_path(
                            app,
                        )
                        .await?
                    else {
                        let resume_id = id.0.clone();
                        return run_on_main_thread(app, "return to capture selection", move |_| {
                            native::action_finished(&resume_id, None);
                            Ok(())
                        })
                        .await;
                    };
                    save_action(path, &screenshot, true)
                }
                Operation::Print => {
                    let png = runtime.render_png(id, &rect, &[], include_cursor)?;
                    let print_id = id.0.clone();
                    run_on_main_thread(app, "print capture", move |_| {
                        native::print(&print_id, png)
                    })
                    .await?;
                    let close_id = id.0.clone();
                    run_on_main_thread(app, "finish capture print", move |_| {
                        native::close_session(&close_id);
                        Ok(())
                    })
                    .await?;
                    return runtime.cancel_capture_session(id).await;
                }
            };
            runtime
                .complete_capture_selection(id, &rect, include_cursor, output)
                .await
        }
        Action::Inspect(_, _) => unreachable!(),
    }
}

fn save_action(
    path: std::path::PathBuf,
    screenshot: &crate::domain::config::ScreenshotSettings,
    remember_choice: bool,
) -> CaptureOutputAction {
    CaptureOutputAction::Save {
        path: path.to_string_lossy().into_owned(),
        format: screenshot.format.clone(),
        quality: screenshot.quality,
        copy_after_save: screenshot.auto_copy,
        remember_choice,
    }
}

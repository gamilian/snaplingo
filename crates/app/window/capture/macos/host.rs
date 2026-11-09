use objc2_app_kit::NSUserInterfaceItemIdentification;
use std::sync::{Arc, Mutex, RwLock, Weak};

use async_trait::async_trait;
use objc2::{rc::Retained, AnyThread};
use objc2_app_kit::{NSApplication, NSPasteboard};
use objc2_foundation::{NSString, NSUserDefaults};

use crate::infrastructure::ui_thread;
use crate::Result;
use snaplingo_core::application::capture::{
    merge_selection_history, remember_selection, AnnotationTool, CaptureEditor,
    CaptureEditorStylePresentation, CaptureOcrDelivery, CaptureOcrStatus, CaptureOcrTarget,
    CaptureSelectionMode, CaptureSessionRuntimeHost, CaptureWindowGeometry,
};
use snaplingo_core::application::{
    CaptureCursorMover, CaptureOutput, CaptureSessionRuntime, CaptureSessions,
    SettingsConfiguration,
};
use snaplingo_core::domain::capture::{
    CaptureOutputAction, CaptureSessionId, LogicalPoint, LogicalRect,
};
use snaplingo_core::domain::ocr::OcrResult;

use super::presentation;
use super::view::{self as native, Action, Operation};

#[derive(Clone)]
struct Services {
    runtime: Weak<CaptureSessionRuntime>,
    sessions: Arc<CaptureSessions>,
    settings: Arc<SettingsConfiguration>,
    output: Arc<CaptureOutput>,
    cursor: Arc<dyn CaptureCursorMover>,
    favorites: Weak<snaplingo_core::application::ScreenshotFavoriteCapture>,
}

impl Services {
    fn runtime(&self) -> Result<Arc<CaptureSessionRuntime>> {
        self.runtime
            .upgrade()
            .ok_or_else(|| "Capture runtime has stopped".into())
    }
}

pub(crate) struct NativeCaptureSessionRuntimeHost {
    services: RwLock<Option<Services>>,
    delivery: Arc<CaptureOcrDelivery>,
    set_status: Arc<dyn Fn(Option<CaptureOcrStatus>) + Send + Sync>,
    history_lock: Mutex<()>,
    history_suite: String,
}

const HISTORY_KEY: &str = "SnapLingoCaptureSelectionHistory";

impl NativeCaptureSessionRuntimeHost {
    pub(crate) fn new(
        history_suite: String,
        delivery: Arc<CaptureOcrDelivery>,
        set_status: impl Fn(Option<CaptureOcrStatus>) + Send + Sync + 'static,
    ) -> Self {
        Self {
            services: RwLock::new(None),
            delivery,
            set_status: Arc::new(set_status),
            history_suite,
            history_lock: Mutex::new(()),
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
        unsafe {
            self.defaults().setObject_forKey(
                Some(&NSString::from_str(&json)),
                &NSString::from_str(HISTORY_KEY),
            );
        }
        Ok(())
    }

    async fn prepare(&self) -> Result<()> {
        let old_id = ui_thread::run("prepare native capture", || {
            let id = native::session_id();
            native::close();
            Ok(id)
        })
        .await?;
        if let Some(old_id) = old_id {
            if let Ok(runtime) = self.services().and_then(|services| services.runtime()) {
                // Cleanup waits for startup to release its lock and targets only
                // the replaced session, never the newly opened one.
                crate::infrastructure::runtime::spawn(async move {
                    if let Err(error) = runtime
                        .cancel_capture_session(&CaptureSessionId(old_id))
                        .await
                    {
                        log::warn!("Failed to release replaced native capture session: {error}");
                    }
                });
            }
        }
        Ok(())
    }
}

#[async_trait]
impl CaptureSessionRuntimeHost for NativeCaptureSessionRuntimeHost {
    async fn request_capture_selection_pin(&self, session_id: &CaptureSessionId) -> Result<bool> {
        let id = session_id.0.clone();
        ui_thread::run("pin native capture selection", move || {
            Ok(native::request_pin(&id))
        })
        .await
    }
    fn clipboard_revision(&self) -> u64 {
        NSPasteboard::generalPasteboard().changeCount() as u64
    }
    fn reserve_capture_ocr_result(&self, target: CaptureOcrTarget) -> Result<u64> {
        self.delivery.reserve_result(target)
    }
    fn set_capture_ocr_status(&self, status: Option<CaptureOcrStatus>) {
        (self.set_status)(status);
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
        ui_thread::run(
            "begin native capture presentation",
            presentation::begin_capture_presentation,
        )
        .await
    }
    async fn end_capture_presentation(&self) -> Result<()> {
        ui_thread::run(
            "end native capture presentation",
            presentation::end_capture_presentation,
        )
        .await
    }
    async fn prepare_capture_window(&self) -> Result<()> {
        self.prepare().await
    }
    async fn prepare_capture_window_for_mode(&self, _mode: &str) -> Result<()> {
        self.prepare().await
    }
    async fn capture_editor_owns_focus(&self, id: &CaptureSessionId) -> Result<bool> {
        let id = id.0.clone();
        ui_thread::run("check native editor focus", move || {
            Ok(super::toolbar::owns_focus(&id))
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
        _geometry: Option<&CaptureWindowGeometry>,
    ) -> Result<()> {
        let id = session_id.cloned();
        ui_thread::run("prepare native capture reveal", move || {
            if id.is_some_and(|id| !native::is_current(&id.0)) {
                return Err("Native capture session is no longer current".into());
            }
            Ok(())
        })
        .await
    }
    async fn reveal_capture_window(&self, _geometry: Option<&CaptureWindowGeometry>) -> Result<()> {
        ui_thread::run("reveal native capture", native::reveal).await
    }
    async fn hide_capture_window(&self) -> Result<()> {
        ui_thread::run("hide native capture", || {
            if let Some(id) = native::session_id() {
                native::suspend(&id);
            }
            Ok(())
        })
        .await
    }
    async fn destroy_inactive_capture_window(&self) -> Result<()> {
        ui_thread::run("release inactive native capture", || {
            if !presentation::is_capture_presentation_active() {
                native::close();
            }
            Ok(())
        })
        .await
    }
    async fn open_capture_window_for_session(
        &self,
        mode: &str,
        session_id: &str,
        geometry: &CaptureWindowGeometry,
    ) -> Result<()> {
        let mode = CaptureSelectionMode::parse(mode).unwrap_or(CaptureSelectionMode::Edit);
        let services = self.services()?;
        let session = services
            .sessions
            .get_session(&CaptureSessionId(session_id.into()))?;
        let preferences = services.settings.snapshot()?.screenshot;
        let history = self.capture_selection_history(Vec::new())?;
        let geometry = geometry.clone();
        ui_thread::run("open native capture", move || {
            let id = session.id.clone();
            let controls = Arc::new(Mutex::new(ControlRequests::default()));
            native::open(
                session,
                mode,
                geometry,
                preferences,
                history,
                move |action| {
                    dispatch(services.clone(), id.clone(), mode, controls.clone(), action);
                },
            )
        })
        .await
    }
    async fn restore_capture_snapshot_windows(
        &self,
        hidden_window_labels: Vec<String>,
    ) -> Result<()> {
        let labels =
            super::backend::capture_snapshot_window_labels_to_restore(&hidden_window_labels);
        ui_thread::run("restore native capture snapshot windows", move || {
            let app = NSApplication::sharedApplication(objc2::MainThreadMarker::new().unwrap());
            for window in app.windows().iter() {
                if window
                    .identifier()
                    .is_some_and(|id| labels.contains(&id.to_string()))
                {
                    window.orderFront(None);
                }
            }
            Ok(())
        })
        .await
    }
}

#[derive(Default)]
struct ControlRequests {
    running: bool,
    pending: Option<(LogicalPoint, u64)>,
    rendering: bool,
    render_pending: Option<super::canvas::Request>,
}

fn dispatch(
    services: Services,
    id: CaptureSessionId,
    mode: CaptureSelectionMode,
    controls: Arc<Mutex<ControlRequests>>,
    action: Action,
) {
    if let Action::Editor(request @ super::canvas::Request::Render { .. }) = action {
        let mut queue = controls.lock().unwrap();
        queue.render_pending = Some(request);
        if queue.rendering {
            return;
        }
        queue.rendering = true;
        drop(queue);
        crate::infrastructure::runtime::spawn(async move {
            loop {
                let request = {
                    let mut queue = controls.lock().unwrap();
                    match queue.render_pending.take() {
                        Some(request) => request,
                        None => {
                            queue.rendering = false;
                            return;
                        }
                    }
                };
                let super::canvas::Request::Render {
                    rect,
                    annotations,
                    revision,
                    excluded,
                    include_cursor,
                } = request
                else {
                    unreachable!()
                };
                let renderer = services.runtime();
                let session = id.clone();
                let result = crate::infrastructure::runtime::spawn_blocking(move || {
                    renderer?.render_rgba(&session, &rect, &annotations, include_cursor)
                })
                .await;
                let session = id.0.clone();
                let _ = ui_thread::run("paint native annotations", move || match result {
                    Ok(Ok(pixels)) => native::editor_rendered(&session, revision, excluded, pixels),
                    Ok(Err(error)) => {
                        native::editor_render_failed(
                            &session,
                            revision,
                            excluded,
                            &error.to_string(),
                        );
                        Ok(())
                    }
                    Err(error) => {
                        native::editor_render_failed(
                            &session,
                            revision,
                            excluded,
                            &error.to_string(),
                        );
                        Ok(())
                    }
                })
                .await;
            }
        });
        return;
    }
    if let Action::Inspect(point, revision) = action {
        let mut queue = controls.lock().unwrap();
        queue.pending = Some((point, revision));
        if queue.running {
            return;
        }
        queue.running = true;
        drop(queue);
        crate::infrastructure::runtime::spawn(async move {
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
                let Ok(runtime) = services.runtime() else {
                    controls.lock().unwrap().running = false;
                    return;
                };
                let result = runtime
                    .control_candidate_at(&id, &request.0)
                    .await
                    .map(|candidate| candidate.map(|candidate| candidate.rect))
                    .map_err(|error| error.to_string());
                let id = id.0.clone();
                let _ = ui_thread::run("update capture control candidate", move || {
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
        Action::Complete(_, _, _)
            | Action::Cancel
            | Action::Refresh
            | Action::Editor(super::canvas::Request::Output { .. })
    );
    let refreshing = matches!(
        action,
        Action::Refresh
            | Action::Editor(super::canvas::Request::Control(
                snaplingo_core::application::capture::CaptureEditorControlAction::Command(
                    "refresh"
                )
            ))
    );
    crate::infrastructure::runtime::spawn(async move {
        if let Err(error) = run_action(&services, &id, mode, action).await {
            let error = error.to_string();
            let _ = ui_thread::run("report capture action error", move || {
                if refreshing
                    && native::session_id().is_none()
                    && !super::presentation::is_capture_presentation_active()
                {
                    crate::infrastructure::system::dialogs::show_native_error("截图失败", &error);
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
    services: &Services,
    id: &CaptureSessionId,
    mode: CaptureSelectionMode,
    action: Action,
) -> Result<()> {
    let runtime = services.runtime()?;
    match action {
        Action::FocusLost => {
            let session = id.0.clone();
            let owned = ui_thread::run(
                "check native editor focus after AppKit transition",
                move || {
                    Ok(!native::is_current(&session)
                        || native::editor_is_busy(&session)
                        || super::toolbar::owns_focus(&session))
                },
            )
            .await?;
            if !owned {
                let session = id.0.clone();
                ui_thread::run("close unfocused native editor", move || {
                    native::close_session(&session);
                    Ok(())
                })
                .await?;
                runtime.cancel_capture_session(id).await?;
            }
            Ok(())
        }
        Action::Cancel => {
            let close_id = id.0.clone();
            ui_thread::run("cancel native capture", move || {
                native::close_session(&close_id);
                Ok(())
            })
            .await?;
            runtime.cancel_capture_session(id).await
        }
        Action::Refresh => {
            let check_id = id.0.clone();
            if !ui_thread::run("check capture refresh", move || {
                Ok(native::is_current(&check_id))
            })
            .await?
            {
                return Ok(());
            }
            let close_id = id.0.clone();
            ui_thread::run("refresh native capture", move || {
                native::close_session(&close_id);
                Ok(())
            })
            .await?;
            runtime.cancel_capture_session(id).await?;
            runtime.open_capture_window_for_mode(mode.as_str()).await
        }
        Action::MoveCursor(dx, dy) => {
            services.cursor.move_relative(dx, dy);
            Ok(())
        }
        Action::CopyText(text) => {
            crate::infrastructure::system::clipboard_text::copy_text(text).map_err(Into::into)
        }
        Action::Complete(operation, rect, include_cursor) => {
            let check_id = id.0.clone();
            if !ui_thread::run("check native capture session", move || {
                Ok(native::is_current(&check_id))
            })
            .await?
            {
                return Ok(());
            }
            if operation == Operation::Edit {
                let settings = services.settings.snapshot()?;
                let mut styling = CaptureEditorStylePresentation::default();
                styling.style.stroke_width = settings.screenshot.default_stroke_width as u32;
                styling.font_size = settings.screenshot.default_font_size as u32;
                styling.color_presets = settings.screenshot.annotation_colors;
                let tool = if settings.screenshot.remember_last_tool {
                    AnnotationTool::parse(&runtime.last_annotation_tool())
                } else {
                    Some(AnnotationTool::Rectangle)
                };
                let model = CaptureEditor::new(rect.clone(), tool, styling);
                let pixels = runtime.render_rgba(id, &rect, &[], include_cursor)?;
                let theme = settings.general.theme;
                let interface_theme = settings.general.interface_theme;
                let session = id.0.clone();
                return ui_thread::run("open native annotation editor", move || {
                    let dark = crate::infrastructure::system::native_theme::is_dark(&theme);
                    native::begin_editor(&session, model, pixels, dark, interface_theme)
                })
                .await;
            }
            let screenshot = services.settings.snapshot()?.screenshot;
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
                    .prepare_capture_ocr(id, &rect, &[], target, services.settings.snapshot()?.ocr)
                    .await?;
                return runtime.complete_capture_ocr(id).await;
            }
            let output = match operation {
                Operation::Edit => unreachable!(),
                Operation::Primary | Operation::Copy => CaptureOutputAction::Copy,
                Operation::Pin => CaptureOutputAction::Pin,
                Operation::QuickSave => {
                    let path = services.output.quick_capture_save_path(
                        Some(&screenshot.save_path),
                        &screenshot.format,
                        &screenshot.naming_rule,
                        &screenshot.custom_file_name,
                    );
                    save_action(path, &screenshot, false)
                }
                Operation::Save => {
                    let suspend_id = id.0.clone();
                    ui_thread::run("show capture save dialog", move || {
                        native::suspend(&suspend_id);
                        Ok(())
                    })
                    .await?;
                    let Some(path) =
                        crate::infrastructure::system::image_save_dialog::select_native_capture_save_path(
                            &services.settings, &services.output,
                        )
                        .await?
                    else {
                        let resume_id = id.0.clone();
                        return ui_thread::run("return to capture selection", move || {
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
                    ui_thread::run("print capture", move || native::print(&print_id, png)).await?;
                    let close_id = id.0.clone();
                    ui_thread::run("finish capture print", move || {
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
        Action::Editor(request) => editor_request(services, id, mode, request).await,
    }
}

async fn editor_request(
    services: &Services,
    id: &CaptureSessionId,
    mode: CaptureSelectionMode,
    request: super::canvas::Request,
) -> Result<()> {
    use super::canvas::Request;
    use snaplingo_core::application::capture::{
        CaptureEditorControlAction as Control, CaptureEditorStyleChange as Style,
    };
    let runtime = services.runtime()?;
    match request {
        Request::SelectionInput { .. } | Request::SelectionNudge(_, _) => {
            unreachable!("Selection input is handled on the native UI thread")
        }
        Request::Render { .. } => unreachable!("Preview rendering uses the coalesced render queue"),
        Request::Fonts => {
            let families = runtime.annotation_font_families();
            let session = id.0.clone();
            ui_thread::run("load native annotation fonts", move || {
                native::editor_fonts(&session, families);
                Ok(())
            })
            .await
        }
        Request::Control(Control::Command("cancel" | "refresh")) => {
            let refresh = matches!(request, Request::Control(Control::Command("refresh")));
            let session = id.0.clone();
            ui_thread::run("close native annotation editor", move || {
                native::close_session(&session);
                Ok(())
            })
            .await?;
            runtime.cancel_capture_session(id).await?;
            if refresh {
                runtime.open_capture_window_for_mode(mode.as_str()).await?;
            }
            Ok(())
        }
        Request::Control(Control::Command(command)) => {
            if AnnotationTool::parse(command).is_some()
                && services.settings.snapshot()?.screenshot.remember_last_tool
            {
                runtime.remember_annotation_tool(command.into())?;
            }
            Ok(())
        }
        Request::Control(Control::Style { change, commit }) => {
            if !commit {
                return Ok(());
            }
            let mut settings = services.settings.snapshot()?.screenshot;
            match change {
                Style::StrokeWidth(value) => settings.default_stroke_width = value as u8,
                Style::FontSize(value) => settings.default_font_size = value as u8,
                _ => return Ok(()),
            }
            services.settings.update_screenshot(settings)?;
            Ok(())
        }
        Request::Control(Control::Presets(colors)) => {
            let saved = services.settings.update_annotation_colors(colors)?;
            let session = id.0.clone();
            ui_thread::run("confirm native annotation presets", move || {
                native::editor_presets(&session, saved.screenshot.annotation_colors);
                Ok(())
            })
            .await
        }
        Request::Output {
            command,
            rect,
            annotations,
            include_cursor,
        } => {
            let screenshot = services.settings.snapshot()?.screenshot;
            let action = match command {
                "copy" => CaptureOutputAction::Copy,
                "pin" => CaptureOutputAction::Pin,
                "quick-save" => save_action(
                    services.output.quick_capture_save_path(
                        Some(&screenshot.save_path),
                        &screenshot.format,
                        &screenshot.naming_rule,
                        &screenshot.custom_file_name,
                    ),
                    &screenshot,
                    false,
                ),
                "save" => {
                    let session = id.0.clone();
                    ui_thread::run("show native editor save dialog", move || {
                        native::suspend(&session);
                        Ok(())
                    })
                    .await?;
                    let Some(path) =
                        crate::infrastructure::system::image_save_dialog::select_native_capture_save_path(
                            &services.settings, &services.output,
                        )
                        .await?
                    else {
                        let session = id.0.clone();
                        return ui_thread::run("resume native editor", move || {
                            native::action_finished(&session, None);
                            Ok(())
                        })
                        .await;
                    };
                    save_action(path, &screenshot, true)
                }
                "ocr" => {
                    runtime
                        .prepare_capture_ocr(
                            id,
                            &rect,
                            &annotations,
                            CaptureOcrTarget::OcrWindow,
                            services.settings.snapshot()?.ocr,
                        )
                        .await?;
                    return runtime.complete_capture_ocr(id).await;
                }
                "scrolling" => {
                    runtime.begin_scrolling_capture(id.clone(), rect).await?;
                    return Ok(());
                }
                "favorite" => {
                    services
                        .favorites
                        .upgrade()
                        .ok_or("Screenshot favorite runtime has stopped")?
                        .add_selection(id, &rect, &annotations, include_cursor)
                        .await?;
                    runtime.remember_capture_selection(&rect)?;
                    let session = id.0.clone();
                    ui_thread::run("finish native editor favorite", move || {
                        native::close_session(&session);
                        Ok(())
                    })
                    .await?;
                    return runtime.cancel_capture_session(id).await;
                }
                "print" => {
                    let png = runtime.render_png(id, &rect, &annotations, include_cursor)?;
                    let session = id.0.clone();
                    ui_thread::run("print native annotations", move || {
                        native::print(&session, png)
                    })
                    .await?;
                    let session = id.0.clone();
                    ui_thread::run("finish native editor print", move || {
                        native::close_session(&session);
                        Ok(())
                    })
                    .await?;
                    return runtime.cancel_capture_session(id).await;
                }
                _ => return Err("Unknown native editor command".into()),
            };
            runtime
                .complete_annotated_capture_selection(
                    id,
                    &rect,
                    &annotations,
                    include_cursor,
                    action,
                )
                .await
        }
    }
}

fn save_action(
    path: std::path::PathBuf,
    screenshot: &snaplingo_core::domain::config::ScreenshotSettings,
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

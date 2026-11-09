//! Native scrolling sampler and application actions on the shared Windows loop.
use super::scrolling_view_windows as view;
use crate::application::capture::{
    CaptureSessionSource, ScrollingCaptureGeometry, ScrollingCaptureHost, ScrollingWindowOptions,
};
use crate::domain::capture::{
    CaptureSessionId, MonitorSnapshot, ScrollingCaptureOutputAction, ScrollingCaptureStatus,
    ScrollingCaptureView, ScrollingShortcutAction,
};
use crate::infrastructure::ui_thread;
use crate::{CaptureOutput, CaptureSessionRuntime, SettingsConfiguration};
use snaplingo_ui::scrolling::ScrollingAction as Action;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, RwLock, Weak,
    },
};
#[derive(Clone)]
pub(super) struct Services {
    pub runtime: Weak<CaptureSessionRuntime>,
    pub settings: Arc<SettingsConfiguration>,
    pub output: Arc<CaptureOutput>,
    pub ended: Arc<dyn Fn(&CaptureSessionId, bool) + Send + Sync>,
}
#[derive(Default)]
pub(super) struct Session {
    started: AtomicBool,
    closed: AtomicBool,
    working: AtomicBool,
    paused: AtomicBool,
    task: Mutex<Option<tokio::task::JoinHandle<()>>>,
}
impl Session {
    fn stop(&self) {
        self.closed.store(true, Ordering::SeqCst);
        if let Some(task) = self.task.lock().unwrap().take() {
            task.abort();
        }
    }
    fn start(self: &Arc<Self>, services: Services, id: String) {
        if self.closed.load(Ordering::SeqCst) || self.started.swap(true, Ordering::SeqCst) {
            return;
        }
        let session = self.clone();
        let task = crate::infrastructure::runtime::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                if session.closed.load(Ordering::SeqCst) {
                    break;
                }
                if session.working.load(Ordering::SeqCst) || session.paused.load(Ordering::SeqCst) {
                    continue;
                }
                let Some(runtime) = services.runtime.upgrade() else {
                    break;
                };
                match runtime.capture_scrolling_frame(&id).await {
                    Ok(next) => {
                        session.paused.store(
                            next.status == ScrollingCaptureStatus::Error,
                            Ordering::SeqCst,
                        );
                        if next.status == ScrollingCaptureStatus::Completed {
                            break;
                        }
                    }
                    Err(error) => {
                        if session.closed.load(Ordering::SeqCst) {
                            break;
                        }
                        session.paused.store(true, Ordering::SeqCst);
                        let id = id.clone();
                        ui_thread::dispatch(move || {
                            view::set_working(&id, false, Some(error.to_string()))
                        });
                    }
                }
            }
        });
        *self.task.lock().unwrap() = Some(task);
    }
}
pub(super) fn dispatch(services: Services, id: String, session: Arc<Session>, action: Action) {
    if session.closed.load(Ordering::SeqCst) || session.working.swap(true, Ordering::SeqCst) {
        return;
    }
    let working_id = id.clone();
    ui_thread::dispatch(move || view::set_working(&working_id, true, None));
    crate::infrastructure::runtime::spawn(async move {
        let result = run_action(&services, &id, action).await;
        let next = if !session.closed.load(Ordering::SeqCst) {
            if let Some(runtime) = services.runtime.upgrade() {
                runtime.scrolling_capture_view(&id).await.ok()
            } else {
                None
            }
        } else {
            None
        };
        session.working.store(false, Ordering::SeqCst);
        session.paused.store(
            result.is_err()
                || next
                    .as_ref()
                    .is_some_and(|view| view.status == ScrollingCaptureStatus::Error),
            Ordering::SeqCst,
        );
        let error = result.err().map(|error| error.to_string());
        ui_thread::dispatch(move || {
            if let Some(next) = next {
                view::update(next);
            }
            view::set_working(&id, false, error);
        });
    });
}
async fn run_action(services: &Services, id: &str, action: Action) -> crate::Result<()> {
    let runtime = services
        .runtime
        .upgrade()
        .ok_or("Capture runtime has stopped")?;
    match action {
        Action::Undo => runtime.undo_scrolling_capture(id).await.map(|_| ()),
        Action::Resume => runtime.capture_scrolling_frame(id).await.map(|_| ()),
        Action::Preview => {
            let view = runtime.scrolling_capture_view(id).await?;
            runtime
                .set_scrolling_preview_visible(id, !view.preview_expanded)
                .await
                .map(|_| ())
        }
        Action::Cancel => runtime.cancel_scrolling_capture(id).await,
        Action::Copy => {
            runtime
                .finish_scrolling_capture(id, ScrollingCaptureOutputAction::Copy)
                .await
        }
        Action::Pin => {
            runtime
                .finish_scrolling_capture(id, ScrollingCaptureOutputAction::Pin)
                .await
        }
        Action::Complete => {
            runtime
                .finish_scrolling_capture(id, ScrollingCaptureOutputAction::Complete)
                .await
        }
        Action::Save => {
            let preferences = services.settings.snapshot()?.screenshot;
            let format = preferences
                .last_save_format
                .as_deref()
                .unwrap_or(&preferences.format);
            let path = services.output.dialog_capture_save_path(
                preferences
                    .last_save_directory
                    .as_deref()
                    .or(Some(&preferences.save_path)),
                Some(&preferences.save_path),
                format,
                &preferences.naming_rule,
                &preferences.custom_file_name,
            );
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let directory = path.parent().map(|p| p.to_path_buf());
            let future = ui_thread::run("save scrolling capture", move || {
                let mut dialog = rfd::AsyncFileDialog::new()
                    .set_title("保存滚动截图")
                    .set_file_name(name)
                    .add_filter("图片", &["png", "jpg", "webp"]);
                if let Some(directory) = directory {
                    dialog = dialog.set_directory(directory);
                }
                Ok(dialog.save_file())
            })
            .await?;
            let Some(path) = future.await else {
                return Ok(());
            };
            runtime
                .finish_scrolling_capture(
                    id,
                    ScrollingCaptureOutputAction::Save {
                        path: path.path().to_string_lossy().into_owned(),
                        format: preferences.format,
                        quality: preferences.quality,
                        copy_after_save: preferences.auto_copy,
                        remember_choice: true,
                    },
                )
                .await
        }
    }
}
pub(crate) struct NativeScrollingCaptureHost {
    source: Arc<dyn CaptureSessionSource>,
    services: RwLock<Option<Services>>,
    sessions: Mutex<HashMap<String, Arc<Session>>>,
    shortcuts: Mutex<HashMap<String, Vec<String>>>,
}
impl NativeScrollingCaptureHost {
    pub(crate) fn new(source: Arc<dyn CaptureSessionSource>) -> Self {
        Self {
            source,
            services: RwLock::new(None),
            sessions: Mutex::new(HashMap::new()),
            shortcuts: Mutex::new(HashMap::new()),
        }
    }
    pub(crate) fn bind(
        &self,
        state: &crate::AppState,
        ended: impl Fn(&CaptureSessionId, bool) + Send + Sync + 'static,
    ) {
        *self.services.write().unwrap() = Some(Services {
            runtime: Arc::downgrade(&state.capture.runtime),
            settings: state.settings.configuration.clone(),
            output: state.capture.output.clone(),
            ended: Arc::new(ended),
        });
    }
    fn services(&self) -> crate::Result<Services> {
        self.services
            .read()
            .unwrap()
            .clone()
            .ok_or_else(|| "Native scrolling services are not initialized".into())
    }
}
#[async_trait::async_trait]
impl ScrollingCaptureHost for NativeScrollingCaptureHost {
    async fn open(
        &self,
        initial: &ScrollingCaptureView,
        geometry: &ScrollingCaptureGeometry,
    ) -> crate::Result<ScrollingWindowOptions> {
        let services = self.services()?;
        let session = Arc::new(Session::default());
        self.sessions
            .lock()
            .unwrap()
            .insert(initial.id.clone(), session.clone());
        let general = services.settings.snapshot()?.general;
        let initial_view = initial.clone();
        let geometry = geometry.clone();
        let actions = services.clone();
        let action_session = session.clone();
        let stop_session = session.clone();
        let action_id = initial.id.clone();
        let preview_expanded =
            ui_thread::run_with_event_loop("open scrolling capture", move |event_loop| {
                super::capture_window::yield_capture_window_focus();
                view::open(
                    event_loop,
                    initial_view,
                    geometry,
                    general,
                    move |action| {
                        dispatch(
                            actions.clone(),
                            action_id.clone(),
                            action_session.clone(),
                            action,
                        )
                    },
                    move || stop_session.stop(),
                )
            })
            .await?;
        let mut installed = vec![];
        let mut shortcuts = vec![];
        for (key, intent, action) in [
            ("Enter", ScrollingShortcutAction::Complete, Action::Complete),
            ("Escape", ScrollingShortcutAction::Cancel, Action::Cancel),
            ("Ctrl+Z", ScrollingShortcutAction::Undo, Action::Undo),
        ] {
            if !matches!(super::shortcut_native::is_registered(key), Ok(false)) {
                continue;
            }
            let services = services.clone();
            let session = session.clone();
            let id = initial.id.clone();
            match super::shortcut_native::register(
                key,
                super::shortcut_native::ShortcutTriggerTiming::Released,
                move || dispatch(services.clone(), id.clone(), session.clone(), action),
            ) {
                Ok(()) => {
                    installed.push(key.to_owned());
                    shortcuts.push(intent);
                }
                Err(error) => log::debug!("Scrolling key {key} unavailable: {error}"),
            }
        }
        self.shortcuts
            .lock()
            .unwrap()
            .insert(initial.id.clone(), installed);
        Ok(ScrollingWindowOptions {
            shortcuts,
            preview_expanded,
        })
    }
    async fn capture_snapshots(
        &self,
        id: &str,
        geometry: &ScrollingCaptureGeometry,
    ) -> crate::Result<Vec<MonitorSnapshot>> {
        let hide_id = id.to_owned();
        let viewport = geometry.viewport.clone();
        let restore_id = id.to_owned();
        let result = super::scrolling_sampling::sample(
            ui_thread::run("hide scrolling overlays", move || {
                Ok(view::begin_sampling(&hide_id, &viewport))
            }),
            async {
                let mut snapshots = Vec::new();
                for monitor in &geometry.monitors {
                    snapshots.push(self.source.capture_monitor_snapshot(&monitor.id).await?);
                }
                Ok(snapshots)
            },
            move || ui_thread::dispatch(move || view::end_sampling(&restore_id)),
        )
        .await;
        // The queued restore runs before this barrier, including source errors.
        ui_thread::run("restore scrolling overlays", || Ok(())).await?;
        result
    }

    fn publish(&self, next: &ScrollingCaptureView) {
        if let Some(session) = self.sessions.lock().unwrap().get(&next.id).cloned() {
            if let Ok(services) = self.services() {
                session.start(services, next.id.clone());
            }
        }
        let next = next.clone();
        ui_thread::dispatch(move || view::update(next));
    }
    async fn set_preview_visible(&self, id: &str, visible: bool) -> crate::Result<bool> {
        let id = id.to_owned();
        ui_thread::run("toggle scrolling preview", move || {
            Ok(view::set_preview_visible(&id, visible))
        })
        .await
    }
    async fn close(
        &self,
        id: &str,
        session_id: &CaptureSessionId,
        completed: Option<bool>,
    ) -> crate::Result<()> {
        if let Some(session) = self.sessions.lock().unwrap().remove(id) {
            session.stop();
        }
        let keys = self
            .shortcuts
            .lock()
            .unwrap()
            .remove(id)
            .unwrap_or_default();
        for key in keys {
            let _ = super::shortcut_native::unregister(&key);
        }
        if let Some(completed) = completed {
            (self.services()?.ended)(session_id, completed);
        }
        let id = id.to_owned();
        ui_thread::run("close scrolling capture", move || {
            view::close(&id);
            Ok(())
        })
        .await
    }
}
pub(crate) fn settings_changed(general: crate::domain::GeneralSettings) {
    ui_thread::dispatch(move || view::settings_changed(general));
}
pub(crate) use view::{
    accessibility_event as windows_accessibility, close_all as windows_close_all,
    window_event as windows_window_event,
};

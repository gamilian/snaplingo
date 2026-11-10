use super::super::session::{dispatch, Services, Session};
use crate::features::capture as capture_window;
use snaplingo_platform::hotkeys as shortcut_native;
// Native scrolling sampler and application actions on the shared Windows loop.
use super::scrolling_view_windows as view;
use crate::integrations::ui_thread;
use snaplingo_core::capture::model::CaptureSessionId;
use snaplingo_core::capture::model::MonitorSnapshot;
use snaplingo_core::capture::model::ScrollingCaptureView;
use snaplingo_core::capture::model::ScrollingShortcutAction;
use snaplingo_core::capture::CaptureSessionSource;
use snaplingo_core::capture::ScrollingCaptureGeometry;
use snaplingo_core::capture::ScrollingCaptureHost;
use snaplingo_core::capture::ScrollingWindowOptions;
use snaplingo_ui::scrolling::ScrollingAction as Action;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, RwLock},
};
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
    pub(crate) fn bind(&self, services: Services) {
        *self.services.write().unwrap() = Some(services);
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
                capture_window::yield_capture_window_focus();
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
            if !matches!(shortcut_native::is_registered(key), Ok(false)) {
                continue;
            }
            let services = services.clone();
            let session = session.clone();
            let id = initial.id.clone();
            match shortcut_native::register(
                key,
                shortcut_native::ShortcutTriggerTiming::Released,
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
        let result = super::super::sampling::sample(
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
            let _ = shortcut_native::unregister(&key);
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
pub(crate) fn settings_changed(general: snaplingo_core::settings::model::GeneralSettings) {
    ui_thread::dispatch(move || view::settings_changed(general));
}
pub(crate) use view::{
    accessibility_event as windows_accessibility, close_all as windows_close_all,
    window_event as windows_window_event,
};

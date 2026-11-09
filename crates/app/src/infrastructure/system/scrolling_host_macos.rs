use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

use crate::application::capture::{
    CaptureSessionSource, ScrollingCaptureGeometry, ScrollingCaptureHost, ScrollingWindowOptions,
};
use crate::domain::capture::{
    CaptureSessionId, LogicalRect, MonitorSnapshot, ScrollingCaptureView, ScrollingShortcutAction,
};
use crate::infrastructure::ui_thread;

use super::scrolling_geometry::{outside_position, preview_placement};
use super::scrolling_native_macos::{self as actions, Services, Session};

pub(crate) fn settings_changed(general: crate::domain::config::GeneralSettings) {
    ui_thread::dispatch(move || super::scrolling_controls_macos::update_theme(general));
}

pub(crate) struct NativeScrollingCaptureHost {
    source: Arc<dyn CaptureSessionSource>,
    services: RwLock<Option<Services>>,
    shortcuts: Mutex<HashMap<String, Vec<String>>>,
    overlays: Arc<super::scrolling_overlays_macos::ScrollingOverlays>,
    preview_cache: Mutex<HashMap<String, String>>,
    sessions: Mutex<HashMap<String, Arc<Session>>>,
}

impl NativeScrollingCaptureHost {
    pub(crate) fn new(source: Arc<dyn CaptureSessionSource>) -> Self {
        Self {
            source,
            services: RwLock::new(None),
            shortcuts: Mutex::new(HashMap::new()),
            overlays: Arc::new(super::scrolling_overlays_macos::ScrollingOverlays),
            preview_cache: Mutex::new(HashMap::new()),
            sessions: Mutex::new(HashMap::new()),
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
        view: &ScrollingCaptureView,
        geometry: &ScrollingCaptureGeometry,
    ) -> crate::Result<ScrollingWindowOptions> {
        let services = self.services()?;
        let session = Arc::new(Session::default());
        self.sessions
            .lock()
            .unwrap()
            .insert(view.id.clone(), session.clone());
        self.preview_cache
            .lock()
            .unwrap()
            .insert(view.id.clone(), view.preview_base64.clone());
        let general = services.settings.snapshot()?.general;
        let initial = view.clone();
        let geometry = geometry.clone();
        let overlays = self.overlays.clone();
        let event_session = session.clone();
        let event_services = services.clone();
        let preview_expanded = ui_thread::run("open scrolling capture", move || {
            super::capture_window::yield_capture_window_focus();
            let width = 408.0 * initial.ui_scale;
            let height = 132.0 * initial.ui_scale;
            let (x, y) = outside_position(&geometry, width, height).unwrap_or_else(|| {
                let screen = geometry.displays.first().unwrap_or(&geometry.viewport);
                (screen.x + screen.width - width - 12.0, screen.y + 12.0)
            });
            let panel = LogicalRect {
                x,
                y,
                width,
                height,
            };
            let id = initial.id.clone();
            let event_id = id.clone();
            super::scrolling_controls_macos::open(
                initial.clone(),
                &panel,
                general,
                move |action| {
                    actions::dispatch(
                        event_services.clone(),
                        event_id.clone(),
                        event_session.clone(),
                        action,
                    );
                },
            )?;
            overlays.open(&id, &geometry.viewport, initial.ui_scale)?;
            if let Some(rect) = preview_placement(&geometry, initial.ui_scale, Some(&panel)) {
                overlays.open_preview(&id, &rect)?;
                overlays.update_preview(&id, &initial.preview_base64);
                Ok(true)
            } else {
                Ok(false)
            }
        })
        .await?;

        let mut installed = Vec::new();
        let mut shortcuts = Vec::new();
        for (key, intent) in [
            ("Enter", ScrollingShortcutAction::Complete),
            ("Escape", ScrollingShortcutAction::Cancel),
            ("Super+Z", ScrollingShortcutAction::Undo),
        ] {
            match super::shortcut_native::is_registered(key) {
                Ok(false) => {}
                Ok(true) => continue,
                Err(error) => {
                    log::debug!("Scrolling capture key {key} is unavailable: {error}");
                    continue;
                }
            }
            let id = view.id.clone();
            let session = session.clone();
            let services = services.clone();
            let action = match intent {
                ScrollingShortcutAction::Complete => {
                    super::scrolling_controls_macos::Action::Complete
                }
                ScrollingShortcutAction::Cancel => super::scrolling_controls_macos::Action::Cancel,
                ScrollingShortcutAction::Undo => super::scrolling_controls_macos::Action::Undo,
                ScrollingShortcutAction::Copy => unreachable!(),
            };
            match super::shortcut_native::register(
                key,
                super::shortcut_native::ShortcutTriggerTiming::Released,
                move || {
                    actions::dispatch(services.clone(), id.clone(), session.clone(), action);
                },
            ) {
                Ok(()) => {
                    installed.push(key.to_owned());
                    shortcuts.push(intent);
                }
                Err(error) => log::debug!("Scrolling capture key {key} is unavailable: {error}"),
            }
        }
        self.shortcuts
            .lock()
            .unwrap()
            .insert(view.id.clone(), installed);
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
        let hidden_id = id.to_owned();
        let viewport = geometry.viewport.clone();
        let overlays = self.overlays.clone();
        let hidden = ui_thread::run("hide scrolling capture overlays", move || {
            Ok((
                overlays.hide_if_overlapping(&hidden_id, &viewport),
                overlays.hide_preview_if_overlapping(&hidden_id, &viewport),
                super::scrolling_controls_macos::hide_if_overlapping(&hidden_id, &viewport),
            ))
        })
        .await?;
        if hidden.0 || hidden.1 || hidden.2 {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        let result = async {
            let mut snapshots = Vec::new();
            for monitor in &geometry.monitors {
                snapshots.push(self.source.capture_monitor_snapshot(&monitor.id).await?);
            }
            Ok(snapshots)
        }
        .await;
        if hidden.0 || hidden.1 || hidden.2 {
            let id = id.to_owned();
            let overlays = self.overlays.clone();
            ui_thread::run("restore scrolling capture overlays", move || {
                if hidden.0 {
                    overlays.show(&id);
                }
                if hidden.1 {
                    overlays.set_preview_visible(&id, true);
                }
                if hidden.2 {
                    super::scrolling_controls_macos::show(&id);
                }
                Ok(())
            })
            .await?;
        }
        result
    }

    fn publish(&self, view: &ScrollingCaptureView) {
        if !view.preview_base64.is_empty()
            && self
                .preview_cache
                .lock()
                .unwrap()
                .insert(view.id.clone(), view.preview_base64.clone())
                .as_deref()
                != Some(view.preview_base64.as_str())
        {
            let overlays = self.overlays.clone();
            let id = view.id.clone();
            let preview = view.preview_base64.clone();
            ui_thread::dispatch(move || overlays.update_preview(&id, &preview));
        }
        if let Some(session) = self.sessions.lock().unwrap().get(&view.id).cloned() {
            if let Ok(services) = self.services() {
                session.start(services, view.id.clone());
            }
        }
        let view = view.clone();
        ui_thread::dispatch(move || super::scrolling_controls_macos::update(view));
    }

    async fn set_preview_visible(&self, id: &str, visible: bool) -> crate::Result<bool> {
        let id = id.to_owned();
        let overlays = self.overlays.clone();
        ui_thread::run("toggle scrolling capture preview", move || {
            Ok(overlays.set_preview_visible(&id, visible))
        })
        .await
    }

    async fn close(
        &self,
        id: &str,
        session_id: &CaptureSessionId,
        completed: Option<bool>,
    ) -> crate::Result<()> {
        self.preview_cache.lock().unwrap().remove(id);
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
            if let Err(error) = super::shortcut_native::unregister(&key) {
                log::debug!("Could not release scrolling capture key {key}: {error}");
            }
        }
        if let Some(completed) = completed {
            (self.services()?.ended)(session_id, completed);
        }
        let id = id.to_owned();
        let overlays = self.overlays.clone();
        ui_thread::run("close scrolling capture", move || {
            super::scrolling_controls_macos::close(&id);
            overlays.close(&id);
            overlays.close_preview(&id);
            Ok(())
        })
        .await
    }
}

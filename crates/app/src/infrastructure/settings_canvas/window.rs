//! Application adapter for the shared seven-page Settings window.
use super::{library::LibrarySession, preferences::PreferencesSession, services::ServicesSession};
use crate::application::hotkeys::HotkeyRecording;
use crate::infrastructure::desktop_events::{DesktopEvent, DesktopSubscription};
use snaplingo_ui::{
    library as library_ui, services as services_ui, Action, Page, Size, WindowIntent,
    WindowSnapshot,
};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex,
};

#[derive(Default)]
struct Changes {
    notifier: Mutex<Option<Arc<dyn Fn() + Send + Sync>>>,
    error: Mutex<Option<String>>,
    reload: AtomicBool,
    generation: AtomicU64,
}
impl Changes {
    fn notify(&self) {
        let callback = self.notifier.lock().unwrap().clone();
        if let Some(callback) = callback {
            callback();
        }
    }
    fn spawn(
        self: &Arc<Self>,
        future: impl std::future::Future<Output = Result<(), String>> + Send + 'static,
    ) {
        let changes = self.clone();
        let generation = self.generation.load(Ordering::Acquire);
        crate::infrastructure::runtime::spawn(async move {
            let result = future.await;
            if changes.generation.load(Ordering::Acquire) == generation {
                *changes.error.lock().unwrap() = result.err();
                changes.notify();
            }
        });
    }
}
pub(crate) struct SettingsWindowSession {
    state: Arc<crate::AppState>,
    preferences: PreferencesSession,
    page: Page,
    services: Option<Arc<ServicesSession>>,
    library: Option<Arc<LibrarySession>>,
    capacity: Arc<dyn Fn() -> crate::Result<()> + Send + Sync>,
    changes: Arc<Changes>,
    _subscription: DesktopSubscription,
    system_dark: bool,
}
impl SettingsWindowSession {
    pub(crate) fn new(
        state: Arc<crate::AppState>,
        recording: HotkeyRecording,
        notices: Arc<dyn Fn() -> crate::Result<()> + Send + Sync>,
        capacity: Arc<dyn Fn() -> crate::Result<()> + Send + Sync>,
    ) -> Self {
        let changes = Arc::new(Changes::default());
        let events = changes.clone();
        let subscription = state.desktop_events.subscribe(move |event| {
            if matches!(
                event,
                DesktopEvent::HistoryChanged
                    | DesktopEvent::FavoritesChanged
                    | DesktopEvent::ScreenshotFavoritesChanged
            ) {
                events.reload.store(true, Ordering::Release);
            }
            if matches!(
                event,
                DesktopEvent::SettingsChanged
                    | DesktopEvent::HotkeysChanged
                    | DesktopEvent::ProvidersChanged
                    | DesktopEvent::HistoryChanged
                    | DesktopEvent::FavoritesChanged
                    | DesktopEvent::ScreenshotFavoritesChanged
            ) {
                events.notify();
            }
        });
        Self {
            preferences: PreferencesSession::new(state.clone(), recording, notices),
            state,
            page: Page::General,
            services: None,
            library: None,
            capacity,
            changes,
            _subscription: subscription,
            system_dark: false,
        }
    }
    pub(crate) fn set_notifier(&self, callback: Option<Arc<dyn Fn() + Send + Sync>>) {
        *self.changes.notifier.lock().unwrap() = callback;
    }
    pub(crate) fn set_system_dark(&mut self, dark: bool) {
        self.system_dark = dark;
    }
    pub(crate) fn snapshot(&self) -> Result<WindowSnapshot, String> {
        let mut shell = self.preferences.snapshot()?;
        if shell.general.appearance == "system" {
            shell.dark = self.system_dark;
        }
        if let Some(error) = self.changes.error.lock().unwrap().clone() {
            shell.error = Some(error);
        }
        if let Some(services) = &self.services {
            let error = shell.error.clone();
            return services.snapshot(shell).map(|mut snapshot| {
                if error.is_some() {
                    snapshot.error = error;
                }
                WindowSnapshot::Services(snapshot)
            });
        }
        if let Some(library) = &self.library {
            if self.changes.reload.swap(false, Ordering::AcqRel) {
                self.reload_library(library);
            }
            let error = shell.error.clone();
            let mut snapshot = library.snapshot(shell);
            if error.is_some() {
                snapshot.error = error;
            }
            return Ok(WindowSnapshot::Library(snapshot));
        }
        Ok(WindowSnapshot::Settings(shell))
    }
    pub(crate) fn interface_scale(&self, display: Size) -> f32 {
        self.preferences.interface_scale(display)
    }
    pub(crate) fn navigate(&mut self, page: Page) -> Result<(), String> {
        self.preferences
            .dispatch(self.page, Action::Navigate(page))?;
        if page != self.page {
            self.changes.generation.fetch_add(1, Ordering::AcqRel);
            if let Some(old) = self.services.take() {
                old.dispose();
            }
            if let Some(old) = self.library.take() {
                old.dispose();
            }
            self.page = page;
            *self.changes.error.lock().unwrap() = None;
            if page == Page::Services {
                let services =
                    Arc::new(ServicesSession::from_state(&self.state, Default::default()));
                self.services = Some(services);
                self.service(services_ui::ServicesIntent::Refresh)?;
            } else if matches!(page, Page::History | Page::Favorites) {
                let kind = if page == Page::History {
                    library_ui::LibraryKind::History
                } else {
                    library_ui::LibraryKind::Favorites
                };
                let library = Arc::new(LibrarySession::from_state(&self.state, kind));
                self.reload_library(&library);
                self.library = Some(library);
            }
        }
        if page == Page::General {
            let controller = self.preferences.general.clone();
            let changes = self.changes.clone();
            self.changes.spawn(async move {
                controller
                    .ensure_checked(move |_| changes.notify())
                    .await
                    .map(|_| ())
            });
        }
        Ok(())
    }
    fn reload_library(&self, library: &Arc<LibrarySession>) {
        let snapshot = library.snapshot(Default::default());
        let changes = self.changes.clone();
        let future = library.dispatch(
            library_ui::LibraryIntent::Query {
                revision: snapshot.revision,
                kind: snapshot.kind,
                query: snapshot.query,
            },
            move || changes.notify(),
        );
        self.changes.spawn(future);
    }
    pub(crate) fn dispatch(&mut self, page: Page, intent: WindowIntent) -> Result<(), String> {
        *self.changes.error.lock().unwrap() = None;
        match intent {
            WindowIntent::Settings(Action::Navigate(page))
            | WindowIntent::Services(services_ui::ServicesIntent::Navigate(page)) => {
                self.navigate(page)
            }
            WindowIntent::Settings(Action::ConfigureCapacity)
            | WindowIntent::Services(services_ui::ServicesIntent::ConfigureCapacity) => {
                (self.capacity)().map_err(|e| e.to_string())
            }
            WindowIntent::Settings(Action::GeneralCommand(command)) => {
                let controller = self.preferences.general.clone();
                let changes = self.changes.clone();
                self.changes.spawn(async move {
                    controller
                        .perform(super::general::action(command), move |_| changes.notify())
                        .await
                        .map(|_| ())
                });
                Ok(())
            }
            WindowIntent::Settings(action) => self.preferences.dispatch(page, action),
            WindowIntent::Services(intent) => self.service(intent),
            WindowIntent::Library(intent) => {
                let library = self.library.as_ref().ok_or("Library page is not active")?;
                let changes = self.changes.clone();
                let future = library.dispatch(intent, move || changes.notify());
                self.changes.spawn(future);
                Ok(())
            }
        }
    }
    fn service(&self, intent: services_ui::ServicesIntent) -> Result<(), String> {
        if let services_ui::ServicesIntent::OpenUrl(url) = intent {
            return crate::infrastructure::system::external_open::open_url(&url)
                .map_err(|e| e.to_string());
        }
        let session = self.services.clone().ok_or("Services page is not active")?;
        let route = matches!(intent, services_ui::ServicesIntent::Route(_));
        if session.edit(&intent)? {
            if route {
                return self.service(services_ui::ServicesIntent::Refresh);
            }
            return Ok(());
        }
        let changes = self.changes.clone();
        self.changes.spawn(async move {
            if let services_ui::ServicesIntent::ConfirmRemove(id) = intent {
                return session.remove_confirmed(id, move || changes.notify()).await;
            }
            let effects = session.dispatch(intent, move || changes.notify()).await?;
            if effects.is_empty() {
                Ok(())
            } else {
                Err("Window action was not handled by the native host".into())
            }
        });
        Ok(())
    }
}
impl Drop for SettingsWindowSession {
    fn drop(&mut self) {
        self.changes.generation.fetch_add(1, Ordering::AcqRel);
        self.set_notifier(None);
        if let Some(session) = &self.services {
            session.dispose();
        }
        if let Some(session) = &self.library {
            session.dispose();
        }
    }
}

//! Application adapter for the shared seven-page Settings window.
use super::{library::LibrarySession, preferences::PreferencesSession, services::ServicesSession};
use snaplingo_core::application::hotkeys::HotkeyRecording;
use snaplingo_ui::{
    library as library_ui, services as services_ui, Action, Page, Size, WindowIntent,
    WindowSnapshot,
};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex,
};

struct Changes {
    notifier: Mutex<Option<Arc<dyn Fn() + Send + Sync>>>,
    error: Mutex<Option<String>>,
    reload: AtomicBool,
    generation: AtomicU64,
    executor: tokio::runtime::Handle,
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
        self.executor.spawn(async move {
            let result = future.await;
            if changes.generation.load(Ordering::Acquire) == generation {
                *changes.error.lock().unwrap() = result.err();
                changes.notify();
            }
        });
    }
}
pub struct SettingsSessionDependencies {
    pub preferences: super::preferences::PreferencesDependencies,
    pub services: Arc<dyn Fn() -> ServicesSession + Send + Sync>,
    pub library: Arc<dyn Fn(library_ui::LibraryKind) -> LibrarySession + Send + Sync>,
    pub capacity: Arc<dyn Fn() -> Result<(), String> + Send + Sync>,
    pub open_url: Arc<dyn Fn(&str) -> Result<(), String> + Send + Sync>,
}

pub struct SettingsWindowSession {
    dependencies: SettingsSessionDependencies,
    preferences: PreferencesSession,
    page: Page,
    services: Option<Arc<ServicesSession>>,
    library: Option<Arc<LibrarySession>>,
    changes: Arc<Changes>,
    system_dark: bool,
}
impl SettingsWindowSession {
    pub fn new(
        dependencies: SettingsSessionDependencies,
        recording: HotkeyRecording,
        executor: tokio::runtime::Handle,
    ) -> Self {
        let changes = Arc::new(Changes {
            executor: executor.clone(),
            notifier: Default::default(),
            error: Default::default(),
            reload: AtomicBool::new(false),
            generation: AtomicU64::new(0),
        });
        let notify = changes.clone();
        let preferences = PreferencesSession::new(
            dependencies.preferences.clone(),
            recording,
            executor,
            Arc::new(move || notify.notify()),
        );
        Self {
            dependencies,
            preferences,
            page: Page::General,
            services: None,
            library: None,
            changes,
            system_dark: false,
        }
    }
    /// Safe to call from synchronous workflow notifications, including dispatch.
    pub fn invalidator(&self) -> Arc<dyn Fn(bool) + Send + Sync> {
        let changes = self.changes.clone();
        let usage = self.dependencies.preferences.usage.clone();
        Arc::new(move |library_changed| {
            if library_changed {
                changes.reload.store(true, Ordering::Release);
                usage.refresh();
            }
            changes.notify();
        })
    }
    pub fn page(&self) -> Page {
        self.page
    }
    pub fn saving_hotkeys(&self) -> bool {
        self.preferences.saving_hotkeys()
    }
    pub fn recording_id(&self) -> Option<String> {
        self.preferences.recording_id()
    }
    pub fn set_notifier(&self, callback: Option<Arc<dyn Fn() + Send + Sync>>) {
        *self.changes.notifier.lock().unwrap() = callback;
    }
    pub fn set_system_dark(&mut self, dark: bool) {
        self.system_dark = dark;
    }
    pub fn snapshot(&self) -> Result<WindowSnapshot, String> {
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
    pub fn interface_scale(&self, display: Size) -> f32 {
        self.preferences.interface_scale(display)
    }
    pub fn navigate(&mut self, page: Page) -> Result<(), String> {
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
                let services = Arc::new((self.dependencies.services)());
                self.services = Some(services);
                self.service(services_ui::ServicesIntent::Refresh)?;
            } else if matches!(page, Page::History | Page::Favorites) {
                let kind = if page == Page::History {
                    library_ui::LibraryKind::History
                } else {
                    library_ui::LibraryKind::Favorites
                };
                let library = Arc::new((self.dependencies.library)(kind));
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
    pub fn dispatch(&mut self, page: Page, intent: WindowIntent) -> Result<(), String> {
        *self.changes.error.lock().unwrap() = None;
        match intent {
            WindowIntent::Settings(Action::Navigate(page))
            | WindowIntent::Services(services_ui::ServicesIntent::Navigate(page)) => {
                self.navigate(page)
            }
            WindowIntent::Settings(Action::ConfigureCapacity)
            | WindowIntent::Services(services_ui::ServicesIntent::ConfigureCapacity) => {
                (self.dependencies.capacity)()
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
            return (self.dependencies.open_url)(&url);
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

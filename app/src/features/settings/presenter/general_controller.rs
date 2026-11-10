use std::sync::{Arc, Mutex};

use snaplingo_core::library::history::History;
use snaplingo_core::permissions::RequiredPermissions;
use snaplingo_core::permissions::RequiredPermissionsContext;
use snaplingo_core::permissions::RequiredPermissionsStatus;
use snaplingo_core::permissions::SystemPermission;
use snaplingo_core::settings::model::GeneralSettings;
use snaplingo_core::settings::AppLogEntry;
use snaplingo_core::settings::GeneralField;
use snaplingo_core::settings::GeneralValue;
use snaplingo_core::settings::SettingsApplication;
use snaplingo_core::settings::SettingsConfiguration;
use snaplingo_core::updates::AppUpdateStatus;
use snaplingo_core::updates::AppUpdates;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdatePhase {
    Idle,
    Checking,
    Downloading,
}

#[derive(Clone)]
pub struct GeneralProjection {
    pub general: GeneralSettings,
    pub logs: Vec<AppLogEntry>,
    pub permissions: Option<RequiredPermissionsStatus>,
    pub permission_context: Option<RequiredPermissionsContext>,
    pub permission_busy: bool,
    pub version: String,
    pub update: Option<AppUpdateStatus>,
    pub update_phase: UpdatePhase,
    pub downloaded_path: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone, Debug)]
pub enum GeneralAction {
    Edit(GeneralField, GeneralValue),
    RefreshLogs,
    ClearLogs,
    ClearHistory,
    OpenProject,
    OpenNotices,
    CheckUpdates,
    OpenReleases,
    DownloadUpdate,
    RefreshPermissions,
    RequestPermission(SystemPermission),
    ResetPermission(SystemPermission),
    Restart,
}

#[derive(Clone)]
pub struct GeneralServices {
    pub configuration: Arc<SettingsConfiguration>,
    pub settings: Arc<SettingsApplication>,
    pub history: Arc<History>,
    pub permissions: Arc<RequiredPermissions>,
    pub updates: Arc<AppUpdates>,
    pub version: String,
    pub open_url: Arc<dyn Fn(&str) -> snaplingo_core::Result<()> + Send + Sync>,
    pub open_notices: Arc<dyn Fn() -> snaplingo_core::Result<()> + Send + Sync>,
}

struct State {
    logs: Vec<AppLogEntry>,
    update: Option<AppUpdateStatus>,
    phase: UpdatePhase,
    downloaded_path: Option<String>,
    error: Option<String>,
    attempted_check: bool,
    update_generation: u64,
    permission_busy: bool,
}

struct PermissionOperation<'a>(&'a Mutex<State>);

impl Drop for PermissionOperation<'_> {
    fn drop(&mut self) {
        self.0.lock().unwrap().permission_busy = false;
    }
}

/// Owns General-page actions and update state. The shared Settings session owns confirmation and async dispatch; native
/// adapters display dialogs and deliver projections on their UI thread.
pub struct GeneralController {
    services: GeneralServices,
    state: Mutex<State>,
}

impl GeneralController {
    pub fn new(services: GeneralServices) -> Self {
        let logs = services.settings.list_logs(8).unwrap_or_default();
        Self {
            services,
            state: Mutex::new(State {
                logs,
                update: None,
                phase: UpdatePhase::Idle,
                downloaded_path: None,
                error: None,
                attempted_check: false,
                update_generation: 0,
                permission_busy: false,
            }),
        }
    }

    pub fn projection(&self) -> Result<GeneralProjection, String> {
        let general = self
            .services
            .configuration
            .snapshot()
            .map_err(|error| error.to_string())?
            .general;
        let permissions = self.services.permissions.status();
        let permission_context = self.services.permissions.context();
        let state = self.state.lock().unwrap();
        Ok(GeneralProjection {
            general,
            logs: state.logs.clone(),
            permissions: Some(permissions),
            permission_context: Some(permission_context),
            permission_busy: state.permission_busy,
            version: self.services.version.clone(),
            update: state.update.clone(),
            update_phase: state.phase,
            downloaded_path: state.downloaded_path.clone(),
            error: state.error.clone(),
        })
    }

    pub fn edit(
        &self,
        field: GeneralField,
        value: GeneralValue,
    ) -> Result<GeneralProjection, String> {
        let result = self
            .services
            .settings
            .edit_general(field, value)
            .map(|_| ());
        self.finish(result)
    }

    /// Call when the owning native view is disposed. A late network result must
    /// not replace the state of a subsequently mounted page.
    pub fn invalidate_pending_updates(&self) {
        let mut state = self.state.lock().unwrap();
        state.update_generation += 1;
        if state.phase == UpdatePhase::Checking {
            state.attempted_check = false;
        }
        state.phase = UpdatePhase::Idle;
    }

    pub async fn ensure_checked(
        &self,
        changed: impl Fn(GeneralProjection),
    ) -> Result<GeneralProjection, String> {
        {
            let mut state = self.state.lock().unwrap();
            if state.attempted_check {
                drop(state);
                return self.projection();
            }
            state.attempted_check = true;
        }
        self.perform(GeneralAction::CheckUpdates, changed).await
    }

    pub async fn perform(
        &self,
        action: GeneralAction,
        changed: impl Fn(GeneralProjection),
    ) -> Result<GeneralProjection, String> {
        if matches!(
            action,
            GeneralAction::CheckUpdates | GeneralAction::DownloadUpdate
        ) {
            return self.update(action, changed).await;
        }
        if matches!(
            action,
            GeneralAction::RequestPermission(_)
                | GeneralAction::ResetPermission(_)
                | GeneralAction::Restart
        ) {
            return self.permission(action, changed);
        }
        if matches!(action, GeneralAction::RefreshPermissions)
            && self.state.lock().unwrap().permission_busy
        {
            return self.projection();
        }
        let result = match action {
            GeneralAction::Edit(field, value) => self
                .services
                .settings
                .edit_general(field, value)
                .map(|_| ()),
            GeneralAction::RefreshLogs => self.services.settings.list_logs(8).map(|logs| {
                self.state.lock().unwrap().logs = logs;
            }),
            GeneralAction::ClearLogs => self.services.settings.clear_logs().map(|()| {
                self.state.lock().unwrap().logs.clear();
            }),
            GeneralAction::ClearHistory => self.services.history.clear_all_history().await,
            GeneralAction::OpenProject => {
                (self.services.open_url)("https://github.com/gamilian/snaplingo")
            }
            GeneralAction::OpenNotices => (self.services.open_notices)(),
            GeneralAction::OpenReleases => self.services.updates.open_release_page(),
            GeneralAction::RefreshPermissions => Ok(()),
            GeneralAction::RequestPermission(_)
            | GeneralAction::ResetPermission(_)
            | GeneralAction::Restart => unreachable!(),
            GeneralAction::CheckUpdates | GeneralAction::DownloadUpdate => unreachable!(),
        };
        let result = self.finish(result);
        if let Ok(projection) = self.projection() {
            changed(projection);
        }
        result
    }

    fn finish(&self, result: snaplingo_core::Result<()>) -> Result<GeneralProjection, String> {
        self.state.lock().unwrap().error = result.as_ref().err().map(ToString::to_string);
        result.map_err(|error| error.to_string())?;
        self.projection()
    }

    fn permission(
        &self,
        action: GeneralAction,
        changed: impl Fn(GeneralProjection),
    ) -> Result<GeneralProjection, String> {
        {
            let mut state = self.state.lock().unwrap();
            if state.permission_busy {
                drop(state);
                return self.projection();
            }
            state.permission_busy = true;
            state.error = None;
        }
        let operation = PermissionOperation(&self.state);
        changed(self.projection()?);
        let result = match action {
            GeneralAction::RequestPermission(permission) => {
                self.services.permissions.request(permission).map(|_| ())
            }
            GeneralAction::ResetPermission(permission) => {
                self.services.permissions.reset(permission).map(|_| ())
            }
            GeneralAction::Restart => self.services.permissions.restart(),
            _ => unreachable!(),
        };
        self.state.lock().unwrap().error = result.as_ref().err().map(ToString::to_string);
        drop(operation);
        let projection = self.projection()?;
        changed(projection.clone());
        result.map_err(|error| error.to_string())?;
        Ok(projection)
    }

    async fn update(
        &self,
        action: GeneralAction,
        changed: impl Fn(GeneralProjection),
    ) -> Result<GeneralProjection, String> {
        let checking = matches!(action, GeneralAction::CheckUpdates);
        let generation = {
            let mut state = self.state.lock().unwrap();
            if state.phase != UpdatePhase::Idle {
                drop(state);
                return self.projection();
            }
            state.phase = if checking {
                UpdatePhase::Checking
            } else {
                UpdatePhase::Downloading
            };
            state.error = None;
            if checking {
                state.attempted_check = true;
            } else {
                state.downloaded_path = None;
            }
            state.update_generation += 1;
            state.update_generation
        };
        changed(self.projection()?);
        let result = if checking {
            self.services
                .updates
                .check()
                .await
                .map(|status| (Some(status), None))
        } else {
            self.services
                .updates
                .download_and_open()
                .await
                .map(|path| (None, Some(path)))
        };
        let mut state = self.state.lock().unwrap();
        if state.update_generation != generation {
            drop(state);
            return self.projection();
        }
        state.phase = UpdatePhase::Idle;
        let error = match result {
            Ok((status, path)) => {
                if checking {
                    state.update = status;
                }
                state.downloaded_path = path;
                None
            }
            Err(error) => Some(error.to_string()),
        };
        state.error = error.clone();
        drop(state);
        let projection = self.projection()?;
        changed(projection.clone());
        match error {
            Some(error) => Err(error),
            None => Ok(projection),
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    use async_trait::async_trait;

    use snaplingo_core::permissions::RequiredPermissionsHost;
    use snaplingo_core::settings::StartOnBoot;
    use snaplingo_core::updates::AppUpdateHost;
    use snaplingo_core::updates::PublishedRelease;
    use snaplingo_core::updates::ReleaseAsset;
    use snaplingo_core::updates::UpdateInstaller;
    use snaplingo_storage::{
        Database, SqliteAppLogRepository, SqliteConfigStore, SqliteHistoryRepository,
    };

    struct Registrar;
    impl snaplingo_core::hotkeys::HotkeyRegistrar for Registrar {
        fn register(
            &self,
            _: snaplingo_core::hotkeys::HotkeyRegistration,
        ) -> snaplingo_core::Result<()> {
            Ok(())
        }
        fn unregister(&self, _: &str) -> snaplingo_core::Result<()> {
            Ok(())
        }
    }
    struct Notifier;
    impl snaplingo_core::hotkeys::HotkeyChangeNotifier for Notifier {
        fn hotkeys_changed(&self) {}
    }
    #[derive(Default)]
    struct Boot(AtomicBool);
    impl StartOnBoot for Boot {
        fn set_enabled(&self, _: bool) -> snaplingo_core::Result<()> {
            if self.0.load(Ordering::SeqCst) {
                Err("startup registration failed".into())
            } else {
                Ok(())
            }
        }
    }

    struct Permissions {
        status: Mutex<RequiredPermissionsStatus>,
        calls: Mutex<Vec<String>>,
        entered: tokio::sync::Notify,
        blocked: Mutex<Option<std::sync::mpsc::Receiver<()>>>,
        fail: AtomicBool,
    }
    impl RequiredPermissionsHost for Permissions {
        fn status(&self) -> RequiredPermissionsStatus {
            *self.status.lock().unwrap()
        }
        fn context(&self) -> RequiredPermissionsContext {
            RequiredPermissionsContext {
                platform: "macos".into(),
                app_path: Some("/Applications/SnapLingo.app".into()),
                needs_installation: false,
                can_reset: true,
            }
        }
        fn request(&self, permission: SystemPermission) -> snaplingo_core::Result<()> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("request:{permission:?}"));
            let blocked = self.blocked.lock().unwrap().take();
            self.entered.notify_one();
            if let Some(blocked) = blocked {
                blocked
                    .recv_timeout(std::time::Duration::from_secs(5))
                    .unwrap();
            }
            if self.fail.load(Ordering::SeqCst) {
                return Err("fixture permission unavailable".into());
            }
            match permission {
                SystemPermission::Accessibility => self.status.lock().unwrap().accessibility = true,
                SystemPermission::ScreenRecording => {
                    self.status.lock().unwrap().screen_recording = true
                }
            }
            Ok(())
        }
        fn reset(&self, permission: SystemPermission) -> snaplingo_core::Result<()> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("reset:{permission:?}"));
            Ok(())
        }
        fn restart(&self) -> snaplingo_core::Result<()> {
            self.calls.lock().unwrap().push("restart".into());
            Ok(())
        }
    }

    #[derive(Default)]
    struct Updates {
        checks: AtomicUsize,
        fail: AtomicBool,
        entered: tokio::sync::Notify,
        blocked: Mutex<Option<tokio::sync::oneshot::Receiver<()>>>,
        calls: Mutex<Vec<String>>,
    }
    #[async_trait]
    impl AppUpdateHost for Updates {
        async fn latest_release(&self) -> snaplingo_core::Result<Option<PublishedRelease>> {
            self.checks.fetch_add(1, Ordering::SeqCst);
            let blocked = self.blocked.lock().unwrap().take();
            self.entered.notify_one();
            if let Some(blocked) = blocked {
                let _ = blocked.await;
            }
            if self.fail.load(Ordering::SeqCst) {
                return Err("fixture update unavailable".into());
            }
            let name = "snaplingo-v0.1.24-macos-aarch64.dmg";
            Ok(Some(PublishedRelease {
                tag_name: "v0.1.24".into(),
                draft: false,
                prerelease: false,
                body: Some("Fixture release notes".into()),
                published_at: None,
                assets: vec![ReleaseAsset {
                    name: name.into(),
                    browser_download_url: format!(
                        "https://github.com/gamilian/snaplingo/releases/download/v0.1.24/{name}"
                    ),
                    size: 100,
                    digest: Some(format!("sha256:{}", "a".repeat(64))),
                    state: "uploaded".into(),
                }],
            }))
        }
        async fn download_and_open(
            &self,
            installer: &UpdateInstaller,
        ) -> snaplingo_core::Result<String> {
            self.calls.lock().unwrap().push(installer.name.clone());
            Ok("/tmp/SnapLingo fixture.dmg".into())
        }
        fn open_release_page(&self) -> snaplingo_core::Result<()> {
            self.calls.lock().unwrap().push("releases".into());
            Ok(())
        }
    }

    pub(crate) struct Fixture {
        controller: Arc<GeneralController>,
        database: Arc<Database>,
        boot: Arc<Boot>,
        permissions: Arc<Permissions>,
        updates: Arc<Updates>,
        opened: Arc<Mutex<Vec<String>>>,
    }
    impl Fixture {
        pub(crate) fn preferences(
            &self,
        ) -> crate::features::settings::presenter::preferences::PreferencesDependencies {
            let configuration = self.controller.services.configuration.clone();
            crate::features::settings::presenter::preferences::PreferencesDependencies {
                configuration: configuration.clone(),
                settings: self.controller.services.settings.clone(),
                hotkeys: Arc::new(
                    snaplingo_core::hotkeys::HotkeyRuntime::with_change_notifier(
                        Arc::new(snaplingo_core::hotkeys::HotkeyConfiguration::new(Arc::new(
                            SqliteConfigStore::new(self.database.clone()),
                        ))),
                        Arc::new(Registrar),
                        Arc::new(Notifier),
                    ),
                ),
                general: self.controller.clone(),
                usage: crate::features::settings::presenter::capacity_usage::CapacityUsage::new(
                    Arc::new(snaplingo_core::library::index::LibraryIndex::new(Arc::new(
                        snaplingo_storage::SqliteLibraryIndexRepository::new(self.database.clone()),
                    ))),
                    tokio::runtime::Handle::current(),
                    Arc::new(|| {}),
                ),
            }
        }
    }
    pub(crate) fn fixture() -> Fixture {
        let database = Arc::new(Database::in_memory().unwrap());
        let configuration = Arc::new(SettingsConfiguration::new(Arc::new(
            SqliteConfigStore::new(database.clone()),
        )));
        let history = Arc::new(History::new(Arc::new(SqliteHistoryRepository::new(
            database.clone(),
        ))));
        let logs = Arc::new(SqliteAppLogRepository::new(database.clone()));
        logs.record("INFO", "fixture", "Initial log").unwrap();
        let boot = Arc::new(Boot::default());
        let permissions = Arc::new(Permissions {
            status: Mutex::new(RequiredPermissionsStatus {
                screen_recording: true,
                accessibility: false,
            }),
            calls: Mutex::new(Vec::new()),
            entered: tokio::sync::Notify::new(),
            blocked: Mutex::new(None),
            fail: AtomicBool::new(false),
        });
        let updates = Arc::new(Updates::default());
        let opened = Arc::new(Mutex::new(Vec::new()));
        let open_url = opened.clone();
        let notices = opened.clone();
        let controller = Arc::new(GeneralController::new(GeneralServices {
            configuration: configuration.clone(),
            settings: Arc::new(SettingsApplication::new(
                configuration,
                boot.clone(),
                history.clone(),
                logs,
            )),
            history,
            permissions: Arc::new(RequiredPermissions::new(permissions.clone())),
            updates: Arc::new(AppUpdates::new(
                updates.clone(),
                "0.1.23".into(),
                "macos".into(),
                "aarch64".into(),
            )),
            version: "0.1.23".into(),
            open_url: Arc::new(move |url| {
                open_url.lock().unwrap().push(url.into());
                Ok(())
            }),
            open_notices: Arc::new(move || {
                notices.lock().unwrap().push("notices".into());
                Ok(())
            }),
        }));
        Fixture {
            controller,
            database,
            boot,
            permissions,
            updates,
            opened,
        }
    }

    #[test]
    fn edits_preserve_latest_settings_and_report_startup_rollback() {
        let f = fixture();
        let mut general = f.controller.projection().unwrap().general;
        general.system_tts_rate = 210;
        f.controller
            .services
            .configuration
            .update_general(general)
            .unwrap();
        let projection = f
            .controller
            .edit(GeneralField::Appearance, GeneralValue::Text("dark".into()))
            .unwrap();
        assert_eq!(projection.general.system_tts_rate, 210);
        assert_eq!(projection.general.theme, "dark");
        f.boot.0.store(true, Ordering::SeqCst);
        assert!(f
            .controller
            .edit(GeneralField::StartOnBoot, GeneralValue::Toggle(true))
            .is_err());
        let projection = f.controller.projection().unwrap();
        assert!(!projection.general.start_on_boot);
        assert!(projection
            .error
            .unwrap()
            .contains("startup registration failed"));
        f.controller
            .edit(GeneralField::Appearance, GeneralValue::Text("light".into()))
            .unwrap();
        assert!(f.controller.projection().unwrap().error.is_none());
    }

    #[tokio::test]
    async fn maintenance_permissions_and_external_actions_use_rust_services() {
        let f = fixture();
        assert_eq!(f.controller.projection().unwrap().logs.len(), 1);
        f.database.with_connection(|connection| {
            connection.execute("INSERT INTO history_records (kind, timestamp) VALUES ('translation', '2026-11-07')", []).map_err(snaplingo_core::AppError::database)?;
            Ok(())
        }).unwrap();
        for action in [
            GeneralAction::RefreshLogs,
            GeneralAction::ClearLogs,
            GeneralAction::ClearHistory,
            GeneralAction::RequestPermission(SystemPermission::Accessibility),
            GeneralAction::ResetPermission(SystemPermission::ScreenRecording),
            GeneralAction::Restart,
            GeneralAction::OpenProject,
            GeneralAction::OpenNotices,
            GeneralAction::OpenReleases,
        ] {
            f.controller.perform(action, |_| {}).await.unwrap();
        }
        assert!(f.controller.projection().unwrap().logs.is_empty());
        let count = f
            .database
            .with_connection(|connection| {
                Ok(connection
                    .query_row("SELECT count(*) FROM history_records", [], |row| {
                        row.get::<_, u32>(0)
                    })
                    .map_err(snaplingo_core::AppError::database)?)
            })
            .unwrap();
        assert_eq!(count, 0);
        assert!(
            f.controller
                .projection()
                .unwrap()
                .permissions
                .unwrap()
                .accessibility
        );
        assert_eq!(
            *f.permissions.calls.lock().unwrap(),
            [
                "request:Accessibility",
                "reset:ScreenRecording",
                "request:ScreenRecording",
                "restart"
            ]
        );
        assert_eq!(
            *f.opened.lock().unwrap(),
            ["https://github.com/gamilian/snaplingo", "notices"]
        );
        assert_eq!(*f.updates.calls.lock().unwrap(), ["releases"]);
    }

    #[tokio::test]
    async fn checks_once_automatically_and_preserves_update_error_and_download_state() {
        let f = fixture();
        let phases = Mutex::new(Vec::new());
        f.controller
            .ensure_checked(|p| phases.lock().unwrap().push(p.update_phase))
            .await
            .unwrap();
        f.controller
            .ensure_checked(|_| panic!("The initial check must not repeat"))
            .await
            .unwrap();
        assert_eq!(f.updates.checks.load(Ordering::SeqCst), 1);
        assert_eq!(
            *phases.lock().unwrap(),
            [UpdatePhase::Checking, UpdatePhase::Idle]
        );
        let projection = f
            .controller
            .perform(GeneralAction::DownloadUpdate, |_| {})
            .await
            .unwrap();
        assert_eq!(
            projection.downloaded_path.as_deref(),
            Some("/tmp/SnapLingo fixture.dmg")
        );
        assert!(projection.update.unwrap().available);
        f.updates.fail.store(true, Ordering::SeqCst);
        assert!(f
            .controller
            .perform(GeneralAction::CheckUpdates, |_| {})
            .await
            .is_err());
        let projection = f.controller.projection().unwrap();
        assert_eq!(projection.update_phase, UpdatePhase::Idle);
        assert!(
            projection.update.unwrap().available,
            "A failed refresh keeps the previous status"
        );
        assert!(
            projection.downloaded_path.is_some(),
            "A failed refresh keeps the downloaded path"
        );
        assert!(projection
            .error
            .unwrap()
            .contains("fixture update unavailable"));
    }

    #[tokio::test]
    async fn repeated_clicks_do_not_start_another_check_and_disposal_rejects_late_completion() {
        let f = fixture();
        let (release, blocked) = tokio::sync::oneshot::channel();
        *f.updates.blocked.lock().unwrap() = Some(blocked);
        let controller = f.controller.clone();
        let pending = tokio::spawn(async move {
            controller
                .perform(GeneralAction::CheckUpdates, |_| {})
                .await
        });
        f.updates.entered.notified().await;
        assert_eq!(
            f.controller
                .perform(GeneralAction::CheckUpdates, |_| {})
                .await
                .unwrap()
                .update_phase,
            UpdatePhase::Checking
        );
        assert_eq!(f.updates.checks.load(Ordering::SeqCst), 1);
        f.controller.invalidate_pending_updates();
        release.send(()).unwrap();
        pending.await.unwrap().unwrap();
        let projection = f.controller.projection().unwrap();
        assert_eq!(projection.update_phase, UpdatePhase::Idle);
        assert!(projection.update.is_none());
        assert!(projection.error.is_none());
        f.controller.ensure_checked(|_| {}).await.unwrap();
        assert!(
            f.controller.projection().unwrap().update.is_some(),
            "Remount must retry a check discarded during disposal"
        );
        assert_eq!(f.updates.checks.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn pending_permission_actions_suppress_repeated_requests_and_recover_after_errors() {
        let f = fixture();
        let (release, blocked) = std::sync::mpsc::channel();
        *f.permissions.blocked.lock().unwrap() = Some(blocked);
        f.permissions.fail.store(true, Ordering::SeqCst);
        let phases = Arc::new(Mutex::new(Vec::new()));
        let observed = phases.clone();
        let controller = f.controller.clone();
        let pending = std::thread::spawn(move || {
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(controller.perform(
                    GeneralAction::RequestPermission(SystemPermission::Accessibility),
                    move |projection| observed.lock().unwrap().push(projection.permission_busy),
                ))
        });
        f.permissions.entered.notified().await;
        assert!(f.controller.projection().unwrap().permission_busy);
        for action in [
            GeneralAction::RequestPermission(SystemPermission::Accessibility),
            GeneralAction::ResetPermission(SystemPermission::ScreenRecording),
            GeneralAction::Restart,
            GeneralAction::RefreshPermissions,
        ] {
            assert!(
                f.controller
                    .perform(action, |_| panic!(
                        "Repeated action must not start another operation"
                    ))
                    .await
                    .unwrap()
                    .permission_busy
            );
        }
        assert_eq!(
            *f.permissions.calls.lock().unwrap(),
            ["request:Accessibility"]
        );
        release.send(()).unwrap();
        assert!(pending.join().unwrap().is_err());
        let projection = f.controller.projection().unwrap();
        assert!(!projection.permission_busy);
        assert!(projection
            .error
            .unwrap()
            .contains("fixture permission unavailable"));
        assert_eq!(*phases.lock().unwrap(), [true, false]);
        f.permissions.fail.store(false, Ordering::SeqCst);
        let projection = f
            .controller
            .perform(
                GeneralAction::RequestPermission(SystemPermission::Accessibility),
                |_| {},
            )
            .await
            .unwrap();
        assert!(!projection.permission_busy);
        assert!(projection.error.is_none());
        assert!(projection.permissions.unwrap().accessibility);
        assert_eq!(f.permissions.calls.lock().unwrap().len(), 2);
    }
}

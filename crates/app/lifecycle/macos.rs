//! Standalone AppKit application ownership. The retained Tauri entry stays the
//! default while the complete native window surface is being accepted.
use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use objc2::{define_class, msg_send, rc::Retained, runtime::ProtocolObject, MainThreadMarker};
use objc2_app_kit::{
    NSApplication, NSApplicationDelegate, NSApplicationTerminateReply, NSEvent,
    NSEventModifierFlags, NSEventType,
};
use objc2_foundation::{NSObject, NSObjectProtocol, NSPoint};
use snaplingo_ui::Page;

use crate::infrastructure::desktop_events::{DesktopEvent, DesktopSubscription, SettingsRoute};
use crate::infrastructure::storage::Database;
use crate::infrastructure::{runtime, ui_thread};
use crate::window::settings::macos::NativeSettingsWindow;
use crate::{AppState, RuntimeLifecycle};
use snaplingo_core::application::{
    HotkeyRecording, SelectedTextAcquirer, SelectionScheme, SystemSelectionProvider,
};

pub struct NativeAppOptions {
    pub data_dir: PathBuf,
    pub identifier: String,
    pub ocr_resource_root: Option<PathBuf>,
}

thread_local! { static ACTIVE: RefCell<Option<Rc<NativeApplication>>> = const { RefCell::new(None) }; }
fn current() -> Option<Rc<NativeApplication>> {
    ACTIVE.with_borrow(Clone::clone)
}

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "SnapLingoNativeApplicationDelegate"]
    #[thread_kind = objc2::MainThreadOnly]
    struct ApplicationDelegate;
    unsafe impl NSObjectProtocol for ApplicationDelegate {}
    unsafe impl NSApplicationDelegate for ApplicationDelegate {
        #[unsafe(method(applicationShouldTerminate:))]
        fn should_terminate(&self, _: &NSApplication) -> NSApplicationTerminateReply {
            if let Some(app) = current() {
                app.begin_shutdown(false);
            }
            NSApplicationTerminateReply::TerminateCancel
        }
        #[unsafe(method(applicationShouldTerminateAfterLastWindowClosed:))]
        fn terminate_after_last_window(&self, _: &NSApplication) -> bool {
            false
        }
        #[unsafe(method(applicationShouldHandleReopen:hasVisibleWindows:))]
        fn reopen(&self, _: &NSApplication, _: bool) -> bool {
            if let Some(app) = current() {
                if let Err(error) = app.show_settings(None) {
                    log::warn!("Failed to reopen native Settings: {error}");
                }
            }
            true
        }
    }
);

struct NativeApplication {
    state: Arc<AppState>,
    lifecycle: Arc<RuntimeLifecycle>,
    recording: HotkeyRecording,
    settings: RefCell<Option<Rc<NativeSettingsWindow>>>,
    instance: RefCell<Option<crate::infrastructure::system::single_instance::SingleInstance>>,
    subscription: RefCell<Option<DesktopSubscription>>,
    restart: Cell<bool>,
}

impl NativeApplication {
    fn show_settings(&self, route: Option<SettingsRoute>) -> Result<(), String> {
        let (page, section) = match route {
            Some(SettingsRoute::History) => (Page::History, None),
            Some(SettingsRoute::About) => (Page::General, Some("about")),
            Some(SettingsRoute::Permissions) => (Page::General, Some("permissions")),
            None => (
                self.settings
                    .borrow()
                    .as_ref()
                    .and_then(|window| window.page())
                    .unwrap_or(Page::General),
                None,
            ),
        };
        self.show_page(page, section)
    }

    fn show_page(&self, page: Page, section: Option<&str>) -> Result<(), String> {
        if self.lifecycle.is_shutdown_complete() {
            return Ok(());
        }
        if self.settings.borrow().is_none() {
            let window = NativeSettingsWindow::new(
                MainThreadMarker::new().ok_or("Native Settings must open on the main thread")?,
                self.state.clone(),
                self.recording.clone(),
            )?;
            *self.settings.borrow_mut() = Some(window);
        }
        self.settings
            .borrow()
            .as_ref()
            .unwrap()
            .show_page(page, section)
    }

    fn receive(&self, event: DesktopEvent) -> Result<(), String> {
        match event {
            DesktopEvent::OpenSettings => self.show_settings(None),
            DesktopEvent::OpenSettingsAt(route) => self.show_settings(Some(route)),
            DesktopEvent::Quit => {
                self.begin_shutdown(false);
                Ok(())
            }
            DesktopEvent::Restart => {
                self.begin_shutdown(true);
                Ok(())
            }
            DesktopEvent::SettingsChanged => {
                if let Some(window) = self.settings.borrow().as_ref() {
                    window.layout();
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn begin_shutdown(&self, restart: bool) {
        if !self.lifecycle.request_shutdown() {
            return;
        }
        self.restart.set(restart);
        self.state.actions.shutdown();
        if let Some(window) = self.settings.borrow().as_ref() {
            if let Err(error) = window.hide() {
                log::warn!("Failed to remember native Settings before exit: {error}");
            }
        }
        let state = self.state.clone();
        let lifecycle = self.lifecycle.clone();
        runtime::spawn(async move {
            if let Err(error) = lifecycle.shutdown(&state).await {
                log::warn!("Failed to gracefully shut down native application: {error}");
            }
            ui_thread::dispatch(stop_event_loop);
        });
    }

    fn dispose(&self) {
        self.state.actions.shutdown();
        self.subscription.borrow_mut().take();
        if let Some(window) = self.settings.borrow_mut().take() {
            window.dispose();
        }
        crate::lifecycle::shell::shutdown_menu_bar();
        if let Err(error) = crate::infrastructure::system::shortcut::shutdown_shortcuts() {
            log::warn!("Failed to release native shortcuts: {error}");
        }
        self.instance.borrow_mut().take();
    }
}

/// Open native Settings explicitly from a shared page's navigation callback.

pub fn run() -> Result<(), String> {
    let data_dir =
        crate::infrastructure::system::get_app_data_dir().map_err(|error| error.to_string())?;
    run_with_options(NativeAppOptions {
        data_dir,
        identifier: "com.snaplingo.app".into(),
        ocr_resource_root: ocr_resource_root(),
    })
}

pub fn run_with_options(options: NativeAppOptions) -> Result<(), String> {
    run_application(options, |_| {}, |_| {})
}

fn run_application(
    options: NativeAppOptions,
    prepare: impl FnOnce(&mut AppState),
    ready: impl FnOnce(&Rc<NativeApplication>),
) -> Result<(), String> {
    use objc2::MainThreadOnly;
    let mtm =
        MainThreadMarker::new().ok_or("Native application must run on the process main thread")?;
    let app = NSApplication::sharedApplication(mtm);
    if app.isRunning() || current().is_some() {
        return Err("A native application event loop is already running".into());
    }
    std::fs::create_dir_all(&options.data_dir).map_err(|error| error.to_string())?;
    let Some(instance) = crate::infrastructure::system::single_instance::SingleInstance::acquire(
        &options.data_dir.join("instance.lock"),
    )
    .map_err(|error| error.to_string())?
    else {
        return Ok(());
    };
    let database_path = options.data_dir.join("snaplingo.db");
    let first_launch = !database_path.exists();
    let database = Arc::new(Database::open(&database_path).map_err(|error| error.to_string())?);
    crate::infrastructure::logging::initialize(
        crate::infrastructure::storage::SqliteAppLogRepository::new(database.clone()),
    )
    .map_err(|error| error.to_string())?;
    let provider = Arc::new(
        crate::infrastructure::system::selection::macos::MacSelectionProvider::new(Some(
            options.identifier.clone(),
        )),
    );
    let mut state = crate::composition::build_app_state(
        database,
        database_path,
        crate::composition::CompositionOptions {
            identifier: options.identifier,
            version: env!("CARGO_PKG_VERSION").into(),
            ocr_resource_root: options.ocr_resource_root,
            cache_dir: Ok(options.data_dir.join("cache")),
            selected_text: Arc::new(SelectedTextAcquirer::new(
                SelectionScheme::new(provider.default_scheme()),
                provider.methods(),
                provider,
            )),
        },
    );
    prepare(&mut state);
    let state = Arc::new(state);
    let recording = HotkeyRecording::default();
    let events = state.desktop_events.clone();
    crate::infrastructure::system::shortcut::initialize_shortcuts(
        recording.clone(),
        move |label, payload| {
            events.publish(DesktopEvent::HotkeyRecorded {
                window_label: label.into(),
                recording_id: payload.recording_id,
                hotkey: payload.hotkey,
            });
        },
    )
    .map_err(|error| error.to_string())?;
    let lifecycle = Arc::new(RuntimeLifecycle::start(&state));
    let owner = Rc::new(NativeApplication {
        state,
        lifecycle,
        recording,
        settings: RefCell::new(None),
        instance: RefCell::new(Some(instance)),
        subscription: RefCell::new(None),
        restart: Cell::new(false),
    });
    *owner.subscription.borrow_mut() =
        Some(owner.state.desktop_events.connect_controller(|event| {
            let event = event.clone();
            if MainThreadMarker::new().is_some() {
                current()
                    .ok_or("Native desktop controller is unavailable")?
                    .receive(event)
            } else {
                ui_thread::dispatch(move || {
                    if let Some(app) = current() {
                        if let Err(error) = app.receive(event) {
                            log::warn!("Failed to handle native desktop event: {error}");
                        }
                    }
                });
                Ok(())
            }
        }));
    ACTIVE.with_borrow_mut(|active| *active = Some(owner.clone()));
    let events = owner.state.desktop_events.clone();
    owner
        .instance
        .borrow()
        .as_ref()
        .unwrap()
        .on_activation(move || {
            let events = events.clone();
            ui_thread::dispatch(move || {
                events.publish(DesktopEvent::AppActionRequested(
                    crate::actions::AppAction::OpenSettings,
                ))
            });
        });
    let delegate = ApplicationDelegate::alloc(mtm).set_ivars(());
    let delegate: Retained<ApplicationDelegate> = unsafe { msg_send![super(delegate), init] };
    app.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
    crate::lifecycle::shell::initialize_menu_events();
    let startup = (|| {
        crate::lifecycle::shell::apply_resting_activation_policy()?;
        let events = owner.state.desktop_events.clone();
        crate::lifecycle::shell::setup_menu_bar(
            &owner
                .state
                .settings
                .hotkeys
                .snapshot()
                .map_err(|error| error.to_string())?,
            move |action| {
                events.publish(DesktopEvent::AppActionRequested(action));
            },
        )?;
        app.finishLaunching();
        if crate::should_show_settings_on_startup(
            first_launch,
            owner.state.permissions.status().screen_recording,
        ) {
            owner.show_settings(None)?;
        }
        ready(&owner);
        app.run();
        Ok::<(), String>(())
    })();
    owner.dispose();
    ACTIVE.with_borrow_mut(|active| active.take());
    app.setDelegate(None);
    if startup.is_ok() && owner.restart.get() {
        let executable = std::env::current_exe().map_err(|error| error.to_string())?;
        std::process::Command::new(executable)
            .args(std::env::args_os().skip(1))
            .spawn()
            .map_err(|error| error.to_string())?;
    }
    startup
}

fn stop_event_loop() {
    let app = NSApplication::sharedApplication(MainThreadMarker::new().unwrap());
    app.stop(None);
    if let Some(wake) = NSEvent::otherEventWithType_location_modifierFlags_timestamp_windowNumber_context_subtype_data1_data2(
        NSEventType::ApplicationDefined, NSPoint::new(0., 0.), NSEventModifierFlags::empty(), 0., 0, None, 0, 0, 0,
    ) { app.postEvent_atStart(&wake, true); }
}

fn ocr_resource_root() -> Option<PathBuf> {
    if cfg!(debug_assertions) {
        return Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/ocr"));
    }
    std::env::current_exe().ok().and_then(|executable| {
        executable
            .parent()?
            .parent()
            .map(|contents| contents.join("Resources/ocr"))
    })
}

#[cfg(test)]
async fn smoke_on_main<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    ui_thread::dispatch(move || {
        let _ = sender.send(std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)));
    });
    match tokio::time::timeout(std::time::Duration::from_secs(15), receiver)
        .await
        .unwrap()
        .unwrap()
    {
        Ok(result) => result,
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

#[cfg(test)]
pub(crate) fn permission_smoke_test() {
    use snaplingo_core::application::{
        RequiredPermissions, RequiredPermissionsContext, RequiredPermissionsHost,
        RequiredPermissionsStatus, SystemPermission,
    };
    use snaplingo_ui::permission_prompt::{
        PermissionPromptInput as Input, PermissionPromptTarget as Target,
    };
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    struct Startup;
    impl snaplingo_core::application::StartOnBoot for Startup {
        fn set_enabled(&self, _: bool) -> crate::Result<()> {
            Ok(())
        }
    }
    struct Missing {
        screen: AtomicBool,
        temporary: AtomicBool,
        fail: AtomicBool,
        requests: AtomicUsize,
    }
    impl RequiredPermissionsHost for Missing {
        fn status(&self) -> RequiredPermissionsStatus {
            RequiredPermissionsStatus {
                screen_recording: self.screen.load(Ordering::SeqCst),
                accessibility: false,
            }
        }
        fn context(&self) -> RequiredPermissionsContext {
            RequiredPermissionsContext {
                platform: "macos".into(),
                app_path: Some("/Applications/SnapLingo.app".into()),
                needs_installation: self.temporary.load(Ordering::SeqCst),
                can_reset: true,
            }
        }
        fn request(&self, p: SystemPermission) -> crate::Result<()> {
            assert_eq!(p, SystemPermission::ScreenRecording);
            self.requests.fetch_add(1, Ordering::SeqCst);
            if self.fail.load(Ordering::SeqCst) {
                Err("Synthetic permission error".into())
            } else {
                Ok(())
            }
        }
        fn reset(&self, _: SystemPermission) -> crate::Result<()> {
            panic!("Permission guide must not reset permissions")
        }
        fn restart(&self) -> crate::Result<()> {
            panic!("Permission guide must not restart")
        }
    }
    let permissions = Arc::new(Missing {
        screen: AtomicBool::new(false),
        temporary: AtomicBool::new(false),
        fail: AtomicBool::new(true),
        requests: AtomicUsize::new(0),
    });
    let directory = tempfile::tempdir().unwrap();
    run_application(NativeAppOptions{data_dir:directory.path().into(),identifier:"com.snaplingo.permission-guide-smoke".into(),ocr_resource_root:None},
        |state|{state.permissions=Arc::new(RequiredPermissions::new(permissions.clone()));let settings=Arc::get_mut(&mut state.settings).unwrap();settings.application=Arc::new(snaplingo_core::application::SettingsApplication::new(settings.configuration.clone(),Arc::new(Startup),state.history.history.clone(),state.logs.repository.clone()));},
        |owner|{
            let settings=owner.settings.borrow().as_ref().unwrap().clone();
            assert!(settings.test_window().attachedSheet().is_some(),"Missing Screen Recording must present the permission guide when native Settings opens");
            assert_eq!(permissions.requests.load(Ordering::SeqCst),0,"Opening the guide must not request permissions");
            let permissions=permissions.clone();
            std::thread::spawn(move||{
                let result=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||runtime::block_on(async move{
                    smoke_on_main(||{assert!(crate::window::settings::macos::permissions::test_input(Input::Activate(Target::Authorize)));assert!(crate::window::settings::macos::permissions::test_snapshot().unwrap().busy);crate::window::settings::macos::permissions::test_input(Input::Activate(Target::Authorize));Ok(())}).await.unwrap();
                    for attempt in 0..100{
                        let done=smoke_on_main(||Ok(crate::window::settings::macos::permissions::test_snapshot().is_some_and(|s|!s.busy&&s.error.is_some()))).await.unwrap();
                        if done{break;}assert!(attempt<99,"Permission request did not finish");tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                    }
                    assert_eq!(permissions.requests.load(Ordering::SeqCst),1);
                    permissions.fail.store(false,Ordering::SeqCst);
                    smoke_on_main(||{crate::window::settings::macos::permissions::test_input(Input::Activate(Target::Authorize));Ok(())}).await.unwrap();
                    for attempt in 0..100{
                        let done=smoke_on_main(||Ok(crate::window::settings::macos::permissions::test_snapshot().is_some_and(|s|!s.busy&&s.error.is_none()))).await.unwrap();
                        if done{break;}assert!(attempt<99,"Permission retry did not finish");tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                    }
                    assert_eq!(permissions.requests.load(Ordering::SeqCst),2);
                    permissions.screen.store(true,Ordering::SeqCst);
                    smoke_on_main(||{current().unwrap().settings.borrow().as_ref().unwrap().refresh_permissions();assert!(!crate::window::settings::macos::permissions::test_is_open(),"Screen Recording grant closes the guide even when accessibility is missing");Ok(())}).await.unwrap();
                    permissions.screen.store(false,Ordering::SeqCst);
                    permissions.temporary.store(true,Ordering::SeqCst);
                    smoke_on_main(||{current().unwrap().settings.borrow().as_ref().unwrap().refresh_permissions();assert!(crate::window::settings::macos::permissions::test_snapshot().unwrap().needs_installation);crate::window::settings::macos::permissions::test_input(Input::Activate(Target::Authorize));Ok(())}).await.unwrap();
                    assert_eq!(permissions.requests.load(Ordering::SeqCst),2,"Temporary copies cannot request permissions");
                    permissions.temporary.store(false,Ordering::SeqCst);
                    smoke_on_main(||{let owner=current().unwrap();let settings=owner.settings.borrow().as_ref().unwrap().clone();settings.refresh_permissions();assert!(crate::window::settings::macos::permissions::test_snapshot().unwrap().attempted);crate::window::settings::macos::permissions::test_input(Input::Activate(Target::Help));assert!(!crate::window::settings::macos::permissions::test_is_open());assert_eq!(settings.page(),Some(Page::General));settings.hide()?;owner.show_settings(None)?;assert!(!crate::window::settings::macos::permissions::test_is_open(),"Dismissal survives Settings hide/show");owner.state.desktop_events.request(DesktopEvent::Quit)?;Ok(())}).await.unwrap();
                })));
                if result.is_err(){std::process::exit(1);}
            });
        }).unwrap();
    println!("Native permission guide: explicit-only authorization, duplicate guard, failure/retry, returning grant refresh, optional accessibility, temporary-copy block, recovery navigation and dismissal passed");
}

#[cfg(test)]
pub(crate) fn smoke_test() {
    use snaplingo_core::application::{SettingsApplication, StartOnBoot};
    use std::time::Duration;

    struct Startup;
    impl StartOnBoot for Startup {
        fn set_enabled(&self, _: bool) -> crate::Result<()> {
            Ok(())
        }
    }
    struct Granted;
    impl snaplingo_core::application::RequiredPermissionsHost for Granted {
        fn status(&self) -> snaplingo_core::application::RequiredPermissionsStatus {
            snaplingo_core::application::RequiredPermissionsStatus {
                screen_recording: true,
                accessibility: true,
            }
        }
        fn context(&self) -> snaplingo_core::application::RequiredPermissionsContext {
            snaplingo_core::application::RequiredPermissionsContext {
                platform: "macos".into(),
                app_path: None,
                needs_installation: false,
                can_reset: false,
            }
        }
        fn request(&self, _: snaplingo_core::application::SystemPermission) -> crate::Result<()> {
            panic!("Settings smoke must not request real permissions")
        }
        fn reset(&self, _: snaplingo_core::application::SystemPermission) -> crate::Result<()> {
            panic!("Settings smoke must not reset real permissions")
        }
        fn restart(&self) -> crate::Result<()> {
            Ok(())
        }
    }
    let directory = tempfile::tempdir().unwrap();
    let options = NativeAppOptions {
        data_dir: directory.path().to_path_buf(),
        identifier: "com.snaplingo.native-app-smoke".into(),
        ocr_resource_root: None,
    };
    let state_after = RefCell::new(None);
    run_application(
        options,
        |state| {
            // The isolated application exercises the production lifecycle without
            // changing the user's login item while using a temporary database.
            state.permissions=Arc::new(snaplingo_core::application::RequiredPermissions::new(Arc::new(Granted)));
            let settings = Arc::get_mut(&mut state.settings).unwrap();
            settings.application = Arc::new(SettingsApplication::new(
                settings.configuration.clone(),
                Arc::new(Startup),
                state.history.history.clone(),
                state.logs.repository.clone(),
            ));
        },
        |owner| {
            *state_after.borrow_mut() = Some(owner.state.clone());
            let lifecycle = owner.lifecycle.clone();
            let state = owner.state.clone();
            std::thread::spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    runtime::block_on(async move {
                        smoke_on_main(|| {
                            let owner = current().unwrap();
                            owner.show_settings(Some(SettingsRoute::About))?;
                            let settings = owner.settings.borrow().as_ref().unwrap().clone();
                            assert_eq!(settings.page(), Some(Page::General));
                            assert!(settings.test_window().isVisible());
                            settings
                                .test_window()
                                .setContentSize(objc2_foundation::NSSize::new(1010., 720.));
                            settings.test_window().performClose(None);
                            assert!(
                                !settings.test_window().isVisible(),
                                "Closing Settings must keep its application resident"
                            );
                            assert!(!owner.lifecycle.is_shutdown_complete());
                            assert_eq!(
                                owner
                                    .state
                                    .settings
                                    .configuration
                                    .snapshot()
                                    .unwrap()
                                    .general
                                    .settings_window_width,
                                Some(1010)
                            );
                            owner.show_settings(Some(SettingsRoute::History))?;
                            assert_eq!(settings.page(), Some(Page::History));
                            assert!(settings.test_window().isVisible());
                            Ok(())
                        }).await.unwrap();
                        for page in [Page::Screenshot, Page::Translation, Page::Ocr, Page::Services, Page::Favorites, Page::History, Page::General] {
                            smoke_on_main(move || {
                                let owner = current().unwrap();
                                let settings = owner.settings.borrow().as_ref().unwrap().clone();
                                settings.test_navigate(page);
                                Ok(())
                            }).await.unwrap();
                            let deadline = std::time::Instant::now() + Duration::from_secs(5);
                            loop {
                                let actual = smoke_on_main(|| Ok(current().unwrap().settings.borrow().as_ref().unwrap().page())).await.unwrap();
                                if actual == Some(page) { break; }
                                assert!(std::time::Instant::now() < deadline, "The native window did not route its shared navigation: expected {page:?}, actual {actual:?}");
                                tokio::time::sleep(Duration::from_millis(10)).await;
                            }
                        }
                        smoke_on_main(|| {
                            let owner = current().unwrap();
                            let settings = owner.settings.borrow().as_ref().unwrap().clone();
                            let mut screenshot = owner.state.settings.configuration.snapshot().unwrap().screenshot;
                            screenshot.capture_ui_scale = "175".into();
                            owner.state.settings.configuration.update_screenshot(screenshot).unwrap();
                            let (size, scale) = settings.test_geometry();
                            assert!((scale - 1.75).abs() < 0.001);
                            assert!((size.width * scale - 1010.).abs() < 0.1);
                            owner.show_settings(Some(SettingsRoute::Permissions))?;
                            assert_eq!(settings.page(), Some(Page::General));
                            {
                                use snaplingo_ui::capacity::{CapacityInput as Input, CapacityTarget as Target};
                                let before = owner.state.settings.configuration.snapshot().unwrap().history;
                                settings.test_activate(snaplingo_ui::Target::Capacity);
                                assert!(crate::window::settings::macos::capacity::test_is_open());
                                crate::window::settings::macos::capacity::test_input(Input::SetValue(Target::Days, "999".into()));
                                crate::window::settings::macos::capacity::test_input(Input::Activate(Target::Save));
                                assert!(crate::window::settings::macos::capacity::test_is_open(), "Invalid days must not close or save the sheet");
                                assert_eq!(owner.state.settings.configuration.snapshot().unwrap().history, before);
                                crate::window::settings::macos::capacity::test_input(Input::SetValue(Target::Days, "365".into()));
                                crate::window::settings::macos::capacity::test_input(Input::Activate(Target::Cancel));
                                assert!(!crate::window::settings::macos::capacity::test_is_open());
                                assert_eq!(owner.state.settings.configuration.snapshot().unwrap().history, before);
                                settings.test_activate(snaplingo_ui::Target::Capacity);
                                crate::window::settings::macos::capacity::test_input(Input::SetValue(Target::Days, "40".into()));
                                crate::window::settings::macos::capacity::test_input(Input::Activate(Target::Save));
                                assert!(!crate::window::settings::macos::capacity::test_is_open());
                                assert_eq!(owner.state.settings.configuration.snapshot().unwrap().history.retention_days, 40);
                                settings.test_activate(snaplingo_ui::Target::Capacity);
                                crate::window::settings::macos::capacity::test_input(Input::Activate(Target::Save));
                                assert_eq!(owner.state.settings.configuration.snapshot().unwrap().history.retention_days, 40);
                                settings.test_activate(snaplingo_ui::Target::Capacity);
                                settings.hide()?;
                                assert!(!crate::window::settings::macos::capacity::test_is_open(), "Hiding its parent must cancel the capacity sheet");
                            }
                            owner.state.desktop_events.request(DesktopEvent::Quit)?;
                            Ok(())
                        }).await.unwrap();
                        tokio::time::timeout(Duration::from_secs(8), async {
                            while !lifecycle.is_shutdown_complete() {
                                tokio::task::yield_now().await;
                            }
                        })
                        .await
                        .expect("Native quit did not complete graceful shutdown");
                        assert!(state.history.events.drain(Duration::from_secs(1)).await);
                    });
                }));
                if result.is_err() {
                    std::process::exit(1);
                }
            });
        },
    )
    .unwrap();
    let state = state_after.into_inner().unwrap();
    assert!(state
        .settings
        .configuration
        .snapshot()
        .unwrap()
        .general
        .settings_window_width
        .is_some());
    assert!(current().is_none());
    assert!(
        crate::infrastructure::system::single_instance::SingleInstance::acquire(
            &directory.path().join("instance.lock")
        )
        .unwrap()
        .is_some(),
        "Native application exit must release its instance lock"
    );
    println!("Standalone AppKit application: seven-page pointer navigation, 175% zoom, capacity validation/cancel/save/reopen, close-to-hide, geometry persistence, graceful quit and instance release passed without Tauri");
}

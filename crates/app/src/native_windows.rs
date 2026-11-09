//! Application ownership for the native Windows event loop.
use crate::application::hotkeys::HotkeyRecording;
use crate::infrastructure::system::{shortcut, single_instance::SingleInstance};
use crate::infrastructure::{
    desktop_events::{DesktopEvent, DesktopSubscription, SettingsRoute},
    runtime,
    ui_thread::{self, UiQueue},
};
use crate::native_settings_windows::NativeSettingsWindow;
use crate::{AppState, RuntimeLifecycle};
use snaplingo_desktop::HostEvent;
use snaplingo_ui::{Page, Section};
use std::{cell::RefCell, rc::Rc, sync::Arc};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::WindowId,
};

thread_local! { static ACTIVE: RefCell<Option<Rc<RefCell<Application>>>> = const { RefCell::new(None) }; }
#[cfg(test)]
thread_local! {static SMOKE_READY:RefCell<Option<Box<dyn FnOnce()>>>=const {RefCell::new(None)};}
fn current() -> Option<Rc<RefCell<Application>>> {
    ACTIVE.with_borrow(Clone::clone)
}
struct Application {
    state: Arc<AppState>,
    lifecycle: Option<Arc<RuntimeLifecycle>>,
    recording: HotkeyRecording,
    settings: Option<NativeSettingsWindow>,
    instance: Option<SingleInstance>,
    subscription: Option<DesktopSubscription>,
    restart: bool,
    first_launch: bool,
    failure: Option<String>,
}
impl Application {
    fn show_settings(
        &mut self,
        event_loop: &ActiveEventLoop,
        route: Option<SettingsRoute>,
    ) -> Result<(), String> {
        let page = match route {
            Some(SettingsRoute::History) => Page::History,
            _ => Page::General,
        };
        if self.settings.is_none() {
            self.settings = Some(NativeSettingsWindow::new(
                event_loop,
                ui_thread::event_proxy()?,
                self.state.clone(),
                self.recording.clone(),
                Arc::new(|| {
                    ui_thread::with_event_loop(|event_loop| {
                        if let Some(app) = current() {
                            if let Err(error) = app.borrow_mut().show_notices(event_loop) {
                                log::warn!("Failed to show notices: {error}");
                            }
                        }
                    })
                    .map_err(Into::into)
                }),
                Arc::new(|| {
                    ui_thread::with_event_loop(|event_loop| {
                        if let Some(app) = current() {
                            if let Err(error) = app.borrow_mut().show_capacity(event_loop) {
                                log::warn!("Failed to show capacity: {error}");
                            }
                        }
                    })
                    .map_err(Into::into)
                }),
            )?);
        }
        let settings = self.settings.as_mut().unwrap();
        settings.show(page)?;
        match route {
            Some(SettingsRoute::About) => settings.section(Section::About),
            Some(SettingsRoute::Permissions) => settings.section(Section::Permissions),
            _ => {}
        }
        Ok(())
    }
    fn show_capacity(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        if let Some(settings) = &self.settings {
            crate::native_capacity_windows::show(
                event_loop,
                settings.window_rc(),
                self.state.clone(),
            )?;
        }
        Ok(())
    }
    fn show_notices(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        if let Some(settings) = &self.settings {
            crate::native_notices_windows::show(
                event_loop,
                settings.window_rc(),
                self.state.clone(),
            )?;
        }
        Ok(())
    }
    fn receive(&mut self, event_loop: &ActiveEventLoop, event: DesktopEvent) -> Result<(), String> {
        match event {
            DesktopEvent::OpenSettings => self.show_settings(event_loop, None),
            DesktopEvent::OpenSettingsAt(route) => self.show_settings(event_loop, Some(route)),
            DesktopEvent::Quit | DesktopEvent::Restart => {
                if self
                    .lifecycle
                    .as_ref()
                    .is_some_and(|l| l.request_shutdown())
                {
                    self.restart = matches!(event, DesktopEvent::Restart);
                    self.state.actions.shutdown();
                    if let Some(settings) = &mut self.settings {
                        if let Err(error) = settings.hide() {
                            log::warn!("Failed to save Settings before exit: {error}");
                        }
                    }
                    let lifecycle = self.lifecycle.as_ref().unwrap().clone();
                    let state = self.state.clone();
                    runtime::spawn(async move {
                        if let Err(error) = lifecycle.shutdown(&state).await {
                            log::warn!("Native shutdown failed: {error}");
                        }
                        let _ = ui_thread::with_event_loop(|event_loop| event_loop.exit());
                    });
                }
                Ok(())
            }
            DesktopEvent::HotkeyRecorded {
                window_label,
                recording_id,
                hotkey,
            } if window_label == "settings" => {
                if self
                    .recording
                    .session()
                    .is_some_and(|s| s.id == recording_id && s.window_label == window_label)
                {
                    if let Some(settings) = &mut self.settings {
                        settings.recorded_shortcut(hotkey);
                    }
                }
                Ok(())
            }
            DesktopEvent::HotkeysChanged => crate::app_shell::refresh_menu_bar_shortcuts(
                &self
                    .state
                    .settings
                    .hotkeys
                    .snapshot()
                    .map_err(|e| e.to_string())?,
            ),
            _ => Ok(()),
        }
    }
}
struct Loop {
    owner: Rc<RefCell<Application>>,
    queue: UiQueue,
}
impl ApplicationHandler<HostEvent> for Loop {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.owner.borrow().lifecycle.is_some() {
            return;
        }
        let result = (|| {
            let owner = self.owner.borrow();
            let events = owner.state.desktop_events.clone();
            shortcut::initialize_native_shortcuts(
                owner.recording.clone(),
                |task| ui_thread::with_event_loop(move |_| task()).map_err(Into::into),
                |label| {
                    current().is_some_and(|app| {
                        label == "settings"
                            && app
                                .borrow()
                                .settings
                                .as_ref()
                                .is_some_and(|s| s.window().has_focus())
                    })
                },
                move |label, payload| {
                    events.publish(DesktopEvent::HotkeyRecorded {
                        window_label: label.into(),
                        recording_id: payload.recording_id,
                        hotkey: payload.hotkey,
                    })
                },
            )
            .map_err(|e| e.to_string())?;
            crate::app_shell::initialize_menu_events();
            let events = owner.state.desktop_events.clone();
            crate::app_shell::setup_menu_bar(
                &owner
                    .state
                    .settings
                    .hotkeys
                    .snapshot()
                    .map_err(|e| e.to_string())?,
                move |action| events.publish(DesktopEvent::AppActionRequested(action)),
            )?;
            let lifecycle = Arc::new(RuntimeLifecycle::start(&owner.state));
            let show = crate::should_show_settings_on_startup(
                owner.first_launch,
                owner.state.permissions.status().screen_recording,
            );
            drop(owner);
            self.owner.borrow_mut().lifecycle = Some(lifecycle);
            if show {
                self.owner.borrow_mut().show_settings(event_loop, None)?;
            }
            Ok::<(), String>(())
        })();
        if let Err(error) = result {
            self.owner.borrow_mut().failure = Some(error);
            event_loop.exit();
        }
        #[cfg(test)]
        if self.owner.borrow().failure.is_none() {
            let ready = SMOKE_READY.with_borrow_mut(Option::take);
            if let Some(ready) = ready {
                ready();
            }
        }
    }
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: HostEvent) {
        self.queue.drain(event_loop);
        if let HostEvent::Accessibility(event) = &event {
            crate::infrastructure::system::result_window::windows_accessibility(event);
            crate::infrastructure::system::pinned_window::windows_accessibility(event);
            crate::infrastructure::system::capture_window::windows_accessibility(event);
            crate::infrastructure::system::scrolling_host_windows::windows_accessibility(event);
            crate::native_capacity_windows::accessibility(event);
            crate::native_notices_windows::accessibility(event);
        }
        if let Some(settings) = &mut self.owner.borrow_mut().settings {
            settings.user_event(event_loop, event);
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        crate::infrastructure::system::result_window::windows_window_event(event_loop, id, &event);
        crate::infrastructure::system::pinned_window::windows_window_event(event_loop, id, &event);
        crate::infrastructure::system::capture_window::windows_window_event(event_loop, id, &event);
        crate::infrastructure::system::scrolling_host_windows::windows_window_event(
            event_loop, id, &event,
        );
        crate::native_capacity_windows::window_event(event_loop, id, &event);
        crate::native_notices_windows::window_event(event_loop, id, &event);
        let mut owner = self.owner.borrow_mut();
        if let Some(settings) = &mut owner.settings {
            if settings.window_id() == id && matches!(event, WindowEvent::CloseRequested) {
                crate::native_capacity_windows::close_for(id);
                crate::native_notices_windows::close_for(id);
            }
            settings.window_event(event_loop, id, event);
            if let Some(error) = settings.take_failure() {
                owner.failure = Some(error);
            }
        }
    }
    fn exiting(&mut self, _: &ActiveEventLoop) {
        let mut owner = self.owner.borrow_mut();
        owner.state.actions.shutdown();
        owner.subscription.take();
        if let Some(settings) = owner.settings.take() {
            crate::native_capacity_windows::close_for(settings.window_id());
            crate::native_notices_windows::close_for(settings.window_id());
            let _ = settings.remember_geometry();
        }
        crate::app_shell::shutdown_menu_bar();
        let _ = shortcut::shutdown_shortcuts();
        crate::infrastructure::system::result_window::windows_close_all();
        crate::infrastructure::system::pinned_window::windows_close_all();
        crate::infrastructure::system::capture_window::windows_close_all();
        crate::infrastructure::system::scrolling_host_windows::windows_close_all();
    }
}

pub struct NativeAppOptions {
    pub data_dir: std::path::PathBuf,
    pub identifier: String,
    pub ocr_resource_root: Option<std::path::PathBuf>,
}
pub fn run() -> Result<(), String> {
    run_with_options(NativeAppOptions {
        data_dir: crate::infrastructure::system::get_app_data_dir().map_err(|e| e.to_string())?,
        identifier: "com.snaplingo.app".into(),
        ocr_resource_root: if cfg!(debug_assertions) {
            Some(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/ocr"))
        } else {
            std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(|path| path.join("ocr")))
        },
    })
}
pub fn run_with_options(options: NativeAppOptions) -> Result<(), String> {
    use crate::application::{SelectedTextAcquirer, SelectionScheme, SystemSelectionProvider};
    use crate::infrastructure::storage::{Database, SqliteAppLogRepository};
    std::fs::create_dir_all(&options.data_dir).map_err(|e| e.to_string())?;
    let Some(instance) = SingleInstance::acquire(&options.data_dir.join("instance.lock"))
        .map_err(|e| e.to_string())?
    else {
        return Ok(());
    };
    let path = options.data_dir.join("snaplingo.db");
    let first_launch = !path.exists();
    let database = Arc::new(Database::open(&path).map_err(|e| e.to_string())?);
    crate::infrastructure::logging::initialize(SqliteAppLogRepository::new(database.clone()))
        .map_err(|e| e.to_string())?;
    let provider =
        Arc::new(crate::infrastructure::system::selection::windows::PlatformSelectionProvider);
    let state = crate::composition::build_app_state(
        database,
        path,
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
    run_state(Arc::new(state), instance, first_launch)
}
pub(crate) fn run_state(
    state: Arc<AppState>,
    instance: SingleInstance,
    first_launch: bool,
) -> Result<(), String> {
    let event_loop = EventLoop::<HostEvent>::with_user_event()
        .build()
        .map_err(|e| e.to_string())?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let queue = UiQueue::install(event_loop.create_proxy())?;
    let owner = Rc::new(RefCell::new(Application {
        state,
        lifecycle: None,
        recording: Default::default(),
        settings: None,
        instance: Some(instance),
        subscription: None,
        restart: false,
        first_launch,
        failure: None,
    }));
    let subscription = owner
        .borrow()
        .state
        .desktop_events
        .connect_controller(move |event| {
            let event = event.clone();
            ui_thread::with_event_loop(move |event_loop| {
                if let Some(app) = current() {
                    if let Err(error) = app.borrow_mut().receive(event_loop, event) {
                        log::warn!("Failed to route native desktop event: {error}");
                    }
                }
            })
        });
    owner.borrow_mut().subscription = Some(subscription);
    owner.borrow().instance.as_ref().unwrap().on_activation(|| {
        let _ = ui_thread::with_event_loop(|event_loop| {
            if let Some(app) = current() {
                let _ = app.borrow_mut().show_settings(event_loop, None);
            }
        });
    });
    ACTIVE.with_borrow_mut(|active| *active = Some(owner.clone()));
    let result = event_loop
        .run_app(&mut Loop {
            owner: owner.clone(),
            queue,
        })
        .map_err(|e| e.to_string());
    ACTIVE.with_borrow_mut(Option::take);
    // Startup/window failures also drain history before releasing the process
    // lock. Successful Quit has already completed this same idempotent path.
    let (lifecycle, state) = {
        let owner = owner.borrow();
        (owner.lifecycle.clone(), owner.state.clone())
    };
    if let Some(lifecycle) = lifecycle {
        if let Err(error) = runtime::block_on(lifecycle.shutdown(&state)) {
            log::warn!("Failed to finish native shutdown: {error}");
        }
    }
    let mut owner = owner.borrow_mut();
    owner.instance.take();
    result?;
    if let Some(error) = owner.failure.take() {
        return Err(error);
    }
    if owner.restart {
        std::process::Command::new(std::env::current_exe().map_err(|e| e.to_string())?)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
async fn smoke_on_ui<T: Send + 'static>(
    work: impl FnOnce(&ActiveEventLoop) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    ui_thread::with_event_loop(move |event_loop| {
        let _ = sender.send(std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            || work(event_loop),
        )));
    })?;
    match tokio::time::timeout(std::time::Duration::from_secs(15), receiver)
        .await
        .expect("Windows UI queue stalled")
        .unwrap()
    {
        Ok(result) => result,
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

#[cfg(test)]
pub(crate) fn smoke_test() {
    use crate::application::{
        SelectedTextAcquirer, SelectionScheme, SettingsApplication, StartOnBoot,
        SystemSelectionProvider,
    };
    use crate::infrastructure::storage::{Database, SqliteAppLogRepository};
    use snaplingo_ui::Event;
    use std::time::{Duration, Instant};
    struct Startup;
    impl StartOnBoot for Startup {
        fn set_enabled(&self, _: bool) -> crate::Result<()> {
            Ok(())
        }
    }
    let directory = tempfile::tempdir().unwrap();
    let lock_path = directory.path().join("instance.lock");
    let instance = SingleInstance::acquire(&lock_path).unwrap().unwrap();
    let database_path = directory.path().join("snaplingo.db");
    let database = Arc::new(Database::open(&database_path).unwrap());
    crate::infrastructure::logging::initialize(SqliteAppLogRepository::new(database.clone()))
        .unwrap();
    let provider =
        Arc::new(crate::infrastructure::system::selection::windows::PlatformSelectionProvider);
    let mut state = crate::composition::build_app_state(
        database,
        database_path,
        crate::composition::CompositionOptions {
            identifier: "com.snaplingo.native-windows-smoke".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            ocr_resource_root: None,
            cache_dir: Ok(directory.path().join("cache")),
            selected_text: Arc::new(SelectedTextAcquirer::new(
                SelectionScheme::new(provider.default_scheme()),
                provider.methods(),
                provider,
            )),
        },
    );
    let settings = Arc::get_mut(&mut state.settings).unwrap();
    settings.application = Arc::new(SettingsApplication::new(
        settings.configuration.clone(),
        Arc::new(Startup),
        state.history.history.clone(),
        state.logs.repository.clone(),
    ));
    let state = Arc::new(state);
    let after = state.clone();
    let active_lock = lock_path.clone();
    SMOKE_READY.with_borrow_mut(|ready| {
        *ready = Some(Box::new(move || {
            let owner = current().unwrap();
            let state = owner.borrow().state.clone();
            let lifecycle = owner.borrow().lifecycle.clone().unwrap();
            std::thread::spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    runtime::block_on(async move {
                        assert!(
                            SingleInstance::acquire(&active_lock).unwrap().is_none(),
                            "A second Windows launch must activate the resident process"
                        );
                        smoke_on_ui(|event_loop| {
                            let owner = current().unwrap();
                            let mut app = owner.borrow_mut();
                            app.show_settings(event_loop, Some(SettingsRoute::About))?;
                            let settings = app.settings.as_mut().unwrap();
                            assert_eq!(settings.test_page(), Page::General);
                            assert_eq!(settings.window().is_visible(), Some(true));
                            settings.window_event(
                                event_loop,
                                settings.window_id(),
                                WindowEvent::CloseRequested,
                            );
                            assert_eq!(
                                settings.window().is_visible(),
                                Some(false),
                                "Closing Settings must keep Windows resident"
                            );
                            assert!(!app.lifecycle.as_ref().unwrap().is_shutdown_complete());
                            app.show_settings(event_loop, Some(SettingsRoute::History))?;
                            assert_eq!(app.settings.as_ref().unwrap().test_page(), Page::History);
                            Ok(())
                        })
                        .await
                        .unwrap();
                        for page in [
                            Page::Screenshot,
                            Page::Translation,
                            Page::Ocr,
                            Page::Services,
                            Page::Favorites,
                            Page::History,
                            Page::General,
                        ] {
                            smoke_on_ui(move |_| {
                                current()
                                    .unwrap()
                                    .borrow_mut()
                                    .settings
                                    .as_mut()
                                    .unwrap()
                                    .test_navigate(page)
                            })
                            .await
                            .unwrap();
                            let deadline = Instant::now() + Duration::from_secs(5);
                            loop {
                                let actual = smoke_on_ui(|_| {
                                    Ok(current()
                                        .unwrap()
                                        .borrow()
                                        .settings
                                        .as_ref()
                                        .unwrap()
                                        .test_page())
                                })
                                .await
                                .unwrap();
                                if actual == page {
                                    break;
                                }
                                assert!(
                                    Instant::now() < deadline,
                                    "Windows native page did not switch to {page:?}"
                                );
                                tokio::time::sleep(Duration::from_millis(10)).await;
                            }
                        }
                        smoke_on_ui(|_| {
                            let owner = current().unwrap();
                            let app = owner.borrow();
                            let _ = app
                                .settings
                                .as_ref()
                                .unwrap()
                                .window()
                                .request_inner_size(winit::dpi::LogicalSize::new(1010., 720.));
                            Ok(())
                        })
                        .await
                        .unwrap();
                        let deadline = Instant::now() + Duration::from_secs(5);
                        loop {
                            let resized = smoke_on_ui(|_| {
                                let owner = current().unwrap();
                                let app = owner.borrow();
                                let window = app.settings.as_ref().unwrap().window();
                                let size =
                                    window.inner_size().to_logical::<f64>(window.scale_factor());
                                Ok((size.width - 1010.).abs() < 1.
                                    && (size.height - 720.).abs() < 1.)
                            })
                            .await
                            .unwrap();
                            if resized {
                                break;
                            }
                            assert!(Instant::now() < deadline, "Windows Settings did not resize");
                            tokio::time::sleep(Duration::from_millis(10)).await;
                        }
                        smoke_on_ui(|_| {
                            let owner = current().unwrap();
                            let mut app = owner.borrow_mut();
                            let mut screenshot = app
                                .state
                                .settings
                                .configuration
                                .snapshot()
                                .unwrap()
                                .screenshot;
                            screenshot.capture_ui_scale = "175".into();
                            app.state
                                .settings
                                .configuration
                                .update_screenshot(screenshot)
                                .unwrap();
                            let settings = app.settings.as_mut().unwrap();
                            assert!((settings.test_interface_scale() - 1.75).abs() < 0.001);
                            settings.test_input(Event::Activate(snaplingo_ui::Target::Capacity));
                            Ok(())
                        })
                        .await
                        .unwrap();
                        smoke_on_ui(|_| {
                            use snaplingo_ui::capacity::{
                                CapacityInput as Input, CapacityTarget as Target,
                            };
                            let owner = current().unwrap();
                            let app = owner.borrow();
                            let before =
                                app.state.settings.configuration.snapshot().unwrap().history;
                            assert!(crate::native_capacity_windows::test_is_open());
                            crate::native_capacity_windows::test_input(Input::SetValue(
                                Target::Days,
                                "999".into(),
                            ));
                            crate::native_capacity_windows::test_input(Input::Activate(
                                Target::Save,
                            ));
                            assert!(crate::native_capacity_windows::test_is_open());
                            assert_eq!(
                                app.state.settings.configuration.snapshot().unwrap().history,
                                before
                            );
                            crate::native_capacity_windows::test_input(Input::SetValue(
                                Target::Days,
                                "365".into(),
                            ));
                            crate::native_capacity_windows::test_input(Input::Activate(
                                Target::Cancel,
                            ));
                            assert!(!crate::native_capacity_windows::test_is_open());
                            assert_eq!(
                                app.state.settings.configuration.snapshot().unwrap().history,
                                before
                            );
                            Ok(())
                        })
                        .await
                        .unwrap();
                        smoke_on_ui(|_| {
                            current()
                                .unwrap()
                                .borrow_mut()
                                .settings
                                .as_mut()
                                .unwrap()
                                .test_input(Event::Activate(snaplingo_ui::Target::Capacity));
                            Ok(())
                        })
                        .await
                        .unwrap();
                        smoke_on_ui(|_| {
                            use snaplingo_ui::capacity::{
                                CapacityInput as Input, CapacityTarget as Target,
                            };
                            crate::native_capacity_windows::test_input(Input::SetValue(
                                Target::Days,
                                "40".into(),
                            ));
                            crate::native_capacity_windows::test_input(Input::Activate(
                                Target::Save,
                            ));
                            assert!(!crate::native_capacity_windows::test_is_open());
                            assert_eq!(
                                current()
                                    .unwrap()
                                    .borrow()
                                    .state
                                    .settings
                                    .configuration
                                    .snapshot()
                                    .unwrap()
                                    .history
                                    .retention_days,
                                40
                            );
                            current()
                                .unwrap()
                                .borrow_mut()
                                .settings
                                .as_mut()
                                .unwrap()
                                .test_input(Event::Activate(snaplingo_ui::Target::Capacity));
                            Ok(())
                        })
                        .await
                        .unwrap();
                        smoke_on_ui(|_| {
                            use snaplingo_ui::capacity::{CapacityInput as Input, CapacityTarget};
                            assert!(crate::native_capacity_windows::test_is_open());
                            crate::native_capacity_windows::test_input(Input::SetValue(
                                CapacityTarget::Days,
                                "100".into(),
                            ));
                            current()
                                .unwrap()
                                .borrow_mut()
                                .settings
                                .as_mut()
                                .unwrap()
                                .hide()?;
                            assert!(
                                !crate::native_capacity_windows::test_is_open(),
                                "Hiding Settings must cancel its capacity draft"
                            );
                            assert_eq!(
                                current()
                                    .unwrap()
                                    .borrow()
                                    .state
                                    .settings
                                    .configuration
                                    .snapshot()
                                    .unwrap()
                                    .history
                                    .retention_days,
                                40
                            );
                            current()
                                .unwrap()
                                .borrow()
                                .state
                                .desktop_events
                                .request(DesktopEvent::Quit)?;
                            Ok(())
                        })
                        .await
                        .unwrap();
                        tokio::time::timeout(Duration::from_secs(10), async {
                            while !lifecycle.is_shutdown_complete() {
                                tokio::task::yield_now().await;
                            }
                        })
                        .await
                        .expect("Windows quit did not drain runtime work");
                        assert!(state.history.events.drain(Duration::from_secs(1)).await);
                    })
                }));
                if result.is_err() {
                    std::process::exit(1);
                }
            });
        }))
    });
    run_state(state, instance, true).unwrap();
    assert!(current().is_none());
    assert_eq!(
        after
            .settings
            .configuration
            .snapshot()
            .unwrap()
            .history
            .retention_days,
        40
    );
    assert!(after
        .settings
        .configuration
        .snapshot()
        .unwrap()
        .general
        .settings_window_width
        .is_some());
    assert!(
        SingleInstance::acquire(&lock_path).unwrap().is_some(),
        "Windows exit must release its instance lock after shutdown"
    );
    println!("Standalone Windows application: shared seven-page navigation, 175% zoom, capacity validation/cancel/save, close-to-hide, geometry persistence, graceful quit and instance release passed without Tauri");
}

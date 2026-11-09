// Real AppKit preferences controls and Rust services; no Tauri runtime is initialized.
#![allow(dead_code, unused_imports, ambiguous_glob_reexports)]

#[cfg(target_os = "macos")]
#[path = "../src/app_actions.rs"]
mod app_actions;
#[cfg(target_os = "macos")]
#[path = "../src/app_shell.rs"]
mod app_shell;
#[cfg(target_os = "macos")]
#[path = "../src/app_state.rs"]
mod app_state;
#[cfg(target_os = "macos")]
use snaplingo_core::application;
#[cfg(target_os = "macos")]
mod composition {
    include!(concat!(env!("OUT_DIR"), "/native_ui_composition.rs"));
}
#[cfg(target_os = "macos")]
use snaplingo_core::domain;
#[cfg(target_os = "macos")]
use snaplingo_core::error;
#[cfg(target_os = "macos")]
#[path = "../src/infrastructure/mod.rs"]
mod infrastructure;
#[cfg(target_os = "macos")]
#[path = "../src/startup_shortcuts.rs"]
mod startup_shortcuts;
#[cfg(target_os = "macos")]
pub use app_state::AppState;
#[cfg(target_os = "macos")]
pub use application::*;
#[cfg(target_os = "macos")]
pub use domain::*;
#[cfg(target_os = "macos")]
pub use error::{AppError, Result};
#[cfg(target_os = "macos")]
pub use infrastructure::*;

#[cfg(target_os = "macos")]
struct LivePage {
    page: std::rc::Rc<infrastructure::system::settings_window::GeneralPage>,
    window: objc2::rc::Retained<objc2_app_kit::NSWindow>,
}
#[cfg(target_os = "macos")]
thread_local! {
    static LIVE: std::cell::RefCell<Option<LivePage>> = const { std::cell::RefCell::new(None) };
    static DRAFT: std::cell::RefCell<Option<objc2::rc::Retained<objc2_app_kit::NSText>>> = const { std::cell::RefCell::new(None) };
}

#[cfg(target_os = "macos")]
async fn on_main<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    infrastructure::ui_thread::dispatch(move || {
        let _ = sender.send(std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)));
    });
    match receiver.await.unwrap() {
        Ok(value) => value,
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

#[cfg(target_os = "macos")]
async fn wait_for_ui(ready: impl Fn() -> bool + Send + Sync + 'static) {
    let ready = std::sync::Arc::new(ready);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let check = ready.clone();
        if on_main(move || check()).await {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "AppKit settings update did not arrive"
        );
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
}

#[cfg(target_os = "macos")]
fn main() {
    use crate::application::settings::{GeneralField, GeneralSection, GeneralValue};
    use crate::infrastructure::system::settings_window::{
        GeneralAction, GeneralController, GeneralPage, GeneralServices, UpdatePhase,
    };
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSBackingStoreType, NSView, NSWindow, NSWindowStyleMask};
    use objc2_foundation::{NSPoint, NSRect, NSSize};
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct Boot {
        calls: Mutex<Vec<bool>>,
        fail: std::sync::atomic::AtomicBool,
    }
    impl StartOnBoot for Boot {
        fn set_enabled(&self, enabled: bool) -> crate::Result<()> {
            self.calls.lock().unwrap().push(enabled);
            if self.fail.load(std::sync::atomic::Ordering::SeqCst) {
                Err("fixture startup registration failed".into())
            } else {
                Ok(())
            }
        }
    }
    struct Permissions;
    impl RequiredPermissionsHost for Permissions {
        fn status(&self) -> RequiredPermissionsStatus {
            RequiredPermissionsStatus {
                screen_recording: true,
                accessibility: true,
            }
        }
        fn context(&self) -> RequiredPermissionsContext {
            RequiredPermissionsContext {
                platform: "macos".into(),
                app_path: None,
                needs_installation: false,
                can_reset: true,
            }
        }
        fn request(&self, _: SystemPermission) -> crate::Result<()> {
            Ok(())
        }
        fn reset(&self, _: SystemPermission) -> crate::Result<()> {
            Ok(())
        }
        fn restart(&self) -> crate::Result<()> {
            Ok(())
        }
    }
    struct Updates {
        checks: std::sync::atomic::AtomicUsize,
        release: tokio::sync::Notify,
    }
    #[async_trait::async_trait]
    impl application::updates::AppUpdateHost for Updates {
        async fn latest_release(
            &self,
        ) -> crate::Result<Option<application::updates::PublishedRelease>> {
            if self
                .checks
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                == 0
            {
                self.release.notified().await;
            }
            Ok(None)
        }
        async fn download_and_open(
            &self,
            _: &application::updates::UpdateInstaller,
        ) -> crate::Result<String> {
            Err("No installer expected in this fixture".into())
        }
        fn open_release_page(&self) -> crate::Result<()> {
            Ok(())
        }
    }
    let mtm = MainThreadMarker::new().unwrap();
    let app = NSApplication::sharedApplication(mtm);
    app.finishLaunching();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings.db");
    let selection = Arc::new(
        infrastructure::system::selection::macos::MacSelectionProvider::new(Some(
            "com.snaplingo.native-settings-test".into(),
        )),
    );
    let state = composition::build_app_state(
        Arc::new(infrastructure::storage::Database::open(&path).unwrap()),
        path,
        composition::CompositionOptions {
            identifier: "com.snaplingo.native-settings-test".into(),
            version: "0.1.23".into(),
            ocr_resource_root: None,
            cache_dir: Ok(directory.path().join("cache")),
            selected_text: Arc::new(SelectedTextAcquirer::new(
                SelectionScheme::new(selection.default_scheme()),
                selection.methods(),
                selection,
            )),
        },
    );
    let configuration = state.settings.configuration.clone();
    let boot = Arc::new(Boot::default());
    let settings = Arc::new(SettingsApplication::new(
        configuration.clone(),
        boot.clone(),
        state.history.history.clone(),
        state.logs.repository.clone(),
    ));
    let updates = Arc::new(Updates {
        checks: std::sync::atomic::AtomicUsize::new(0),
        release: tokio::sync::Notify::new(),
    });
    let app_updates = Arc::new(application::updates::AppUpdates::new(
        updates.clone(),
        "0.1.23".into(),
        "macos".into(),
        "aarch64".into(),
    ));
    let controller = Arc::new(GeneralController::new(GeneralServices {
        configuration: configuration.clone(),
        settings: settings.clone(),
        history: state.history.history.clone(),
        permissions: Arc::new(RequiredPermissions::new(Arc::new(Permissions))),
        updates: app_updates.clone(),
        version: "0.1.23".into(),
        open_url: Arc::new(|_| Ok(())),
        open_notices: Arc::new(|| Ok(())),
    }));
    let page = GeneralPage::new(mtm, controller.clone(), &state.desktop_events).unwrap();
    let view = page.view();
    let window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            mtm.alloc(),
            NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(698.0, 650.0)),
            NSWindowStyleMask::Titled | NSWindowStyleMask::Resizable,
            NSBackingStoreType::Buffered,
            false,
        )
    };
    unsafe {
        window.setReleasedWhenClosed(false);
    }
    window.setContentView(Some(view));
    assert!(view.test_control_count() >= 20);
    view.test_toggle(GeneralField::StartOnBoot);
    assert!(configuration.snapshot().unwrap().general.start_on_boot);
    assert_eq!(*boot.calls.lock().unwrap(), vec![true]);
    boot.fail.store(true, std::sync::atomic::Ordering::SeqCst);
    view.test_toggle(GeneralField::StartOnBoot);
    assert!(
        configuration.snapshot().unwrap().general.start_on_boot,
        "Failed system registration must not commit the setting"
    );
    boot.fail.store(false, std::sync::atomic::Ordering::SeqCst);
    view.test_choose(GeneralField::ProxyMode, "manual");
    view.test_input(GeneralField::ProxyUrl, "  http://127.0.0.1:7890  ");
    view.test_input(GeneralField::RequestTimeout, "25000");
    view.test_input(GeneralField::RetryCount, "4");
    view.test_choose(GeneralField::LogLevel, "debug");
    view.test_input(GeneralField::LogRetention, "20");
    view.test_input(GeneralField::RequestTimeout, "invalid");
    view.test_toggle(GeneralField::GpuComposition);
    view.test_toggle(GeneralField::PerformanceMonitoring);
    let saved = configuration.snapshot().unwrap();
    assert_eq!(saved.general.proxy_url, "http://127.0.0.1:7890");
    assert_eq!(saved.general.request_timeout_ms, 25000);
    assert_eq!(saved.general.retry_count, 4);
    assert_eq!(saved.general.log_level, "debug");
    assert_eq!(saved.general.log_retention_days, 20);
    assert!(saved.general.experimental_gpu_acceleration && saved.general.performance_monitoring);
    let mut background = saved.general.clone();
    background.system_tts_rate = 210;
    background.last_result_window_x = Some(-320);
    configuration.update_general(background).unwrap();
    view.test_choose(GeneralField::Appearance, "dark");
    assert_eq!(
        configuration.snapshot().unwrap().general.system_tts_rate,
        210
    );
    assert_eq!(
        configuration
            .snapshot()
            .unwrap()
            .general
            .last_result_window_x,
        Some(-320)
    );
    let before = configuration.snapshot().unwrap();
    assert!(settings
        .edit_general(GeneralField::RequestTimeout, GeneralValue::Toggle(true))
        .is_err());
    assert_eq!(configuration.snapshot().unwrap(), before);
    let artifacts =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/native-settings");
    std::fs::create_dir_all(&artifacts).unwrap();
    view.test_choose(GeneralField::Appearance, "light");
    for (theme, width, height) in [
        ("native", 698.0, 650.0),
        ("studio", 832.0, 650.0),
        ("paper", 900.0, 582.0),
    ] {
        view.test_theme(theme);
        window.setContentSize(NSSize::new(width, height));
        view.setFrameSize(NSSize::new(width, height));
        assert_eq!(
            configuration.snapshot().unwrap().general.interface_theme,
            theme
        );
        view.jump(GeneralSection::Interface);
        window.displayIfNeeded();
        let png = infrastructure::system::native_view_snapshot::snapshot(view);
        let image = image::load_from_memory(&png).unwrap();
        assert!(image.width() >= width as u32 && image.height() >= height as u32);
        std::fs::write(artifacts.join(format!("{theme}.png")), png).unwrap();
        std::fs::write(
            artifacts.join(format!("{theme}.json")),
            serde_json::to_vec_pretty(&view.test_geometry()).unwrap(),
        )
        .unwrap();
        let geometry = view.test_geometry();
        let (expected_x, expected_y, expected_width) = match theme {
            "studio" => (161.0, 135.0, 641.0),
            "paper" => (161.0, 139.0, 709.0),
            _ => (22.0, 161.0, 646.0),
        };
        for (key, expected) in [
            ("x", expected_x),
            ("y", expected_y),
            ("width", expected_width),
        ] {
            assert!(
                (geometry["card"][key].as_f64().unwrap() - expected).abs() < 1.0,
                "{theme} first card {key} differs from original React reference: {geometry}"
            );
        }
        view.jump(GeneralSection::Network);
        window.displayIfNeeded();
        std::fs::write(
            artifacts.join(format!("{theme}-network.png")),
            infrastructure::system::native_view_snapshot::snapshot(view),
        )
        .unwrap();
        for section in [
            GeneralSection::Permissions,
            GeneralSection::Maintenance,
            GeneralSection::Experimental,
            GeneralSection::About,
        ] {
            view.jump(section);
            window.displayIfNeeded();
            std::fs::write(
                artifacts.join(format!("{theme}-{}.png", section.id())),
                infrastructure::system::native_view_snapshot::snapshot(view),
            )
            .unwrap();
        }
        let original = view.test_projection();
        let mut repair = original.clone();
        repair.permissions = Some(RequiredPermissionsStatus {
            screen_recording: false,
            accessibility: false,
        });
        repair.permission_context.as_mut().unwrap().app_path =
            Some("/Applications/SnapLingo.app".into());
        view.refresh(repair);
        view.jump(GeneralSection::Permissions);
        view.test_disclosure(false);
        std::fs::write(
            artifacts.join(format!("{theme}-permissions-recovery.png")),
            infrastructure::system::native_view_snapshot::snapshot(view),
        )
        .unwrap();
        view.test_disclosure(false);
        let mut update = original.clone();
        update.update_phase = UpdatePhase::Idle;
        update.downloaded_path = Some("/tmp/SnapLingo fixture.dmg".into());
        update.update = Some(application::updates::AppUpdateStatus {
            current_version: "0.1.23".into(),
            latest_version: Some("0.1.24".into()),
            available: true,
            download_size: Some(2_100_000),
            notes: "新版本更新说明\n• 修复截图窗口与滚动截图交互\n• 改善识别文本和翻译体验".into(),
            published_at: None,
            platform: "macos".into(),
        });
        view.refresh(update);
        view.jump(GeneralSection::About);
        view.test_disclosure(true);
        std::fs::write(
            artifacts.join(format!("{theme}-about-update.png")),
            infrastructure::system::native_view_snapshot::snapshot(view),
        )
        .unwrap();
        view.test_disclosure(true);
        let mut logs = original.clone();
        logs.logs = (0..8)
            .map(|i| application::settings::AppLogEntry {
                id: i,
                timestamp: "2026-11-08T00:00:00Z".into(),
                level: "INFO".into(),
                target: "native-fixture".into(),
                message: format!("Fixture log entry {i}: native preferences state update"),
            })
            .collect();
        view.refresh(logs);
        view.jump(GeneralSection::Maintenance);
        std::fs::write(
            artifacts.join(format!("{theme}-maintenance-logs.png")),
            infrastructure::system::native_view_snapshot::snapshot(view),
        )
        .unwrap();
        view.refresh(original);
    }
    let lifetime = Arc::downgrade(&controller);
    drop(controller);
    LIVE.with_borrow_mut(|live| *live = Some(LivePage { page, window }));
    let worker = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            runtime.block_on(async {
                on_main(|| LIVE.with_borrow(|live| live.as_ref().unwrap().page.ensure_checked()))
                    .await;
                wait_for_ui(|| {
                    LIVE.with_borrow(|live| {
                        live.as_ref()
                            .unwrap()
                            .page
                            .view()
                            .test_projection()
                            .update_phase
                            == UpdatePhase::Checking
                    })
                })
                .await;
                on_main(|| {
                    LIVE.with_borrow(|live| {
                        let live = live.as_ref().unwrap();
                        live.page.view().jump(GeneralSection::Network);
                        let editor = live
                            .page
                            .view()
                            .test_begin_edit(GeneralField::ProxyUrl, "http://127.0.0.1:9090");
                        DRAFT.with_borrow_mut(|draft| *draft = Some(editor));
                    })
                })
                .await;
                updates.release.notify_one();
                wait_for_ui(|| {
                    LIVE.with_borrow(|live| {
                        let projection = live.as_ref().unwrap().page.view().test_projection();
                        projection.update.is_some() && projection.update_phase == UpdatePhase::Idle
                    })
                })
                .await;
                on_main(|| {
                    LIVE.with_borrow(|live| {
                        let live = live.as_ref().unwrap();
                        assert!(
                            live.page.view().test_has_editor(),
                            "Async update replaced an active field editor"
                        );
                        DRAFT.with_borrow(|draft| {
                            let editor = draft.as_ref().unwrap();
                            assert_eq!(editor.string().to_string(), "http://127.0.0.1:9090");
                            assert_eq!(
                                editor.selectedRange(),
                                objc2_foundation::NSRange::new(3, 5)
                            );
                            assert_eq!(
                                objc2::rc::Retained::as_ptr(
                                    live.window.firstResponder().as_ref().unwrap()
                                )
                                .cast::<std::ffi::c_void>(),
                                objc2::rc::Retained::as_ptr(editor).cast::<std::ffi::c_void>()
                            );
                        });
                        live.window.makeFirstResponder(None);
                        DRAFT.with_borrow_mut(Option::take);
                    })
                })
                .await;
                let committed = configuration.clone();
                wait_for_ui(move || {
                    committed.snapshot().unwrap().general.proxy_url == "http://127.0.0.1:9090"
                })
                .await;
                on_main(|| {
                    LIVE.with_borrow(|live| {
                        let view = live.as_ref().unwrap().page.view();
                        assert!(!view.test_has_editor());
                        assert!(view.test_projection().update.is_some());
                        view.test_scroll_to(view.test_section_offset(GeneralSection::Network));
                        assert_eq!(view.test_active_section(), GeneralSection::Network);
                        view.test_scroll_to(0.0);
                        assert_eq!(view.test_active_section(), GeneralSection::Interface);
                    })
                })
                .await;
                on_main(|| LIVE.with_borrow(|live| live.as_ref().unwrap().page.ensure_checked()))
                    .await;
                assert_eq!(updates.checks.load(std::sync::atomic::Ordering::SeqCst), 1);
                let background = configuration.clone();
                on_main(move || {
                    let worker = std::thread::spawn(move || {
                        let mut general = background.snapshot().unwrap().general;
                        general.log_retention_days = 33;
                        background.update_general(general).unwrap();
                    });
                    worker.join().unwrap();
                    // Main queue notifications are pending while this latest edit commits.
                    LIVE.with_borrow(|live| {
                        live.as_ref()
                            .unwrap()
                            .page
                            .view()
                            .test_choose(GeneralField::Appearance, "dark")
                    });
                })
                .await;
                wait_for_ui(|| {
                    LIVE.with_borrow(|live| {
                        live.as_ref()
                            .unwrap()
                            .page
                            .view()
                            .test_projection()
                            .general
                            .log_retention_days
                            == 33
                    })
                })
                .await;
                on_main(move || {
                    LIVE.with_borrow(|live| {
                        let live = live.as_ref().unwrap();
                        assert_eq!(live.page.view().test_projection().general.theme, "dark");
                        live.window.setContentSize(NSSize::new(900.0, 650.0));
                        for (theme, appearance) in [("native", "light"), ("paper", "dark")] {
                            let mut general = configuration.snapshot().unwrap().general;
                            general.interface_theme = theme.into();
                            general.theme = appearance.into();
                            infrastructure::system::settings_window::test_notices_workflow(
                                &live.window,
                                &general,
                                &artifacts.join(format!("{theme}-notices.png")),
                            );
                        }
                        live.page.view().test_action(GeneralAction::RefreshLogs);
                    })
                })
                .await;
                let configuration = state.settings.configuration.clone();
                let events = state.desktop_events.clone();
                let settings = state.settings.application.clone();
                let history = state.history.history.clone();
                let permissions = state.permissions.clone();
                let updates = app_updates.clone();
                on_main(move || {
                    use infrastructure::system::settings_window::native_settings_host as host;
                    let mtm = MainThreadMarker::new().unwrap();
                    let window = unsafe {
                        NSWindow::initWithContentRect_styleMask_backing_defer(
                            mtm.alloc(),
                            NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(900.0, 650.0)),
                            NSWindowStyleMask::Titled,
                            NSBackingStoreType::Buffered,
                            false,
                        )
                    };
                    unsafe {
                        window.setReleasedWhenClosed(false);
                    }
                    let root = NSView::initWithFrame(
                        mtm.alloc(),
                        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(900.0, 650.0)),
                    );
                    window.setContentView(Some(&root));
                    let controller = Arc::new(GeneralController::new(GeneralServices {
                        configuration,
                        settings,
                        history,
                        permissions,
                        updates,
                        version: "0.1.23".into(),
                        open_url: Arc::new(|_| Ok(())),
                        open_notices: Arc::new(|| Ok(())),
                    }));
                    let page = GeneralPage::new(mtm, controller, &events).unwrap();
                    let viewport = host::SettingsViewport {
                        x: 202.0,
                        y: 0.0,
                        width: 698.0,
                        height: 650.0,
                        scale: 1.0,
                    };
                    host::present(&root, "first".into(), viewport, Some("about"), || {
                        Ok(page.into())
                    })
                    .unwrap();
                    let original = host::test_mounted_view().unwrap();
                    assert_eq!(original.frame().origin.x, 202.0);
                    assert_eq!(original.bounds().size.width, 698.0);
                    host::present(
                        &root,
                        "first".into(),
                        host::SettingsViewport {
                            scale: 1.25,
                            ..viewport
                        },
                        None,
                        || panic!("Resize recreated General preferences"),
                    )
                    .unwrap();
                    assert_eq!(
                        host::test_mounted_view().unwrap().bounds().size.width,
                        558.4
                    );
                    host::dismiss("late-old-page");
                    assert!(host::test_mounted_view().is_some());
                    host::dismiss("first");
                    assert!(host::test_mounted_view().is_none());
                    assert!(unsafe { original.superview() }.is_none());
                    window.close();
                })
                .await;
                on_main(|| {
                    LIVE.with_borrow_mut(|live| {
                        let live = live.take().unwrap();
                        live.window.close();
                        live.window.setContentView(None);
                    })
                })
                .await;
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
                while lifetime.upgrade().is_some() {
                    assert!(
                        std::time::Instant::now() < deadline,
                        "Disposed General page retained its controller"
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                }
                state
                    .desktop_events
                    .publish(infrastructure::desktop_events::DesktopEvent::SettingsChanged);
            })
        }));
        infrastructure::ui_thread::dispatch(|| {
            use objc2_app_kit::{NSEvent, NSEventModifierFlags, NSEventType};
            let app = NSApplication::sharedApplication(MainThreadMarker::new().unwrap());
            app.stop(None);
            let wake = NSEvent::otherEventWithType_location_modifierFlags_timestamp_windowNumber_context_subtype_data1_data2(NSEventType::ApplicationDefined, NSPoint::new(0.0,0.0), NSEventModifierFlags::empty(), 0.0, 0, None, 0, 0, 0).unwrap();
            app.postEvent_atStart(&wake, true);
        });
        if let Err(panic) = result {
            std::panic::resume_unwind(panic);
        }
    });
    app.run();
    worker.join().unwrap();
    println!("Native General preferences: real AppKit controls, service actions, latest settings, startup rollback, invalid input, automatic update phases, worker notifications, disposal and six theme snapshots passed without Tauri initialization");
}

#[cfg(not(target_os = "macos"))]
fn main() {}

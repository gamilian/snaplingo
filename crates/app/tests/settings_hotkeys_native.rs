// Real AppKit preferences controls and Rust services; no Tauri runtime is initialized.
#![allow(dead_code, unused_imports, ambiguous_glob_reexports)]

#[cfg(target_os = "macos")]
#[path = "../actions/mod.rs"]
mod actions;
#[cfg(target_os = "macos")]
use lifecycle::shell;
#[cfg(target_os = "macos")]
#[path = "../app_state.rs"]
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
#[path = "../infrastructure/mod.rs"]
mod infrastructure;
#[cfg(target_os = "macos")]
use actions::shortcuts;
#[cfg(target_os = "macos")]
#[path = "../window/mod.rs"]
mod window;
#[cfg(target_os = "macos")]
pub use app_state::AppState;
#[cfg(target_os = "macos")]
pub use error::{AppError, Result};
#[cfg(target_os = "macos")]
pub use infrastructure::*;

#[cfg(target_os = "macos")]
mod carbon {
    include!("native_ui/carbon_hotkey.rs");
}
#[cfg(target_os = "macos")]
struct Live {
    view: objc2::rc::Retained<window::settings::macos::HotkeyView>,
    window: objc2::rc::Retained<objc2_app_kit::NSWindow>,
}
#[cfg(target_os = "macos")]
thread_local! { static LIVE: std::cell::RefCell<Option<Live>> = const { std::cell::RefCell::new(None) }; }

#[cfg(target_os = "macos")]
async fn on_main<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    infrastructure::ui_thread::dispatch(move || {
        let _ = sender.send(std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)));
    });
    match tokio::time::timeout(std::time::Duration::from_secs(10), receiver)
        .await
        .expect("AppKit hotkey test did not return from its main-thread operation")
        .unwrap()
    {
        Ok(value) => value,
        Err(panic) => std::panic::resume_unwind(panic),
    }
}
#[cfg(target_os = "macos")]
async fn idle() {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
    loop {
        if on_main(|| LIVE.with_borrow(|live| !live.as_ref().unwrap().view.test_busy())).await {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "Native hotkey update did not finish"
        );
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
}

#[cfg(target_os = "macos")]
async fn focused() {
    on_main(|| {
        LIVE.with_borrow(|live| live.as_ref().unwrap().window.makeKeyAndOrderFront(None));
        // Cooperative activation requires the foreground app to yield. The
        // isolated keyboard probe must activate its own window explicitly.
        #[allow(deprecated)]
        objc2_app_kit::NSApplication::sharedApplication(objc2::MainThreadMarker::new().unwrap())
            .activateIgnoringOtherApps(true);
    })
    .await;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        if on_main(|| {
            let app = objc2_app_kit::NSApplication::sharedApplication(
                objc2::MainThreadMarker::new().unwrap(),
            );
            app.isActive() && LIVE.with_borrow(|live| live.as_ref().unwrap().window.isKeyWindow())
        })
        .await
        {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "Real AppKit settings window did not become key: {:?}",
            on_main(|| {
                let app = objc2_app_kit::NSApplication::sharedApplication(
                    objc2::MainThreadMarker::new().unwrap(),
                );
                let frontmost = objc2_app_kit::NSWorkspace::sharedWorkspace()
                    .frontmostApplication()
                    .and_then(|app| app.bundleIdentifier())
                    .map(|id| id.to_string());
                LIVE.with_borrow(|live| {
                    let live = live.as_ref().unwrap();
                    (
                        app.isActive(),
                        live.window.isKeyWindow(),
                        live.window.isVisible(),
                        frontmost,
                    )
                })
            })
            .await
        );
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
}

#[cfg(target_os = "macos")]
fn send(kind: objc2_app_kit::NSEventType, code: u16, flags: objc2_app_kit::NSEventModifierFlags) {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSEvent};
    use objc2_foundation::{NSPoint, NSString};
    LIVE.with_borrow(|live| {
        let live=live.as_ref().unwrap();
        let event=NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(kind,NSPoint::new(0.0,0.0),flags,0.0,live.window.windowNumber(),None,&NSString::from_str("d"),&NSString::from_str("d"),false,code).unwrap();
        NSApplication::sharedApplication(MainThreadMarker::new().unwrap()).sendEvent(&event);
    });
}

#[cfg(target_os = "macos")]
fn main() {
    use crate::infrastructure::desktop_events::{DesktopEvent, DesktopEvents};
    use crate::window::settings::macos::{HotkeyView, HotkeyViewServices};
    use objc2::MainThreadMarker;
    use objc2_app_kit::{
        NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSEvent,
        NSEventModifierFlags as Flags, NSEventType as Kind, NSUserInterfaceItemIdentification,
        NSWindow, NSWindowStyleMask,
    };
    use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
    use snaplingo_core::application::hotkeys::{
        HotkeyConfiguration, HotkeyRecording, HotkeyRegistrar, HotkeyRegistration,
    };
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct Registrar {
        calls: Mutex<Vec<String>>,
        fail: AtomicBool,
    }
    impl HotkeyRegistrar for Registrar {
        fn register(&self, r: HotkeyRegistration) -> crate::Result<()> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("register:{}", r.accelerator));
            if self.fail.load(Ordering::SeqCst) {
                Err("fixture registration denied".into())
            } else {
                Ok(())
            }
        }
        fn unregister(&self, key: &str) -> crate::Result<()> {
            self.calls.lock().unwrap().push(format!("unregister:{key}"));
            Ok(())
        }
    }
    let mtm = MainThreadMarker::new().unwrap();
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
    app.finishLaunching();
    let store = Arc::new(infrastructure::storage::SqliteConfigStore::new_temp());
    let registrar = Arc::new(Registrar::default());
    let events = Arc::new(DesktopEvents::default());
    let runtime = Arc::new(HotkeyRuntime::with_change_notifier(
        Arc::new(HotkeyConfiguration::new(store)),
        registrar.clone(),
        events.clone(),
    ));
    let recording = HotkeyRecording::default();
    let delivered = events.clone();
    infrastructure::system::shortcut::initialize_shortcuts(
        recording.clone(),
        move |label, payload| {
            delivered.publish(DesktopEvent::HotkeyRecorded {
                window_label: label.into(),
                recording_id: payload.recording_id,
                hotkey: payload.hotkey,
            })
        },
    )
    .unwrap();
    let view = HotkeyView::new(
        mtm,
        "translation",
        HotkeyViewServices {
            runtime: runtime.clone(),
            recording: recording.clone(),
            events: events.clone(),
            window_label: "settings".into(),
        },
        GeneralSettings::default(),
    )
    .unwrap();
    let window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            mtm.alloc(),
            NSRect::new(NSPoint::new(80.0, 80.0), NSSize::new(650.0, 230.0)),
            NSWindowStyleMask::Titled | NSWindowStyleMask::Closable,
            NSBackingStoreType::Buffered,
            false,
        )
    };
    unsafe {
        window.setReleasedWhenClosed(false);
    }
    window.setIdentifier(Some(&NSString::from_str("settings")));
    window.setContentView(Some(&view));
    window.makeKeyAndOrderFront(None);
    app.activate();
    LIVE.with_borrow_mut(|live| *live = Some(Live { view, window }));
    let worker = std::thread::spawn(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            infrastructure::runtime::block_on(async {
                focused().await;
                on_main(|| {
                    LIVE.with_borrow(|live| {
                        let live = live.as_ref().unwrap();
                        assert_eq!(live.view.test_buttons(0), (false, true));
                        live.view.test_record(0);
                        send(Kind::KeyDown, 2, Flags::Shift | Flags::Option);
                    })
                })
                .await;
                assert_eq!(
                    runtime.snapshot().unwrap().translation["selection-translate"],
                    "⌥D",
                    "Recording must wait for release"
                );
                on_main(|| send(Kind::KeyUp, 2, Flags::empty())).await;
                idle().await;
                assert_eq!(
                    runtime.snapshot().unwrap().translation["selection-translate"],
                    "⇧⌥D"
                );
                on_main(|| {
                    LIVE.with_borrow(|live| {
                        let live = live.as_ref().unwrap();
                        live.view.test_record(1);
                        send(Kind::KeyDown, 13, Flags::Option);
                        let event = NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure(
                            Kind::LeftMouseDown, NSPoint::new(3.0, 3.0), Flags::empty(),
                            0.0, live.window.windowNumber(), None, 1, 1, 1.0,
                        ).unwrap();
                        NSApplication::sharedApplication(MainThreadMarker::new().unwrap()).sendEvent(&event);
                        assert!(live.view.test_recording_id().is_none(), "Clicking outside recorder must cancel it");
                        send(Kind::KeyUp, 13, Flags::empty());
                    });
                }).await;
                assert_eq!(
                    runtime.snapshot().unwrap().translation["screenshot-translate"],
                    "⌥S",
                    "Releasing a canceled recording must not save the pending chord"
                );
                on_main(|| {
                    LIVE.with_borrow(|live| {
                        let live = live.as_ref().unwrap();
                        assert_eq!(live.view.test_buttons(0), (true, true));
                        live.view.test_record(1);
                        send(Kind::KeyDown, 53, Flags::empty());
                        assert!(live.view.test_recording_id().is_none());
                    })
                })
                .await;
                on_main(|| {
                    LIVE.with_borrow(|live| {
                        let live = live.as_ref().unwrap();
                        live.view.test_record(1);
                        live.window.orderOut(None);
                        assert!(live.view.test_recording_id().is_none());
                        live.window.makeKeyAndOrderFront(None);
                        NSApplication::sharedApplication(MainThreadMarker::new().unwrap())
                            .activate();
                    })
                })
                .await;
                focused().await;
                on_main(|| LIVE.with_borrow(|live| live.as_ref().unwrap().view.test_clear(0)))
                    .await;
                idle().await;
                assert_eq!(
                    runtime.snapshot().unwrap().translation["selection-translate"],
                    "未设置"
                );
                on_main(|| {
                    LIVE.with_borrow(|live| {
                        let view = &live.as_ref().unwrap().view;
                        assert_eq!(view.test_buttons(0), (true, false));
                        view.test_reset(0);
                    })
                })
                .await;
                idle().await;
                assert_eq!(
                    runtime.snapshot().unwrap().translation["selection-translate"],
                    "⌥D"
                );
                registrar.fail.store(true, Ordering::SeqCst);
                focused().await;
                on_main(|| {
                    LIVE.with_borrow(|live| {
                        live.as_ref().unwrap().view.test_record(0);
                        send(Kind::KeyDown, 13, Flags::Option);
                        send(Kind::KeyUp, 13, Flags::empty());
                    })
                })
                .await;
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
                loop {
                    let alert = on_main(|| {
                        LIVE.with_borrow(|live| {
                            let window = &live.as_ref().unwrap().window;
                            if let Some(sheet) = window.attachedSheet() {
                                window.endSheet_returnCode(&sheet, 1000);
                                true
                            } else {
                                false
                            }
                        })
                    })
                    .await;
                    if alert {
                        break;
                    }
                    assert!(
                        std::time::Instant::now() < deadline,
                        "Failed shortcut registration did not show the native error alert: {:?}, {:?}",
                        on_main(|| LIVE.with_borrow(|live| {
                            let live = live.as_ref().unwrap();
                            (live.view.test_recording_id(), live.view.test_busy(), live.view.test_error(), live.window.isKeyWindow())
                        })).await,
                        registrar.calls.lock().unwrap()
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                }
                idle().await;
                assert_eq!(
                    runtime.snapshot().unwrap().translation["selection-translate"],
                    "⌥D",
                    "Registration failure must preserve previous shortcut"
                );
                on_main(|| {
                    LIVE.with_borrow(|live| {
                        assert!(live
                            .as_ref()
                            .unwrap()
                            .view
                            .test_error()
                            .unwrap()
                            .contains("fixture registration denied"))
                    })
                })
                .await;
                registrar.fail.store(false, Ordering::SeqCst);
                focused().await;
                let actions = Arc::new(AtomicUsize::new(0));
                let fired = actions.clone();
                infrastructure::system::shortcut_native::register(
                    "F18",
                    infrastructure::system::shortcut_native::ShortcutTriggerTiming::Pressed,
                    move || {
                        fired.fetch_add(1, Ordering::SeqCst);
                    },
                )
                .unwrap();
                on_main(|| {
                    LIVE.with_borrow(|live| {
                        let live = live.as_ref().unwrap();
                        live.view.test_record(2);
                        let shortcut = "F18".parse().unwrap();
                        carbon::send_hotkey(&shortcut, false);
                        assert_eq!(live.view.test_recording_id().is_some(), true);
                        carbon::send_hotkey(&shortcut, true);
                    })
                })
                .await;
                idle().await;
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
                while runtime.snapshot().unwrap().translation["show-window"] != "F18" {
                    assert!(std::time::Instant::now() < deadline);
                    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                }
                assert_eq!(
                    actions.load(Ordering::SeqCst),
                    0,
                    "A registered shortcut must not trigger an app action while recording"
                );
                infrastructure::system::shortcut_native::unregister("F18").unwrap();
                let old = on_main(|| {
                    LIVE.with_borrow(|live| {
                        let live = live.as_ref().unwrap();
                        live.view.test_record(1);
                        let id = live.view.test_recording_id().unwrap();
                        live.view.cancel_recording();
                        live.view.test_record(2);
                        id
                    })
                })
                .await;
                events.publish(DesktopEvent::HotkeyRecorded {
                    window_label: "settings".into(),
                    recording_id: old,
                    hotkey: "F19".into(),
                });
                on_main(|| {
                    LIVE.with_borrow(|live| {
                        assert!(live.as_ref().unwrap().view.test_recording_id().is_some())
                    })
                })
                .await;
                on_main(|| LIVE.with_borrow(|live| live.as_ref().unwrap().view.cancel_recording()))
                    .await;
                assert!(!registrar
                    .calls
                    .lock()
                    .unwrap()
                    .iter()
                    .any(|call| call.contains("CmdOrCtrl+KeyC")));
                let lifetime = Arc::downgrade(&runtime);
                drop(runtime);
                on_main(|| {
                    LIVE.with_borrow_mut(|live| {
                        let live = live.take().unwrap();
                        live.view.dispose();
                        live.window.setContentView(None);
                        live.window.close();
                    })
                })
                .await;
                infrastructure::system::shortcut::shutdown_shortcuts().unwrap();
                assert!(
                    lifetime.upgrade().is_none(),
                    "Disposed hotkey controls retained their runtime"
                );
            })
        }));
        infrastructure::ui_thread::dispatch(|| {
            let app = NSApplication::sharedApplication(MainThreadMarker::new().unwrap());
            app.stop(None);
            let wake=NSEvent::otherEventWithType_location_modifierFlags_timestamp_windowNumber_context_subtype_data1_data2(Kind::ApplicationDefined,NSPoint::new(0.0,0.0),Flags::empty(),0.0,0,None,0,0,0).unwrap();
            app.postEvent_atStart(&wake, true);
        });
        if let Err(panic) = result {
            // A failed main-thread operation may still be inside a native modal loop.
            // Exit the isolated harness rather than leaving it waiting for user input.
            let _ = panic;
            std::process::exit(1);
        }
    });
    app.run();
    worker.join().unwrap();
    println!("Native hotkeys: real AppKit key release, modifier mapping, Esc/focus cancellation, row clear/reset, Carbon registered-key routing and stale-session filtering passed");
}
#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
#[path = "../lifecycle/mod.rs"]
mod lifecycle;
#[cfg(target_os = "macos")]
pub use lifecycle::{should_show_settings_on_startup, RuntimeLifecycle};

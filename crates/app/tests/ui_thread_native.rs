// Exercise the production UI queue with NSApplication alone: no Tauri builder,
// app handle, WebView, or Tauri event loop is initialized in this process.
#![allow(dead_code, unused_imports)]
#[cfg(target_os = "macos")]
#[path = "../infrastructure/system/capture_window/backend.rs"]
mod backend;
#[cfg(target_os = "macos")]
use snaplingo_core::error;
#[cfg(target_os = "macos")]
#[path = "../infrastructure/system/capture_window/presentation_macos.rs"]
mod presentation;
#[cfg(target_os = "macos")]
#[path = "../infrastructure/system/shortcut_native.rs"]
mod shortcut_native;
#[cfg(target_os = "macos")]
#[path = "../infrastructure/ui_thread.rs"]
mod ui_thread;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[cfg(target_os = "macos")]
fn main() {
    use std::future::Future;
    use std::sync::{mpsc, Arc, Mutex};
    use std::task::{Context, Poll, Waker};
    use std::time::Duration;

    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSEvent, NSEventModifierFlags, NSEventType};
    use objc2_foundation::NSPoint;

    let main_id = std::thread::current().id();
    let app = NSApplication::sharedApplication(MainThreadMarker::new().unwrap());
    let activation = app.activationPolicy();
    presentation::begin_capture_presentation().unwrap();
    assert_eq!(app.activationPolicy(), activation);
    presentation::begin_capture_presentation().unwrap();
    presentation::end_capture_presentation().unwrap();
    assert!(presentation::is_capture_presentation_active());
    presentation::end_capture_presentation().unwrap();
    assert!(!presentation::is_capture_presentation_active());
    presentation::end_capture_presentation().unwrap();
    assert!(!presentation::is_capture_presentation_active());
    let recorder = Arc::new(());
    let recorder_lifetime = Arc::downgrade(&recorder);
    shortcut_native::initialize(
        |task| {
            ui_thread::dispatch(task);
            Ok(())
        },
        move |_, _| {
            assert_eq!(std::thread::current().id(), main_id);
            let _ = &recorder;
            false
        },
    )
    .unwrap();
    let events = Arc::new(Mutex::new(Vec::new()));
    let canceled_events = events.clone();
    let mut canceled = Box::pin(ui_thread::run("canceled window creation", move || {
        canceled_events.lock().unwrap().push("canceled");
        Ok(())
    }));
    assert!(matches!(
        canceled
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop())),
        Poll::Pending
    ));
    drop(canceled);

    let (done_tx, done_rx) = mpsc::channel();
    std::thread::spawn(move || {
        if done_rx.recv_timeout(Duration::from_secs(20)).is_err() {
            eprintln!("native UI queue stalled");
            std::process::exit(1);
        }
    });
    let worker = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async move {
            let queued_events = events.clone();
            ui_thread::dispatch(move || {
                assert!(MainThreadMarker::new().is_some());
                queued_events.lock().unwrap().push("first");
                let nested_events = queued_events.clone();
                ui_thread::dispatch(move || nested_events.lock().unwrap().push("nested"));
                assert_eq!(*queued_events.lock().unwrap(), vec!["first"]);
            });
            let actual_id = ui_thread::run("thread identity", || Ok(std::thread::current().id())).await.unwrap();
            assert_eq!(actual_id, main_id);
            let failure = ui_thread::run::<()>("failing operation", || Err("native failure".into())).await.unwrap_err();
            assert_eq!(failure.to_string(), "native failure");
            let observed = ui_thread::run("ordered delivery", move || Ok(events.lock().unwrap().clone())).await.unwrap();
            assert_eq!(observed, vec!["first", "nested"]);
            let delivered_on = Arc::new(Mutex::new(None));
            let callback = Arc::new(());
            let callback_lifetime = Arc::downgrade(&callback);
            let mut selected = None;
            for key in ["F18", "F19", "F20"] {
                let callback = callback.clone();
                let delivered_on = delivered_on.clone();
                if shortcut_native::register(key, shortcut_native::ShortcutTriggerTiming::Released, move || {
                    let _ = &callback;
                    *delivered_on.lock().unwrap() = Some(std::thread::current().id());
                }).is_ok() {
                    selected = Some(key);
                    break;
                }
            }
            let key = selected.expect("a free native probe function key");
            drop(callback);
            assert!(callback_lifetime.upgrade().is_some());
            assert!(shortcut_native::is_registered(key).unwrap());
            assert!(shortcut_native::register(key, shortcut_native::ShortcutTriggerTiming::Pressed, || {}).is_err());
            assert!(shortcut_native::is_registered(key).unwrap());
            // global-hotkey's Windows release originates on a polling worker.
            // Both edges must enter the production dispatcher before recording
            // focus checks and application callbacks inspect their window owner.
            let shortcut: global_hotkey::hotkey::HotKey = key.parse().unwrap();
            for state in [global_hotkey::HotKeyState::Pressed, global_hotkey::HotKeyState::Released] {
                shortcut_native::test_dispatch_event(global_hotkey::GlobalHotKeyEvent {id: shortcut.id(), state});
            }
            let actual = ui_thread::run("shortcut callback thread", move || Ok(*delivered_on.lock().unwrap())).await.unwrap();
            assert_eq!(actual, Some(main_id));
            shortcut_native::unregister(key).unwrap();
            assert!(callback_lifetime.upgrade().is_none());
            assert!(!shortcut_native::is_registered(key).unwrap());
            // Register again, then release the manager and its recording callback
            // from a worker. AppKit owns both registration and destruction.
            shortcut_native::register(key, shortcut_native::ShortcutTriggerTiming::Released, || {}).unwrap();
            shortcut_native::shutdown().unwrap();
            assert!(recorder_lifetime.upgrade().is_none());
            assert!(shortcut_native::is_registered(key).is_err());
            ui_thread::run("reinitialize native shortcuts", || {
                shortcut_native::initialize(|task| { ui_thread::dispatch(task); Ok(()) }, |_, _| false)
                    .map_err(|error| error.to_string())
            }).await.unwrap();
            assert!(!shortcut_native::is_registered(key).unwrap());
            shortcut_native::shutdown().unwrap();
            ui_thread::dispatch(move || {
                let app = NSApplication::sharedApplication(MainThreadMarker::new().unwrap());
                app.stop(None);
                let wake = NSEvent::otherEventWithType_location_modifierFlags_timestamp_windowNumber_context_subtype_data1_data2(
                    NSEventType::ApplicationDefined, NSPoint::new(0.0, 0.0), NSEventModifierFlags::empty(),
                    0.0, 0, None, 0, 0, 0,
                ).unwrap();
                app.postEvent_atStart(&wake, true);
                done_tx.send(()).unwrap();
            });
        });
    });
    app.run();
    worker.join().unwrap();
    println!("AppKit capture lifecycle, main queue and native shortcuts: overlapping presentations, activation policy, worker delivery, cancellation, registration, duplicate rollback, shutdown and reinitialization passed without Tauri initialization");
}

#[cfg(not(target_os = "macos"))]
fn main() {}

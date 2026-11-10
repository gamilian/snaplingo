//! Native macOS regression: registered hotkeys are routed to the focused recorder.
//! Carbon events are sent to this process only; no desktop keys or clipboard actions.
#[cfg(target_os = "macos")]
fn main() {
    native::run();
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("This native recording probe runs on macOS.");
}

#[cfg(target_os = "macos")]
mod native {
    use std::ffi::c_void;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    };

    use snaplingo_lib::{system, HotkeyRecording};
    use tauri::{Listener, Manager, WebviewUrl, WebviewWindowBuilder};
    use tauri_plugin_global_shortcut::Shortcut;

    #[repr(C)]
    struct HotkeyId {
        signature: u32,
        id: u32,
    }

    #[link(name = "Carbon", kind = "framework")]
    extern "C" {
        fn GetApplicationEventTarget() -> *mut c_void;
        fn CreateEvent(
            allocator: *const c_void,
            class: u32,
            kind: u32,
            time: f64,
            attributes: u32,
            event: *mut *mut c_void,
        ) -> i32;
        fn SetEventParameter(
            event: *mut c_void,
            name: u32,
            kind: u32,
            size: u32,
            data: *const c_void,
        ) -> i32;
        fn SendEventToEventTarget(event: *mut c_void, target: *mut c_void) -> i32;
        fn ReleaseEvent(event: *mut c_void);
    }

    fn send_hotkey(shortcut: &Shortcut, released: bool) {
        let id = HotkeyId {
            signature: u32::from_be_bytes(*b"htrs"),
            id: shortcut.id(),
        };
        unsafe {
            let mut event = std::ptr::null_mut();
            assert_eq!(
                CreateEvent(
                    std::ptr::null(),
                    u32::from_be_bytes(*b"keyb"),
                    if released { 6 } else { 5 },
                    0.0,
                    0,
                    &mut event
                ),
                0
            );
            assert_eq!(
                SetEventParameter(
                    event,
                    u32::from_be_bytes(*b"----"),
                    u32::from_be_bytes(*b"hkid"),
                    std::mem::size_of::<HotkeyId>() as u32,
                    &id as *const _ as *const c_void
                ),
                0
            );
            let result = SendEventToEventTarget(event, GetApplicationEventTarget());
            ReleaseEvent(event);
            assert_eq!(result, 0);
        }
    }

    pub fn run() {
        tauri::Builder::default()
            .manage(HotkeyRecording::default())
            .plugin(tauri_plugin_global_shortcut::Builder::new().build())
            .setup(|app| {
                app.set_activation_policy(tauri::ActivationPolicy::Accessory);
                let window = WebviewWindowBuilder::new(
                    app,
                    "settings",
                    WebviewUrl::External("about:blank".parse().unwrap()),
                )
                .title("SnapLingo keyboard verification")
                .inner_size(360.0, 100.0)
                .center()
                .focused(true)
                .build()?;
                window.set_focus()?;

                let actions = Arc::new(AtomicUsize::new(0));
                let mut selected = None;
                for key in ["F18", "F19", "F20"] {
                    let count = actions.clone();
                    if system::register_shortcut(app.handle(), key, move || {
                        count.fetch_add(1, Ordering::SeqCst);
                    }).is_ok() {
                        selected = Some(key);
                        break;
                    }
                }
                let key = selected.expect("a free probe function key");
                let shortcut = key.parse::<Shortcut>().unwrap();
                let deliveries = Arc::new(Mutex::new(Vec::<serde_json::Value>::new()));
                let received = deliveries.clone();
                window.listen("hotkey-recorded", move |event| {
                    received.lock().unwrap().push(serde_json::from_str(event.payload()).unwrap());
                });

                let app = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
                    let main_app = app.clone();
                    let (finished, completion) = tokio::sync::oneshot::channel();
                    app.run_on_main_thread(move || {
                        assert!(window.is_focused().unwrap(), "probe window must have focus");
                        send_hotkey(&shortcut, false);
                        send_hotkey(&shortcut, true);
                        assert_eq!(actions.load(Ordering::SeqCst), 1);
                        assert!(deliveries.lock().unwrap().is_empty());

                        let recording = main_app.state::<HotkeyRecording>();
                        recording.begin("settings", "record-one".into());
                        send_hotkey(&shortcut, false);
                        assert_eq!(actions.load(Ordering::SeqCst), 1);
                        assert!(deliveries.lock().unwrap().is_empty());
                        send_hotkey(&shortcut, true);
                        assert_eq!(actions.load(Ordering::SeqCst), 1);
                        assert_eq!(*deliveries.lock().unwrap(), vec![serde_json::json!({
                            "recordingId": "record-one", "hotkey": key,
                        })]);
                        recording.end("settings", "record-one");
                        send_hotkey(&shortcut, false);
                        send_hotkey(&shortcut, true);
                        assert_eq!(actions.load(Ordering::SeqCst), 2);

                        recording.begin("settings", "record-two".into());
                        window.hide().unwrap();
                        finished.send((shortcut, actions, deliveries)).unwrap();
                    }).unwrap();
                    let (shortcut, actions, deliveries) = completion.await.unwrap();
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    let main_app = app.clone();
                    app.run_on_main_thread(move || {
                        send_hotkey(&shortcut, false);
                        send_hotkey(&shortcut, true);
                        assert_eq!(actions.load(Ordering::SeqCst), 3);
                        assert_eq!(deliveries.lock().unwrap().len(), 1);
                        assert!(main_app.state::<HotkeyRecording>().session().is_none());
                        assert!(!system::is_shortcut_registered(&main_app, "CmdOrCtrl+KeyC").unwrap());
                        system::unregister_shortcut(&main_app, key).unwrap();
                        println!("PASS: native {key} recorded on release; its action was suppressed only while recording; cancellation and focus loss restore normal routing; Cmd+C remains unregistered");
                        main_app.exit(0);
                    }).unwrap();
                });
                Ok(())
            })
            .run(tauri::generate_context!())
            .expect("native hotkey recording probe failed");
    }
}

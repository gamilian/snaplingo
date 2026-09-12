//! Native smoke test: capture editing keys must never be registered globally.
use snaplingo_lib::system::{capture_window, is_shortcut_registered};

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let handle = app.handle();
            let assert_unregistered = |stage: &str| {
                for key in [
                    "Escape",
                    "CmdOrCtrl+KeyC",
                    "CmdOrCtrl+KeyS",
                    "CmdOrCtrl+KeyZ",
                    "CmdOrCtrl+KeyY",
                ] {
                    assert!(
                        !is_shortcut_registered(handle, key).unwrap(),
                        "{stage}: {key} was registered globally"
                    );
                }
                println!("{stage}: all capture editing keys remain unregistered");
            };
            assert_unregistered("idle");
            capture_window::begin_capture_presentation(handle).unwrap();
            assert_unregistered("starting before window creation");
            capture_window::begin_capture_presentation(handle).unwrap();
            capture_window::hide_capture_window(handle).unwrap();
            assert_unregistered("overlapping hidden presentations");
            capture_window::end_capture_presentation(handle).unwrap();
            assert_unregistered("one presentation remains");
            capture_window::end_capture_presentation(handle).unwrap();
            assert_unregistered("finished");
            app.handle().exit(0);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("capture shortcut scope smoke test failed");
}

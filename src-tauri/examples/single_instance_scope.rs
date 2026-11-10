//! Verify a second process exits before setup, using an isolated app identifier.
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn main() {
    let secondary = std::env::args().any(|arg| arg == "--secondary");
    let identifier = std::env::var("SNAPLINGO_INSTANCE_TEST_ID")
        .unwrap_or_else(|_| format!("com.snaplingo.instance-test-{}", std::process::id()));
    let mut context = tauri::generate_context!();
    context.config_mut().identifier = identifier.clone();
    let received = Arc::new(AtomicUsize::new(0));
    let callback_received = received.clone();
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(move |_, args, _| {
            assert!(args.iter().any(|arg| arg == "--secondary"));
            callback_received.fetch_add(1, Ordering::SeqCst);
        }))
        .setup(move |app| {
            assert!(!secondary, "The secondary process reached application setup");
            let handle = app.handle().clone();
            let received = received.clone();
            let identifier = identifier.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(200));
                let mut child = std::process::Command::new(std::env::current_exe().unwrap())
                    .arg("--secondary").env("SNAPLINGO_INSTANCE_TEST_ID", identifier)
                    .spawn().unwrap();
                let deadline = Instant::now() + Duration::from_secs(8);
                loop {
                    if let Some(status) = child.try_wait().unwrap() {
                        if status.success() && received.load(Ordering::SeqCst) == 1 {
                            println!("Single instance: secondary process exited before setup and notified the primary");
                            handle.exit(0);
                            return;
                        }
                    }
                    if Instant::now() >= deadline {
                        let _ = child.kill();
                        eprintln!("Single instance verification timed out");
                        handle.exit(1);
                        return;
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
            });
            Ok(())
        })
        .run(context)
        .expect("Single instance smoke test failed");
}

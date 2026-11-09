//! Verify native process ownership, notification and crash takeover in an isolated directory.
use snaplingo_lib::system::single_instance::SingleInstance;
use std::path::Path;
use std::process::{Child, Command};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn child(mode: &str, path: &Path) -> Child {
    Command::new(std::env::current_exe().unwrap())
        .arg(mode)
        .arg(path)
        .spawn()
        .unwrap()
}

fn wait_success(mut child: Child) {
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(status.success());
            return;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            panic!("Single-instance child timed out");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn main() {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() == 3 {
        let mode = args[1].to_string_lossy();
        let path = Path::new(&args[2]);
        let instance = SingleInstance::acquire(path).unwrap();
        match mode.as_ref() {
            "--secondary" => assert!(instance.is_none(), "Secondary reached primary setup"),
            "--primary-crash" => {
                assert!(instance.is_some());
                std::fs::write(path.with_extension("ready"), b"ready").unwrap();
                // Process termination skips Rust drops. The OS must release ownership anyway.
                std::process::exit(0);
            }
            _ => panic!("Unknown probe mode"),
        }
        return;
    }

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("instance.lock");
    let primary = SingleInstance::acquire(&path).unwrap().unwrap();
    // A secondary can notify before UI/database setup finishes; delivery remains pending.
    wait_success(child("--secondary", &path));
    let received = Arc::new(AtomicUsize::new(0));
    let callbacks = received.clone();
    primary.on_activation(move || {
        callbacks.fetch_add(1, Ordering::SeqCst);
    });
    assert_eq!(received.load(Ordering::SeqCst), 1);
    wait_success(child("--secondary", &path));
    let deadline = Instant::now() + Duration::from_secs(1);
    while received.load(Ordering::SeqCst) != 2 {
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    drop(primary);

    wait_success(child("--primary-crash", &path));
    assert!(path.with_extension("ready").exists());
    let replacement = SingleInstance::acquire(&path).unwrap().unwrap();
    drop(replacement);
    println!("Native single instance: secondary exited before setup, pending/live activation arrived, and crash takeover succeeded");
}

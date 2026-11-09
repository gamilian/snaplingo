#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
fn main() {
    if std::env::args().nth(1).as_deref() == Some("--version") {
        println!("SnapLingo {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    let result = snaplingo_lib::run_native();
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let result: Result<(), String> =
        Err("The standalone native application supports macOS and Windows".into());
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

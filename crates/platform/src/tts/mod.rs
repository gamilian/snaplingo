#[cfg(target_os = "macos")]
mod macos;
#[cfg(any(all(not(target_os = "macos"), not(target_os = "windows")), test))]
mod unavailable;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
pub use macos::MacOsSystemTtsHost;
#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
pub use unavailable::UnavailableSystemTtsHost;
#[cfg(target_os = "windows")]
pub use windows::WindowsSystemTtsHost;

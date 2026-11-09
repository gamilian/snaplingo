mod controller;
#[cfg(target_os = "macos")]
pub(crate) mod macos;

#[cfg(target_os = "macos")]
pub(crate) use macos::{NativeResultWindowHost, NativeResultWindowPresentation};

#[cfg(all(target_os = "windows", feature = "native-app"))]
mod windows;
#[cfg(all(target_os = "windows", feature = "native-app"))]
pub(crate) use windows::{
    accessibility_event as windows_accessibility, close_all as windows_close_all,
    window_event as windows_window_event,
};
#[cfg(all(target_os = "windows", feature = "native-app"))]
pub(crate) use windows::{NativeResultWindowHost, NativeResultWindowPresentation};

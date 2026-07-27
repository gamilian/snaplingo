mod geometry;
mod image;

#[cfg(target_os = "linux")]
pub(crate) mod linux;
#[cfg(target_os = "macos")]
pub(crate) mod macos;
#[cfg(target_os = "windows")]
pub(crate) mod windows;
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
mod xcap_common;

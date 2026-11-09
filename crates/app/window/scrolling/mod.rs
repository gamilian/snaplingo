//! Scrolling Capture owns one session lifetime across two native adapters.
mod geometry;
#[cfg(target_os = "macos")]
pub(crate) mod macos;
#[cfg(any(
    target_os = "macos",
    all(target_os = "windows", feature = "native-app"),
    test
))]
mod sampling;
#[cfg(any(
    target_os = "macos",
    all(target_os = "windows", feature = "native-app")
))]
mod session;
#[cfg(all(target_os = "windows", feature = "native-app"))]
pub(crate) mod windows;
#[cfg(target_os = "macos")]
use macos as platform;
#[cfg(all(target_os = "windows", feature = "native-app"))]
use windows as platform;

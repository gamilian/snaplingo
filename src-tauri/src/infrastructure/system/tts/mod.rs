#[cfg(target_os = "macos")]
mod macos;
#[cfg(any(not(target_os = "macos"), test))]
mod unavailable;

#[cfg(target_os = "macos")]
pub(crate) use macos::MacOsSystemTtsHost;
#[cfg(not(target_os = "macos"))]
pub(crate) use unavailable::UnavailableSystemTtsHost;

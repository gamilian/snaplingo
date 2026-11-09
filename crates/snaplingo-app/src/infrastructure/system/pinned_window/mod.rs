#[cfg(any(all(target_os = "windows", feature = "native-app"), test))]
mod geometry;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
mod native_host;
#[cfg(target_os = "macos")]
mod native_view;
mod presentation;
#[cfg(any(all(target_os = "windows", feature = "native-app"), test))]
mod scene;

#[cfg(target_os = "macos")]
pub(crate) use native_host::NativePinnedImageRuntimeHost;
#[cfg(all(test, target_os = "macos"))]
#[allow(unused_imports)]
pub(crate) use native_view::test_decoded_images;
#[cfg(all(test, target_os = "macos"))]
#[allow(unused_imports)]
pub(crate) use native_view::test_ids;
#[cfg(all(test, target_os = "macos"))]
pub(crate) fn test_close(id: &str) -> Result<(), String> {
    native_view::close(id);
    Ok(())
}

#[cfg(all(target_os = "windows", feature = "native-app"))]
mod windows;
#[cfg(all(target_os = "windows", feature = "native-app"))]
pub(crate) use windows::{
    accessibility_event as windows_accessibility, close_all as windows_close_all,
    window_event as windows_window_event, NativePinnedImageRuntimeHost,
};

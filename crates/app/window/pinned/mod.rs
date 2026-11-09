#[cfg(any(all(target_os = "windows", feature = "native-app"), test))]
mod geometry;
#[cfg(target_os = "macos")]
mod macos;
mod position;
#[cfg(any(all(target_os = "windows", feature = "native-app"), test))]
mod scene;
#[cfg(all(target_os = "windows", feature = "native-app"))]
mod windows;

#[cfg(all(test, target_os = "macos"))]
#[allow(unused_imports)]
pub(crate) use macos::view::{test_decoded_images, test_ids};
#[cfg(target_os = "macos")]
pub(crate) use macos::NativePinnedImageRuntimeHost;
#[cfg(all(test, target_os = "macos"))]
pub(crate) fn test_close(id: &str) -> Result<(), String> {
    macos::test_close(id);
    Ok(())
}
#[cfg(all(target_os = "windows", feature = "native-app"))]
pub(crate) use windows::{
    accessibility_event as windows_accessibility, close_all as windows_close_all,
    window_event as windows_window_event, NativePinnedImageRuntimeHost,
};

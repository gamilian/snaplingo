#[cfg(any(not(target_os = "macos"), test))]
mod backend;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
mod native_view;
#[cfg(any(target_os = "macos", test))]
mod presentation;
mod runtime_host;
#[cfg(not(target_os = "macos"))]
mod tauri;

#[cfg(target_os = "macos")]
pub(crate) use native_view::has_key_window as has_key_native_window;
#[cfg(target_os = "macos")]
pub(crate) use native_view::has_visible_window_on_active_space as has_visible_native_window_on_active_space;
#[cfg(all(test, target_os = "macos"))]
#[allow(unused_imports)]
pub(crate) use native_view::test_decoded_images;
pub(crate) use runtime_host::TauriPinnedImageRuntimeHost;

#[cfg(target_os = "macos")]
pub use macos::{
    apply_pinned_group_window_switch, close_pinned_group_windows, close_pinned_image_window,
    hide_moved_pinned_image_window, hide_pinned_group_windows, open_pinned_image_window,
    show_or_open_pinned_image_window, toggle_pinned_image_windows_visibility,
};
#[cfg(not(target_os = "macos"))]
pub use tauri::{
    apply_pinned_group_window_switch, close_pinned_group_windows, close_pinned_image_window,
    hide_moved_pinned_image_window, hide_pinned_group_windows, open_pinned_image_window,
    show_or_open_pinned_image_window, toggle_pinned_image_windows_visibility,
};

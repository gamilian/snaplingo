pub(crate) mod annotation_fonts;
pub mod capture_cursor;
pub(crate) mod capture_output;
pub mod capture_window;
pub(crate) mod clipboard_text;
pub(crate) mod dialogs;
pub(crate) mod external_open;
mod hotkey_recording;
mod hotkey_registrar;
#[cfg(any(all(target_os = "windows", feature = "native-app"), test))]
pub(crate) mod image_print_windows;
#[cfg(target_os = "macos")]
mod image_save_dialog;
#[cfg(all(target_os = "windows", feature = "native-app"))]
pub(crate) mod layered_windows;
#[cfg(target_os = "macos")]
pub(crate) mod native_theme;
#[cfg(target_os = "macos")]
pub(crate) mod native_view_snapshot;
pub mod ocr;
pub mod paths;
pub mod pinned_window;
pub mod required_permissions;
pub mod result_window;
pub mod screenshot;
pub mod screenshot_favorites;
#[cfg(target_os = "macos")]
mod scrolling_controls_macos;
mod scrolling_geometry;
#[cfg(target_os = "macos")]
pub(crate) mod scrolling_host_macos;
#[cfg(target_os = "macos")]
mod scrolling_native_macos;
#[cfg(target_os = "macos")]
mod scrolling_overlays_macos;
pub mod selection;
#[cfg(target_os = "macos")]
pub(crate) mod settings_window;
pub mod shortcut;
pub(crate) mod shortcut_native;
pub mod single_instance;
pub mod start_on_boot;
pub mod tts;
pub mod updates;
pub(crate) mod window_position;

pub use hotkey_recording::RecordedHotkey;
pub(crate) use hotkey_registrar::NativeHotkeyRegistrar;
pub use paths::{get_app_data_dir, get_database_path};

#[cfg(all(test, target_os = "macos"))]
mod tests {
    #[test]
    fn system_ocr_language_hints_live_in_infrastructure() {
        assert_eq!(
            super::ocr::vision_languages_for_request(None),
            vec!["zh-Hans".to_string(), "en-US".to_string()]
        );
    }
}

#[cfg(all(target_os = "windows", feature = "native-app"))]
pub(crate) mod scrolling_host_windows;
#[cfg(all(target_os = "windows", feature = "native-app"))]
mod scrolling_view_windows;

#[cfg(any(all(target_os = "windows", feature = "native-app"), test))]
mod scrolling_sampling;

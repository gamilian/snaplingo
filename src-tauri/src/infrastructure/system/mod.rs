pub mod capture_window;
pub mod ocr;
pub mod paths;
pub mod pinned_window;
pub mod result_window;
pub mod screenshot;
pub mod screenshot_favorites;
pub mod selection;
pub mod shortcut;

pub use paths::{get_app_data_dir, get_database_path};
pub use screenshot::get_capture_session_source;
pub(crate) use shortcut::TauriHotkeyRegistrar;
pub use shortcut::{
    is_shortcut_registered, register_shortcut, register_shortcut_on_release, unregister_shortcut,
};

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

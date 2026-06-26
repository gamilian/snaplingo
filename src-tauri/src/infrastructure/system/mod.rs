pub mod capture_window;
pub mod hotkey;
pub mod paths;
pub mod pinned_window;
pub mod screenshot;
pub mod selection;
pub mod shortcut;

#[allow(deprecated)]
pub use hotkey::{get_hotkey_backend, HotkeyBackend, HotkeyId};
pub use paths::{get_config_dir, get_config_path, get_history_db_path};
pub use screenshot::{get_screenshot_backend, ScreenRegion, ScreenshotBackend};
pub use shortcut::{
    is_shortcut_registered, register_shortcut, register_shortcut_on_release, unregister_shortcut,
};

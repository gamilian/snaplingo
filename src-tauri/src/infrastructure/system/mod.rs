pub mod capture_window;
pub mod hotkey;
pub mod paths;
pub mod pinned_window;
pub mod screenshot;
pub mod selection;
pub mod shortcut;

pub use hotkey::{HotkeyBackend, HotkeyId, get_hotkey_backend};
pub use paths::{get_config_dir, get_config_path, get_history_db_path};
pub use screenshot::{ScreenRegion, ScreenshotBackend, get_screenshot_backend};
pub use shortcut::{
    is_shortcut_registered, register_shortcut, register_shortcut_on_release, unregister_shortcut,
};

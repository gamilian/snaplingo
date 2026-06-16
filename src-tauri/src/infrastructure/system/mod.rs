pub mod paths;
pub mod hotkey;
pub mod screenshot;
pub mod shortcut;

pub use paths::{get_config_dir, get_config_path, get_history_db_path};
pub use hotkey::{HotkeyBackend, HotkeyId, get_hotkey_backend};
pub use screenshot::{ScreenshotBackend, ScreenRegion, get_screenshot_backend};
pub use shortcut::{is_shortcut_registered, register_shortcut, unregister_shortcut};

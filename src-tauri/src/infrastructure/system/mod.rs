pub mod paths;
pub mod hotkey;
pub mod screenshot;

pub use paths::{get_config_dir, get_config_path, get_history_db_path};
pub use hotkey::{HotkeyBackend, HotkeyId, get_hotkey_backend};
pub use screenshot::{ScreenshotBackend, ScreenRegion, get_screenshot_backend};

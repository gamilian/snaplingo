pub mod paths;
pub mod hotkey;

pub use paths::{get_config_dir, get_config_path, get_history_db_path};
pub use hotkey::{HotkeyBackend, HotkeyId, get_hotkey_backend};

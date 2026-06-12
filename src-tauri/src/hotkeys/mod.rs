mod types;

#[cfg(target_os = "macos")]
mod macos;

pub use types::{CaptureMode, HotkeyConfig};

#[cfg(target_os = "macos")]
pub use macos::MacOSHotkeyManager as HotkeyManager;

#[cfg(not(target_os = "macos"))]
pub struct HotkeyManager;

#[cfg(not(target_os = "macos"))]
impl HotkeyManager {
    pub fn new(_app: tauri::AppHandle) -> Self {
        Self
    }

    pub fn register(&self, _config: &HotkeyConfig) -> Result<(), String> {
        Err("Hotkeys not supported on this platform".to_string())
    }

    pub fn unregister_all(&self) -> Result<(), String> {
        Err("Hotkeys not supported on this platform".to_string())
    }
}

use super::types::HotkeyConfig;
use tauri::AppHandle;

pub struct MacOSHotkeyManager {
    _app: AppHandle,
}

impl MacOSHotkeyManager {
    pub fn new(app: AppHandle) -> Self {
        Self { _app: app }
    }

    pub fn register(&self, _config: &HotkeyConfig) -> Result<(), String> {
        // Placeholder: actual implementation post-MVP
        Ok(())
    }

    pub fn unregister_all(&self) -> Result<(), String> {
        // Placeholder: actual implementation post-MVP
        Ok(())
    }
}

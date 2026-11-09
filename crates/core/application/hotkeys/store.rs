use crate::domain::hotkey_config::HotkeySettingsSnapshot;
use crate::Result;

pub trait HotkeyStore: Send + Sync {
    fn load_hotkeys(&self) -> Result<HotkeySettingsSnapshot>;
    fn save_hotkeys(&self, snapshot: &HotkeySettingsSnapshot) -> Result<()>;
}

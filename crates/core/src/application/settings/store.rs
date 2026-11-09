use crate::domain::SettingsSnapshot;
use crate::Result;

pub trait SettingsStore: Send + Sync {
    fn load_settings(&self) -> Result<SettingsSnapshot>;
    fn save_settings(&self, snapshot: &SettingsSnapshot) -> Result<()>;
}

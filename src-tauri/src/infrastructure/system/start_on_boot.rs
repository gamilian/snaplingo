use crate::application::StartOnBoot;
use crate::Result;
use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;
pub struct TauriStartOnBoot(pub AppHandle);
impl StartOnBoot for TauriStartOnBoot {
    fn set_enabled(&self, enabled: bool) -> Result<()> {
        let result = if enabled {
            self.0.autolaunch().enable()
        } else {
            self.0.autolaunch().disable()
        };
        result.map_err(|error| error.to_string().into())
    }
}

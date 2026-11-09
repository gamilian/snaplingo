use std::sync::Arc;

use crate::application::{SettingsConfiguration, SystemTtsHost, TtsRuntime};
#[cfg(target_os = "macos")]
use crate::infrastructure::system::tts::MacOsSystemTtsHost;
#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
use crate::infrastructure::system::tts::UnavailableSystemTtsHost;
#[cfg(target_os = "windows")]
use crate::infrastructure::system::tts::WindowsSystemTtsHost;

pub(crate) fn build_tts_runtime(settings: Arc<SettingsConfiguration>) -> Arc<TtsRuntime> {
    Arc::new(TtsRuntime::new(build_system_tts_host(), settings))
}

fn build_system_tts_host() -> Arc<dyn SystemTtsHost> {
    #[cfg(target_os = "macos")]
    {
        Arc::new(MacOsSystemTtsHost::new())
    }

    #[cfg(target_os = "windows")]
    {
        Arc::new(WindowsSystemTtsHost::new())
    }

    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        Arc::new(UnavailableSystemTtsHost::new(std::env::consts::OS))
    }
}

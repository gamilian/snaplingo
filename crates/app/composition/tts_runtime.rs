use std::sync::Arc;

#[cfg(target_os = "macos")]
use crate::infrastructure::system::tts::MacOsSystemTtsHost;
#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
use crate::infrastructure::system::tts::UnavailableSystemTtsHost;
#[cfg(target_os = "windows")]
use crate::infrastructure::system::tts::WindowsSystemTtsHost;
use snaplingo_core::application::{SettingsConfiguration, SystemTtsHost, TtsRuntime};

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

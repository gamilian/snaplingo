#[cfg(feature = "native-app")]
pub mod host;
#[cfg(target_os = "macos")]
pub(crate) mod macos;
pub mod presenter;
#[cfg(all(test, target_os = "macos"))]
pub(crate) mod test_support;
#[cfg(all(target_os = "windows", feature = "native-app"))]
pub(crate) mod windows;

use crate::integrations::desktop_events::DesktopEvents;
use crate::integrations::desktop_events::DesktopSubscription;
use presenter::window::SettingsWindowSession;
use snaplingo_core::hotkeys::HotkeyRecording;
#[cfg(test)]
use snaplingo_core::hotkeys::HotkeyRuntime;
use snaplingo_core::permissions::RequiredPermissions;
#[cfg(test)]
use snaplingo_core::settings::SettingsApplication;
use snaplingo_core::settings::SettingsConfiguration;
use std::sync::Arc;

pub(crate) type NativeEffect = Arc<dyn Fn() -> crate::Result<()> + Send + Sync>;
pub(crate) type SessionFactory = Arc<
    dyn Fn(
            HotkeyRecording,
            NativeEffect,
            NativeEffect,
        ) -> (SettingsWindowSession, DesktopSubscription)
        + Send
        + Sync,
>;

/// Dependencies of Settings and its native sheets. Composition owns their
/// construction; window adapters never resolve the global application state.
#[derive(Clone)]
pub(crate) struct Services {
    pub configuration: Arc<SettingsConfiguration>,
    #[cfg(test)]
    pub hotkeys: Arc<HotkeyRuntime>,
    #[cfg(test)]
    pub application: Arc<SettingsApplication>,
    pub permissions: Arc<RequiredPermissions>,
    pub events: Arc<DesktopEvents>,
    pub session: SessionFactory,
}

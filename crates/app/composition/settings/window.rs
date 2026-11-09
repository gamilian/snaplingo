//! Composition of the portable Settings session and native application adapters.
use crate::application::hotkeys::HotkeyRecording;
use crate::infrastructure::desktop_events::{DesktopEvent, DesktopSubscription};
pub(crate) use snaplingo_settings::window::SettingsWindowSession;
use snaplingo_settings::{
    preferences::PreferencesDependencies, window::SettingsSessionDependencies,
};
use std::sync::Arc;

pub(crate) fn from_state(
    state: Arc<crate::AppState>,
    recording: HotkeyRecording,
    notices: Arc<dyn Fn() -> crate::Result<()> + Send + Sync>,
    capacity: Arc<dyn Fn() -> crate::Result<()> + Send + Sync>,
) -> (SettingsWindowSession, DesktopSubscription) {
    let executor = crate::infrastructure::runtime::handle();
    let events = state.desktop_events.clone();
    let changed = events.clone();
    let usage = snaplingo_settings::capacity_usage::CapacityUsage::new(
        state.library_index.clone(),
        executor.clone(),
        Arc::new(move || changed.publish(DesktopEvent::SettingsChanged)),
    );
    let services = state.clone();
    let library = state.clone();
    let session = SettingsWindowSession::new(
        SettingsSessionDependencies {
            preferences: PreferencesDependencies {
                configuration: state.settings.configuration.clone(),
                settings: state.settings.application.clone(),
                hotkeys: state.settings.hotkeys.clone(),
                general: super::general::from_state(
                    &state,
                    env!("CARGO_PKG_VERSION").into(),
                    notices,
                ),
                usage,
            },
            services: Arc::new(move || super::services::from_state(&services, Default::default())),
            library: Arc::new(move |kind| super::library::from_state(&library, kind)),
            capacity: Arc::new(move || capacity().map_err(|e| e.to_string())),
            open_url: Arc::new(|url| {
                crate::infrastructure::system::external_open::open_url(url)
                    .map_err(|e| e.to_string())
            }),
        },
        recording,
        executor,
    );
    let invalidate = session.invalidator();
    let subscription = events.subscribe(move |event| {
        let library = matches!(
            event,
            DesktopEvent::HistoryChanged
                | DesktopEvent::FavoritesChanged
                | DesktopEvent::ScreenshotFavoritesChanged
        );
        if library
            || matches!(
                event,
                DesktopEvent::SettingsChanged
                    | DesktopEvent::HotkeysChanged
                    | DesktopEvent::ProvidersChanged
            )
        {
            invalidate(library);
        }
    });
    (session, subscription)
}

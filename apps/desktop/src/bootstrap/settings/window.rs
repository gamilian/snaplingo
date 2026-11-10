//! Composition of the portable Settings session and native application adapters.
use crate::features::settings::presenter::preferences::PreferencesDependencies;
use crate::features::settings::presenter::window::SettingsSessionDependencies;
pub(crate) use crate::features::settings::presenter::window::SettingsWindowSession;
use crate::integrations::desktop_events::DesktopEvent;
use std::sync::Arc;

fn factory(state: &crate::AppState) -> crate::features::settings::SessionFactory {
    let executor = crate::integrations::runtime::handle();
    let events = state.desktop_events.clone();
    let changed = events.clone();
    let usage = crate::features::settings::presenter::capacity_usage::CapacityUsage::new(
        state.library_index.clone(),
        executor.clone(),
        Arc::new(move || changed.publish(DesktopEvent::SettingsChanged)),
    );
    let configuration = state.settings.configuration.clone();
    let settings = state.settings.application.clone();
    let hotkeys = state.settings.hotkeys.clone();
    let general =
        super::general::dependencies(state, env!("CARGO_PKG_VERSION").into(), Arc::new(|| Ok(())));
    let services = Arc::new(super::services::factory(state));
    let library = Arc::new(super::library::factory(state));
    Arc::new(move |recording, notices, capacity| {
        let mut general = general.clone();
        general.open_notices = notices;
        let session = SettingsWindowSession::new(
        SettingsSessionDependencies {
            preferences: PreferencesDependencies {
                configuration: configuration.clone(),
                settings: settings.clone(),
                hotkeys: hotkeys.clone(),
                general: Arc::new(crate::features::settings::presenter::general_controller::GeneralController::new(general)),
                usage: usage.clone(),
            },
            services: { let services = services.clone(); Arc::new(move || services(Default::default())) },
            library: library.clone(),
            capacity: Arc::new(move || capacity().map_err(|e| e.to_string())),
            open_url: Arc::new(|url| {
                crate::integrations::system::external_open::open_url(url)
                    .map_err(|e| e.to_string())
            }),
        },
        recording,
        executor.clone(),
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
    })
}

/// Assemble the feature dependencies once at the desktop composition root.
pub(crate) fn services(state: Arc<crate::AppState>) -> crate::features::settings::Services {
    crate::features::settings::Services {
        configuration: state.settings.configuration.clone(),
        hotkeys: state.settings.hotkeys.clone(),
        #[cfg(test)]
        application: state.settings.application.clone(),
        permissions: state.permissions.clone(),
        events: state.desktop_events.clone(),
        session: factory(&state),
    }
}

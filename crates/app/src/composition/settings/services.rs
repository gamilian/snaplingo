use crate::application::settings::{
    ServicesController, ServicesDependencies, ServicesKind, TranslationServicesController,
};
pub(crate) use snaplingo_settings::services::ServicesSession;
use std::sync::Arc;

pub(crate) fn from_state(
    state: &crate::AppState,
    route: snaplingo_ui::services::ServicesRoute,
) -> ServicesSession {
    let dependencies = || ServicesDependencies {
        configuration: state.settings.configuration.clone(),
        settings: state.settings.application.clone(),
        providers: state.providers.administration.clone(),
        tts: state.tts.clone(),
    };
    ServicesSession::new(
        Arc::new(ServicesController::new(ServicesKind::Ocr, dependencies())),
        Arc::new(ServicesController::new(ServicesKind::Tts, dependencies())),
        Arc::new(TranslationServicesController::new(
            state.providers.administration.clone(),
        )),
        route,
    )
    .with_navigation(state.settings.navigation.clone())
}

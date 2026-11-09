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
    .with_navigation(Arc::new(Navigation(state.settings.navigation.clone())))
}

struct Navigation(Arc<crate::infrastructure::storage::SqliteConfigStore>);
impl snaplingo_settings::services::ServicesNavigation for Navigation {
    fn load(&self) -> Result<Option<String>, String> {
        self.0.load_services_route().map_err(|e| e.to_string())
    }
    fn save(&self, route: &str) -> Result<(), String> {
        self.0.save_services_route(route).map_err(|e| e.to_string())
    }
}

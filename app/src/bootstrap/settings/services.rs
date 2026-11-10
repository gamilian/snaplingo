pub(crate) use crate::features::settings::presenter::services::ServicesSession;
use snaplingo_core::settings::ServicesController;
use snaplingo_core::settings::ServicesDependencies;
use snaplingo_core::settings::ServicesKind;
use snaplingo_core::settings::TranslationServicesController;
use std::sync::Arc;

pub(super) fn factory(
    state: &crate::AppState,
) -> impl Fn(snaplingo_ui::services::ServicesRoute) -> ServicesSession + Send + Sync + 'static {
    let configuration = state.settings.configuration.clone();
    let settings = state.settings.application.clone();
    let providers = state.providers.administration.clone();
    let tts = state.tts.clone();
    let navigation = state.settings.navigation.clone();
    move |route| {
        let dependencies = || ServicesDependencies {
            configuration: configuration.clone(),
            settings: settings.clone(),
            providers: providers.clone(),
            tts: tts.clone(),
        };
        ServicesSession::new(
            Arc::new(ServicesController::new(ServicesKind::Ocr, dependencies())),
            Arc::new(ServicesController::new(ServicesKind::Tts, dependencies())),
            Arc::new(TranslationServicesController::new(providers.clone())),
            route,
        )
        .with_navigation(Arc::new(Navigation(navigation.clone())))
    }
}

struct Navigation(Arc<snaplingo_storage::SqliteConfigStore>);
impl crate::features::settings::presenter::services::ServicesNavigation for Navigation {
    fn load(&self) -> Result<Option<String>, String> {
        self.0.load_services_route().map_err(|e| e.to_string())
    }
    fn save(&self, route: &str) -> Result<(), String> {
        self.0.save_services_route(route).map_err(|e| e.to_string())
    }
}

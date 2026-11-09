//! Reuses the production General workflow behind the shared Rust drawing.
use std::sync::Arc;

use snaplingo_settings::general_controller::{GeneralController, GeneralServices};

pub(crate) fn from_state(
    state: &crate::AppState,
    version: String,
    open_notices: Arc<dyn Fn() -> crate::Result<()> + Send + Sync>,
) -> Arc<GeneralController> {
    Arc::new(GeneralController::new(GeneralServices {
        configuration: state.settings.configuration.clone(),
        settings: state.settings.application.clone(),
        history: state.history.history.clone(),
        permissions: state.permissions.clone(),
        updates: state.updates.clone(),
        version,
        open_url: Arc::new(crate::infrastructure::system::external_open::open_url),
        open_notices,
    }))
}

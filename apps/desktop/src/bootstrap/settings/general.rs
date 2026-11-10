//! Reuses the production General workflow behind the shared Rust drawing.
use std::sync::Arc;

use crate::features::settings::presenter::general_controller::GeneralServices;

pub(super) fn dependencies(
    state: &crate::AppState,
    version: String,
    open_notices: Arc<dyn Fn() -> crate::Result<()> + Send + Sync>,
) -> GeneralServices {
    GeneralServices {
        configuration: state.settings.configuration.clone(),
        settings: state.settings.application.clone(),
        history: state.history.history.clone(),
        permissions: state.permissions.clone(),
        updates: state.updates.clone(),
        version,
        open_url: Arc::new(crate::integrations::system::external_open::open_url),
        open_notices,
    }
}

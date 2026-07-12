use std::sync::Arc;

use tauri::AppHandle;

use crate::application::{SelectedTextAcquirer, SelectionScheme, SystemSelectionProvider};
use crate::infrastructure::system::selection::platform_selection_provider;

pub(crate) fn build_selected_text_acquirer(app: AppHandle) -> Arc<SelectedTextAcquirer> {
    let self_bundle_id = Some(app.config().identifier.clone());
    let selection_provider = Arc::new(platform_selection_provider(app, self_bundle_id));
    let selection_scheme = SelectionScheme::new(selection_provider.default_scheme());

    Arc::new(SelectedTextAcquirer::new(
        selection_scheme,
        selection_provider.methods(),
        selection_provider,
    ))
}

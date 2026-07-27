use std::sync::Arc;

use tauri::AppHandle;

use crate::application::{SelectedTextAcquirer, SelectionScheme, SystemSelectionProvider};
#[cfg(target_os = "linux")]
use crate::infrastructure::system::selection::linux::PlatformSelectionProvider;
#[cfg(target_os = "macos")]
use crate::infrastructure::system::selection::macos::MacSelectionProvider;
#[cfg(target_os = "windows")]
use crate::infrastructure::system::selection::windows::PlatformSelectionProvider;

pub(crate) fn build_selected_text_acquirer(app: AppHandle) -> Arc<SelectedTextAcquirer> {
    let selection_provider = build_selection_provider(app);
    let selection_scheme = SelectionScheme::new(selection_provider.default_scheme());

    Arc::new(SelectedTextAcquirer::new(
        selection_scheme,
        selection_provider.methods(),
        selection_provider,
    ))
}

fn build_selection_provider(app: AppHandle) -> Arc<dyn SystemSelectionProvider> {
    #[cfg(target_os = "macos")]
    {
        let self_bundle_id = Some(app.config().identifier.clone());
        Arc::new(MacSelectionProvider::new(app, self_bundle_id))
    }

    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        let _ = app;
        Arc::new(PlatformSelectionProvider)
    }
}

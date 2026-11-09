use std::sync::Arc;

use crate::application::{SelectedTextAcquirer, SelectionScheme, SystemSelectionProvider};
#[cfg(target_os = "linux")]
use crate::infrastructure::system::selection::linux::PlatformSelectionProvider;
#[cfg(target_os = "macos")]
use crate::infrastructure::system::selection::macos::MacSelectionProvider;
#[cfg(target_os = "windows")]
use crate::infrastructure::system::selection::windows::PlatformSelectionProvider;

pub(crate) fn build_selected_text_acquirer(identifier: String) -> Arc<SelectedTextAcquirer> {
    let selection_provider = build_selection_provider(identifier);
    let selection_scheme = SelectionScheme::new(selection_provider.default_scheme());

    Arc::new(SelectedTextAcquirer::new(
        selection_scheme,
        selection_provider.methods(),
        selection_provider,
    ))
}

fn build_selection_provider(identifier: String) -> Arc<dyn SystemSelectionProvider> {
    #[cfg(target_os = "macos")]
    {
        Arc::new(MacSelectionProvider::new(Some(identifier)))
    }

    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        let _ = identifier;
        Arc::new(PlatformSelectionProvider)
    }
}

use crate::domain::{SelectionContext, SelectionMethodKind};

use super::backend::{SelectionContextProvider, SelectionMethod, SystemSelectionProvider};

pub struct PlatformSelectionProvider;

impl SelectionContextProvider for PlatformSelectionProvider {
    fn context(&self) -> SelectionContext {
        SelectionContext::default()
    }
}

impl SystemSelectionProvider for PlatformSelectionProvider {
    fn default_scheme(&self) -> Vec<SelectionMethodKind> {
        Vec::new()
    }

    fn methods(&self) -> Vec<Box<dyn SelectionMethod>> {
        Vec::new()
    }
}

pub fn platform_selection_provider(
    _app: tauri::AppHandle,
    _self_bundle_id: Option<String>,
) -> PlatformSelectionProvider {
    PlatformSelectionProvider
}

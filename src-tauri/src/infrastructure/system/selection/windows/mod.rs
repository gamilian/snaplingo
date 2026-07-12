mod shortcut_copy;

use crate::domain::{SelectionContext, SelectionMethodKind};

use crate::application::selected_text::{
    SelectionContextProvider, SelectionMethod, SystemSelectionProvider,
};

pub struct PlatformSelectionProvider;

impl SelectionContextProvider for PlatformSelectionProvider {
    fn context(&self) -> SelectionContext {
        SelectionContext::default()
    }
}

impl SystemSelectionProvider for PlatformSelectionProvider {
    fn default_scheme(&self) -> Vec<SelectionMethodKind> {
        vec![SelectionMethodKind::ShortcutCopy]
    }

    fn methods(&self) -> Vec<Box<dyn SelectionMethod>> {
        vec![Box::new(shortcut_copy::ShortcutCopySelectionMethod)]
    }
}

pub fn platform_selection_provider(
    _app: tauri::AppHandle,
    _self_bundle_id: Option<String>,
) -> PlatformSelectionProvider {
    PlatformSelectionProvider
}

#[cfg(test)]
mod selection_provider_tests {
    use super::*;
    use crate::application::selected_text::SystemSelectionProvider;

    mod windows {
        mod shortcut_copy {
            use super::super::*;

            #[test]
            fn provider_methods_include_shortcut_copy() {
                let provider = PlatformSelectionProvider;

                let method_kinds = provider
                    .methods()
                    .into_iter()
                    .map(|method| method.kind())
                    .collect::<Vec<_>>();

                assert_eq!(method_kinds, vec![SelectionMethodKind::ShortcutCopy]);
            }
        }
    }

    #[test]
    fn default_scheme_prefers_shortcut_copy() {
        let provider = PlatformSelectionProvider;

        assert_eq!(
            provider.default_scheme(),
            vec![SelectionMethodKind::ShortcutCopy]
        );
    }
}

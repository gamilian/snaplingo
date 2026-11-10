mod accessibility;
mod browser_applescript;
pub mod context;
mod menu_copy;
mod pasteboard;
mod self_webview;
mod shortcut_copy;

use crate::domain::{SelectionContext, SelectionMethodKind};

use crate::application::selected_text::{
    SelectionContextProvider, SelectionMethod, SystemSelectionProvider,
};

pub struct MacSelectionProvider {
    app: tauri::AppHandle,
    self_bundle_id: Option<String>,
}

impl MacSelectionProvider {
    pub fn new(app: tauri::AppHandle, self_bundle_id: Option<String>) -> Self {
        Self {
            app,
            self_bundle_id,
        }
    }
}

impl SelectionContextProvider for MacSelectionProvider {
    fn context(&self) -> SelectionContext {
        context::frontmost_context(self.self_bundle_id.clone())
    }
}

impl SystemSelectionProvider for MacSelectionProvider {
    fn default_scheme(&self) -> Vec<SelectionMethodKind> {
        vec![
            SelectionMethodKind::SelfWebview,
            SelectionMethodKind::Accessibility,
            SelectionMethodKind::BrowserScript,
            SelectionMethodKind::MenuCopy,
            SelectionMethodKind::ShortcutCopy,
        ]
    }

    fn methods(&self) -> Vec<Box<dyn SelectionMethod>> {
        vec![
            Box::new(self_webview::SelfWebviewSelectionMethod::new(
                self.app.clone(),
            )),
            Box::new(accessibility::AccessibilitySelectionMethod),
            Box::new(browser_applescript::BrowserAppleScriptSelectionMethod),
            Box::new(menu_copy::MenuCopySelectionMethod),
            Box::new(shortcut_copy::ShortcutCopySelectionMethod::new(
                self.app.clone(),
            )),
        ]
    }
}

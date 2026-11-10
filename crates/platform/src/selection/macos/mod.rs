mod accessibility;
mod browser_applescript;
pub mod context;
mod menu_copy;
mod pasteboard;
mod self_webview;
mod shortcut_copy;

use snaplingo_core::selection::model::{SelectionContext, SelectionMethodKind};

use snaplingo_core::selection::{
    SelectionContextProvider, SelectionMethod, SystemSelectionProvider,
};

pub struct MacSelectionProvider {
    self_bundle_id: Option<String>,
}

impl MacSelectionProvider {
    pub fn new(self_bundle_id: Option<String>) -> Self {
        Self { self_bundle_id }
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
            Box::new(self_webview::SelfWebviewSelectionMethod),
            Box::new(accessibility::AccessibilitySelectionMethod),
            Box::new(browser_applescript::BrowserAppleScriptSelectionMethod),
            Box::new(menu_copy::MenuCopySelectionMethod),
            Box::new(shortcut_copy::ShortcutCopySelectionMethod),
        ]
    }
}

/// Exercise the native responder reader against a host's real text controls.
/// Available only to acceptance fixtures, never to ordinary application builds.
#[cfg(feature = "test-support")]
pub mod test_support {
    pub async fn read_focused_window_selection() -> Result<String, String> {
        super::self_webview::read_focused_window_selection().await
    }

    pub fn read_window(
        window: &objc2_app_kit::NSWindow,
    ) -> tokio::sync::oneshot::Receiver<Result<String, String>> {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        super::self_webview::read_window_selection(window, sender);
        receiver
    }
}

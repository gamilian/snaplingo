use snaplingo_ui::{Page, Size, WindowIntent, WindowSnapshot};
use std::sync::Arc;

/// Services and Library implementations are injected by the application. The
/// window adapter has no provider, storage or desktop-shell ownership.
pub trait WindowBackend {
    fn snapshot(&self) -> Result<WindowSnapshot, String>;
    /// Destructive intents are confirmed by the window before this call.
    fn dispatch(&mut self, page: Page, intent: WindowIntent) -> Result<(), String>;
    /// Async adapters call this when a fresh snapshot is available. Passing None
    /// releases the window callback before the native event loop is destroyed.
    fn set_notifier(&mut self, _notifier: Option<Arc<dyn Fn() + Send + Sync>>) {}
    fn interface_scale(&self, _display: Size) -> f32 {
        1.0
    }
}

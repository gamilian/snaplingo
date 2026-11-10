use crate::features::settings::presenter::confirmation::Confirmation;
use snaplingo_ui::{Page, Size, WindowIntent, WindowSnapshot};
use std::sync::Arc;

/// Services and Library implementations are injected by the application. The
/// window adapter has no provider, storage or desktop-shell ownership.
pub trait WindowBackend {
    fn snapshot(&self) -> Result<WindowSnapshot, String>;
    /// The shared workflow returns a dialog request before executing a mutation.
    fn dispatch(
        &mut self,
        page: Page,
        intent: WindowIntent,
    ) -> Result<Option<Confirmation>, String>;
    fn confirm(&mut self, request: Confirmation, accepted: bool) -> Result<(), String>;
    fn cancel_confirmation(&mut self);
    /// Async adapters call this when a fresh snapshot is available. Passing None
    /// releases the window callback before the native event loop is destroyed.
    fn set_notifier(&mut self, _notifier: Option<Arc<dyn Fn() + Send + Sync>>) {}
    fn interface_scale(&self, _display: Size) -> f32 {
        1.0
    }
}

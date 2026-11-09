//! Compatibility path for hosts using the extracted portable settings adapter.
pub use snaplingo_settings::*;

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

pub struct SettingsWindowBackend<B>(pub B);

impl<B: SettingsBackend> WindowBackend for SettingsWindowBackend<B> {
    fn snapshot(&self) -> Result<WindowSnapshot, String> {
        self.0.snapshot().map(WindowSnapshot::Settings)
    }
    fn dispatch(&mut self, page: Page, intent: WindowIntent) -> Result<(), String> {
        match intent {
            WindowIntent::Settings(action) => self.0.dispatch(page, action),
            _ => Err("当前设置预览未接入此页面".into()),
        }
    }
    fn interface_scale(&self, display: Size) -> f32 {
        self.0.interface_scale(display)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snaplingo_ui::{Action, OcrField, Value};
    use std::sync::Arc;

    #[test]
    fn existing_preview_backend_preserves_real_persistence_through_window_intents() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("preferences.db");
        let backend = PersistentSettings::open(&path, Arc::new(RecordingOnlyRegistrar)).unwrap();
        let mut backend = SettingsWindowBackend(backend);
        backend
            .dispatch(
                Page::Ocr,
                WindowIntent::Settings(Action::EditOcr {
                    field: OcrField::RecognitionLanguage,
                    value: Value::Text("ja".into()),
                }),
            )
            .unwrap();
        let WindowSnapshot::Settings(snapshot) = backend.snapshot().unwrap() else {
            panic!()
        };
        assert_eq!(snapshot.ocr.recognition_language, "ja");
        assert!(backend
            .dispatch(
                Page::Services,
                WindowIntent::Services(snaplingo_ui::services::ServicesIntent::Refresh,)
            )
            .is_err());
        drop(backend);
        let reopened = SettingsWindowBackend(
            PersistentSettings::open(&path, Arc::new(RecordingOnlyRegistrar)).unwrap(),
        );
        let WindowSnapshot::Settings(snapshot) = reopened.snapshot().unwrap() else {
            panic!()
        };
        assert_eq!(snapshot.ocr.recognition_language, "ja");
    }
}

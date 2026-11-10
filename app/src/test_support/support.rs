//! Shared native acceptance environment. Scenario-specific fake effects stay in
//! their owner; database paths, platform selection and main-thread delivery do not.
use std::sync::Arc;

use snaplingo_core::selection::{SelectedTextAcquirer, SelectionScheme, SystemSelectionProvider};

pub(super) mod carbon;

pub(super) struct IsolatedApplication {
    pub(super) state: crate::AppState,
    pub(super) directory: tempfile::TempDir,
}
// When extracting these fields, bind the directory guard before state so the
// application and its database handles are released before removing the files.
impl IsolatedApplication {
    pub(super) fn new(identifier: &str) -> Self {
        let directory = tempfile::tempdir().expect("create native acceptance directory");
        let path = directory.path().join("fixture.db");
        let database = Arc::new(snaplingo_storage::Database::open(&path).unwrap());
        let selection = Arc::new(
            snaplingo_platform::selection::macos::MacSelectionProvider::new(Some(
                identifier.into(),
            )),
        );
        let state = crate::bootstrap::build_app_state(
            database,
            path,
            crate::bootstrap::CompositionOptions {
                identifier: identifier.into(),
                version: env!("CARGO_PKG_VERSION").into(),
                ocr_resource_root: None,
                cache_dir: Ok(directory.path().join("cache")),
                selected_text: Arc::new(SelectedTextAcquirer::new(
                    SelectionScheme::new(selection.default_scheme()),
                    selection.methods(),
                    selection,
                )),
            },
        );
        Self { state, directory }
    }
}

pub(super) async fn on_main<T: Send + 'static>(
    operation: impl FnOnce() -> T + Send + 'static,
) -> T {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    crate::integrations::ui_thread::dispatch(move || {
        let _ = sender.send(std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            operation,
        )));
    });
    match receiver
        .await
        .expect("native acceptance UI dispatcher stopped")
    {
        Ok(value) => value,
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

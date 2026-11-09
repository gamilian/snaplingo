#[cfg(all(test, target_os = "macos"))]
#[path = "library/tests.rs"]
pub(super) mod tests;
#[cfg(all(test, target_os = "macos"))]
use snaplingo_core::application::library_index as core;
use snaplingo_core::application::library_index::{LibraryClipboard, LibraryPages};
pub(crate) use snaplingo_settings::library::LibrarySession;
#[cfg(all(test, target_os = "macos"))]
use snaplingo_ui::library as ui;
use std::sync::Arc;

pub(crate) fn from_state(
    state: &crate::AppState,
    kind: snaplingo_ui::library::LibraryKind,
) -> LibrarySession {
    LibrarySession::new(
        LibraryPages::new(
            state.history.history.clone(),
            state.favorites.favorites.clone(),
            state.screenshot_favorites.favorites.clone(),
            state.library_index.clone(),
            state.history.ocr_replay.clone(),
            state.favorites.ocr_application.clone(),
        ),
        Arc::new(Clipboard),
        kind,
    )
}
struct Clipboard;
#[async_trait::async_trait]
impl LibraryClipboard for Clipboard {
    async fn write_text(&self, text: &str) -> crate::Result<()> {
        crate::infrastructure::system::clipboard_text::copy_text(text.into()).map_err(Into::into)
    }
}

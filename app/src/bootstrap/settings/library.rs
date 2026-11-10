#[cfg(all(test, target_os = "macos"))]
#[path = "library/tests.rs"]
pub(super) mod tests;
pub(crate) use crate::features::settings::presenter::library::LibrarySession;
use snaplingo_core::library::index::LibraryClipboard;
use snaplingo_core::library::index::LibraryPages;
use std::sync::Arc;

pub(super) fn factory(
    state: &crate::AppState,
) -> impl Fn(snaplingo_ui::library::LibraryKind) -> LibrarySession + Send + Sync + 'static {
    let history = state.history.history.clone();
    let favorites = state.favorites.favorites.clone();
    let screenshots = state.screenshot_favorites.favorites.clone();
    let index = state.library_index.clone();
    let history_ocr = state.history.ocr_replay.clone();
    let favorite_ocr = state.favorites.ocr_application.clone();
    move |kind| {
        LibrarySession::new(
            LibraryPages::new(
                history.clone(),
                favorites.clone(),
                screenshots.clone(),
                index.clone(),
                history_ocr.clone(),
                favorite_ocr.clone(),
            ),
            Arc::new(Clipboard),
            kind,
        )
    }
}

struct Clipboard;
#[async_trait::async_trait]
impl LibraryClipboard for Clipboard {
    async fn write_text(&self, text: &str) -> crate::Result<()> {
        snaplingo_platform::clipboard_text::copy_text(text.into()).map_err(Into::into)
    }
}

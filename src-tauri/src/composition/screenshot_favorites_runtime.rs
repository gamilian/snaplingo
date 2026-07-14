use std::sync::Arc;

use crate::application::screenshot_favorites::{
    ScreenshotFavoriteCapture, ScreenshotFavoriteCaptureRenderer, ScreenshotFavoriteChangeNotifier,
    ScreenshotFavorites,
};
use crate::application::CaptureSessionRuntime;
use crate::domain::capture::{AnnotationCommand, CaptureSessionId, LogicalRect};
use crate::infrastructure::storage::{
    Database, FilesystemScreenshotFavoriteAssets, SqliteScreenshotFavoriteRepository,
};
use crate::infrastructure::system::screenshot_favorites::{
    CaptureOutputScreenshotClipboard, SystemScreenshotFavoriteHost,
};
use crate::{CaptureOutput, Result};
use tauri::{AppHandle, Emitter};

struct TauriScreenshotFavoriteChangeNotifier {
    app: AppHandle,
}

impl ScreenshotFavoriteChangeNotifier for TauriScreenshotFavoriteChangeNotifier {
    fn screenshot_favorites_changed(&self) {
        if let Err(error) = self.app.emit("screenshot-favorites-changed", ()) {
            log::warn!("Failed to emit screenshot-favorites-changed: {}", error);
        }
    }
}

struct CaptureRuntimeFavoriteRenderer {
    runtime: Arc<CaptureSessionRuntime>,
}

impl ScreenshotFavoriteCaptureRenderer for CaptureRuntimeFavoriteRenderer {
    fn render_png(
        &self,
        session_id: &CaptureSessionId,
        rect: &LogicalRect,
        annotations: &[AnnotationCommand],
        include_cursor: bool,
    ) -> Result<Vec<u8>> {
        self.runtime
            .render_png(session_id, rect, annotations, include_cursor)
    }
}

pub(crate) fn build_screenshot_favorites(
    database: Arc<Database>,
    asset_root: std::path::PathBuf,
    capture_runtime: Arc<CaptureSessionRuntime>,
    capture_output: Arc<CaptureOutput>,
    app: AppHandle,
) -> (Arc<ScreenshotFavorites>, Arc<ScreenshotFavoriteCapture>) {
    let favorites = Arc::new(ScreenshotFavorites::with_change_notifier(
        Arc::new(SqliteScreenshotFavoriteRepository::new(database)),
        Arc::new(FilesystemScreenshotFavoriteAssets::new(asset_root)),
        Arc::new(CaptureOutputScreenshotClipboard::new(capture_output)),
        Arc::new(SystemScreenshotFavoriteHost),
        Arc::new(TauriScreenshotFavoriteChangeNotifier { app }),
    ));
    let capture = Arc::new(ScreenshotFavoriteCapture::new(
        favorites.clone(),
        Arc::new(CaptureRuntimeFavoriteRenderer {
            runtime: capture_runtime,
        }),
    ));
    (favorites, capture)
}

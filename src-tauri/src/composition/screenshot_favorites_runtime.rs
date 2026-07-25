use std::sync::Arc;

use async_trait::async_trait;

use crate::application::screenshot_favorites::{
    ScreenshotFavoriteCapture, ScreenshotFavoriteCaptureRenderer, ScreenshotFavoriteChangeNotifier,
    ScreenshotFavoriteClipboard, ScreenshotFavorites,
};
use crate::application::{CaptureSessionRuntime, FavoriteCapacity};
use crate::domain::capture::{AnnotationCommand, CaptureSessionId, LogicalRect};
use crate::infrastructure::storage::{
    Database, FilesystemScreenshotFavoriteAssets, SqliteScreenshotFavoriteRepository,
};
use crate::infrastructure::system::screenshot_favorites::SystemScreenshotFavoriteHost;
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

struct CaptureOutputScreenshotClipboard {
    output: Arc<CaptureOutput>,
}

#[async_trait]
impl ScreenshotFavoriteClipboard for CaptureOutputScreenshotClipboard {
    async fn copy_png(&self, png_data: &[u8]) -> Result<()> {
        self.output.copy_png(png_data).await
    }
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
    capacity: Arc<FavoriteCapacity>,
    app: AppHandle,
) -> (Arc<ScreenshotFavorites>, Arc<ScreenshotFavoriteCapture>) {
    let favorites = Arc::new(ScreenshotFavorites::with_change_notifier_and_capacity(
        Arc::new(SqliteScreenshotFavoriteRepository::new(database)),
        Arc::new(FilesystemScreenshotFavoriteAssets::new(asset_root)),
        Arc::new(CaptureOutputScreenshotClipboard {
            output: capture_output,
        }),
        Arc::new(SystemScreenshotFavoriteHost),
        Arc::new(TauriScreenshotFavoriteChangeNotifier { app }),
        capacity,
    ));
    let capture = Arc::new(ScreenshotFavoriteCapture::new(
        favorites.clone(),
        Arc::new(CaptureRuntimeFavoriteRenderer {
            runtime: capture_runtime,
        }),
    ));
    (favorites, capture)
}

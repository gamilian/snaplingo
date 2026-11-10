use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use snaplingo_core::library::capacity::FavoriteCapacity;
use snaplingo_core::library::capacity::FavoriteCapacityPolicyProvider;
use snaplingo_core::library::favorites::FavoriteAssetStore;
use snaplingo_core::library::favorites::Favorites;
use snaplingo_core::library::favorites::OcrFavoriteApplication;
use snaplingo_core::library::favorites::OcrFavoriteRecognizer;
use snaplingo_core::library::favorites::StoredFavoriteAssets;
use snaplingo_core::library::history::History;
use snaplingo_core::library::history::HistoryChangeNotifier;
use snaplingo_core::library::history::OcrHistoryAssetStore;
use snaplingo_core::library::history::OcrHistoryRecognizer;
use snaplingo_core::library::history::OcrHistoryReplay;
use snaplingo_core::library::history::StoredOcrHistoryAssets;
use snaplingo_core::library::index::LibraryClipboard;
use snaplingo_core::library::index::LibraryIndex;
use snaplingo_core::library::index::LibraryIndexRepository;
use snaplingo_core::library::index::LibraryPages;
use snaplingo_core::library::screenshot_favorites::ScreenshotFavoriteAssetStore;
use snaplingo_core::library::screenshot_favorites::ScreenshotFavoriteClipboard;
use snaplingo_core::library::screenshot_favorites::ScreenshotFavoriteHost;
use snaplingo_core::library::screenshot_favorites::ScreenshotFavorites;
use snaplingo_core::library::screenshot_favorites::StoredScreenshotAssets;
use snaplingo_core::providers::ocr::model::OcrResult;
use snaplingo_core::Result;
use snaplingo_storage::{
    Database, SqliteFavoriteCapacityRepository, SqliteFavoriteRepository, SqliteHistoryRepository,
    SqliteScreenshotFavoriteRepository,
};

#[derive(Default)]
pub(super) struct Assets {
    reads: Mutex<Vec<String>>,
    copied: Mutex<Vec<Vec<u8>>>,
    revealed: Mutex<Vec<String>>,
    text: Mutex<Vec<String>>,
}
impl Assets {
    fn read(&self, path: &str) -> Result<Vec<u8>> {
        self.reads.lock().unwrap().push(path.into());
        Ok(path.as_bytes().to_vec())
    }
}
impl FavoriteCapacityPolicyProvider for Assets {
    fn maximum_favorites(&self) -> Result<u32> {
        Ok(0)
    }
}
impl HistoryChangeNotifier for Assets {
    fn history_changed(&self) {}
}
impl OcrHistoryAssetStore for Assets {
    fn store(&self, _: &[u8]) -> Result<StoredOcrHistoryAssets> {
        unreachable!()
    }
    fn read(&self, path: &str) -> Result<Vec<u8>> {
        self.read(path)
    }
    fn delete(&self, _: &str) -> Result<()> {
        Ok(())
    }
}
impl FavoriteAssetStore for Assets {
    fn store_ocr(&self, _: &[u8]) -> Result<StoredFavoriteAssets> {
        unreachable!()
    }
    fn read(&self, path: &str) -> Result<Vec<u8>> {
        self.read(path)
    }
    fn delete(&self, _: &str) -> Result<()> {
        Ok(())
    }
}
impl ScreenshotFavoriteAssetStore for Assets {
    fn store(&self, _: &[u8]) -> Result<StoredScreenshotAssets> {
        unreachable!()
    }
    fn read(&self, path: &str) -> Result<Vec<u8>> {
        self.read(path)
    }
    fn delete(&self, _: &str) -> Result<()> {
        Ok(())
    }
    fn absolute_path(&self, path: &str) -> Result<String> {
        Ok(format!("/assets/{path}"))
    }
}
#[async_trait]
impl ScreenshotFavoriteClipboard for Assets {
    async fn copy_png(&self, png: &[u8]) -> Result<()> {
        self.copied.lock().unwrap().push(png.to_vec());
        Ok(())
    }
}
impl ScreenshotFavoriteHost for Assets {
    fn reveal(&self, path: &str) -> Result<()> {
        self.revealed.lock().unwrap().push(path.into());
        Ok(())
    }
}
#[async_trait]
impl LibraryClipboard for Assets {
    async fn write_text(&self, text: &str) -> Result<()> {
        self.text.lock().unwrap().push(text.into());
        Ok(())
    }
}
#[async_trait]
impl OcrFavoriteRecognizer for Assets {
    fn active_provider_id(&self) -> Option<String> {
        Some("system".into())
    }
    async fn recognize(&self, _: Vec<u8>) -> Result<OcrResult> {
        Ok(OcrResult::from_text("rerun", None))
    }
}
#[async_trait]
impl OcrHistoryRecognizer for Assets {
    async fn recognize(&self, _: Vec<u8>) -> Result<OcrResult> {
        Ok(OcrResult::from_text("rerun", None))
    }
}

pub(super) fn pages(
    database: Arc<Database>,
    assets: Arc<Assets>,
    index: Arc<dyn LibraryIndexRepository>,
) -> LibraryPages {
    let history = Arc::new(History::with_dependencies(
        Arc::new(SqliteHistoryRepository::new(database.clone())),
        assets.clone(),
        assets.clone(),
    ));
    let capacity = Arc::new(FavoriteCapacity::new(
        Arc::new(SqliteFavoriteCapacityRepository::new(database.clone())),
        assets.clone(),
    ));
    let favorites = Arc::new(Favorites::new(
        Arc::new(SqliteFavoriteRepository::new(database.clone())),
        assets.clone(),
        capacity.clone(),
    ));
    LibraryPages::new(
        history.clone(),
        favorites.clone(),
        Arc::new(ScreenshotFavorites::new(
            Arc::new(SqliteScreenshotFavoriteRepository::new(database)),
            assets.clone(),
            assets.clone(),
            assets.clone(),
            capacity,
        )),
        Arc::new(LibraryIndex::new(index)),
        Arc::new(OcrHistoryReplay::new(history.clone(), assets.clone())),
        Arc::new(OcrFavoriteApplication::new(history, favorites, assets)),
    )
}

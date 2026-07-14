use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;

use super::*;
use crate::application::favorite_capacity::{
    FavoriteCapacityPolicyProvider, FavoriteCapacityRepository,
};
use crate::{AppError, Result};

#[derive(Default)]
struct FakeRepository {
    records: Mutex<Vec<ScreenshotFavoriteRecord>>,
    fail_insert: Mutex<bool>,
}

#[async_trait]
impl ScreenshotFavoriteRepository for FakeRepository {
    async fn insert(&self, input: &NewScreenshotFavorite) -> Result<ScreenshotFavoriteRecord> {
        if *self.fail_insert.lock().unwrap() {
            return Err(AppError::Other("insert failed".into()));
        }
        let mut records = self.records.lock().unwrap();
        let record = ScreenshotFavoriteRecord {
            id: records.len() as i64 + 1,
            content_kind: "screenshot".to_string(),
            created_at: input.created_at,
            asset_path: input.asset_path.clone(),
            thumbnail_path: input.thumbnail_path.clone(),
            width: input.width,
            height: input.height,
            note: None,
            tags: vec![],
        };
        records.push(record.clone());
        Ok(record)
    }

    async fn query(
        &self,
        _query: &ScreenshotFavoriteQuery,
    ) -> Result<(Vec<ScreenshotFavoriteRecord>, usize)> {
        let records = self.records.lock().unwrap().clone();
        Ok((records.clone(), records.len()))
    }

    async fn find(&self, id: i64) -> Result<Option<ScreenshotFavoriteRecord>> {
        Ok(self
            .records
            .lock()
            .unwrap()
            .iter()
            .find(|record| record.id == id)
            .cloned())
    }

    async fn update_metadata(
        &self,
        id: i64,
        note: Option<String>,
        tags: Vec<String>,
    ) -> Result<()> {
        let mut records = self.records.lock().unwrap();
        let record = records.iter_mut().find(|record| record.id == id).unwrap();
        record.note = note;
        record.tags = tags;
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<()> {
        self.records
            .lock()
            .unwrap()
            .retain(|record| record.id != id);
        Ok(())
    }
}

#[async_trait]
impl FavoriteCapacityRepository for FakeRepository {
    async fn current_count(&self) -> Result<usize> {
        Ok(self.records.lock().unwrap().len())
    }
}

#[derive(Default)]
struct FakeAssets {
    files: Mutex<HashMap<String, Vec<u8>>>,
}

impl ScreenshotFavoriteAssetStore for FakeAssets {
    fn store(&self, png_data: &[u8]) -> Result<StoredScreenshotAssets> {
        let mut files = self.files.lock().unwrap();
        files.insert("screenshots/one.png".into(), png_data.to_vec());
        files.insert("thumbnails/one.png".into(), vec![4, 5, 6]);
        Ok(StoredScreenshotAssets {
            asset_path: "screenshots/one.png".into(),
            thumbnail_path: "thumbnails/one.png".into(),
            width: 320,
            height: 180,
        })
    }

    fn read(&self, relative_path: &str) -> Result<Vec<u8>> {
        Ok(self.files.lock().unwrap()[relative_path].clone())
    }

    fn delete(&self, relative_path: &str) -> Result<()> {
        self.files.lock().unwrap().remove(relative_path);
        Ok(())
    }

    fn absolute_path(&self, relative_path: &str) -> Result<String> {
        Ok(format!("/assets/{relative_path}"))
    }
}

#[derive(Default)]
struct FakeClipboard(Mutex<Vec<u8>>);

#[async_trait]
impl ScreenshotFavoriteClipboard for FakeClipboard {
    async fn copy_png(&self, png_data: &[u8]) -> Result<()> {
        *self.0.lock().unwrap() = png_data.to_vec();
        Ok(())
    }
}

struct FakeHost;

impl ScreenshotFavoriteHost for FakeHost {
    fn reveal(&self, _absolute_path: &str) -> Result<()> {
        Ok(())
    }
}

struct FakePolicy(u32);

impl FavoriteCapacityPolicyProvider for FakePolicy {
    fn maximum_favorites(&self) -> Result<u32> {
        Ok(self.0)
    }
}

struct FakeNotifier;

impl ScreenshotFavoriteChangeNotifier for FakeNotifier {
    fn screenshot_favorites_changed(&self) {}
}

fn service(
    repository: Arc<FakeRepository>,
    assets: Arc<FakeAssets>,
    clipboard: Arc<FakeClipboard>,
) -> ScreenshotFavorites {
    ScreenshotFavorites::new(
        repository,
        assets,
        clipboard,
        Arc::new(FakeHost),
        Arc::new(FavoriteCapacity::unlimited()),
    )
}

#[tokio::test]
async fn full_capacity_rejects_a_screenshot_before_writing_assets() {
    let repository = Arc::new(FakeRepository::default());
    repository
        .records
        .lock()
        .unwrap()
        .push(ScreenshotFavoriteRecord {
            id: 1,
            content_kind: "screenshot".to_string(),
            created_at: Utc::now(),
            asset_path: "existing.png".to_string(),
            thumbnail_path: "existing-thumb.png".to_string(),
            width: 100,
            height: 50,
            note: None,
            tags: vec![],
        });
    let assets = Arc::new(FakeAssets::default());
    let capacity = Arc::new(FavoriteCapacity::new(
        repository.clone(),
        Arc::new(FakePolicy(1)),
    ));
    let favorites = ScreenshotFavorites::with_change_notifier_and_capacity(
        repository,
        assets.clone(),
        Arc::new(FakeClipboard::default()),
        Arc::new(FakeHost),
        Arc::new(FakeNotifier),
        capacity,
    );

    let error = favorites.add_png(&[1, 2, 3]).await.unwrap_err();

    assert!(error.to_string().contains("收藏夹容量已满"));
    assert!(assets.files.lock().unwrap().is_empty());
}

#[tokio::test]
async fn failed_metadata_insert_rolls_back_written_assets() {
    let repository = Arc::new(FakeRepository::default());
    *repository.fail_insert.lock().unwrap() = true;
    let assets = Arc::new(FakeAssets::default());
    let favorites = service(
        repository,
        assets.clone(),
        Arc::new(FakeClipboard::default()),
    );

    assert!(favorites.add_png(&[1, 2, 3]).await.is_err());
    assert!(assets.files.lock().unwrap().is_empty());
}

#[tokio::test]
async fn query_returns_embedded_thumbnail_and_delete_removes_both_assets() {
    let repository = Arc::new(FakeRepository::default());
    let assets = Arc::new(FakeAssets::default());
    let favorites = service(
        repository.clone(),
        assets.clone(),
        Arc::new(FakeClipboard::default()),
    );
    let record = favorites.add_png(&[1, 2, 3]).await.unwrap();

    let page = favorites
        .query(ScreenshotFavoriteQuery {
            limit: 20,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(page.total, 1);
    assert_eq!(
        page.items[0].thumbnail_data_url,
        "data:image/png;base64,BAUG"
    );

    favorites.delete(record.id).await.unwrap();
    assert!(assets.files.lock().unwrap().is_empty());
    assert!(repository.records.lock().unwrap().is_empty());
}

#[tokio::test]
async fn metadata_and_copy_use_the_original_asset() {
    let repository = Arc::new(FakeRepository::default());
    let clipboard = Arc::new(FakeClipboard::default());
    let favorites = service(
        repository.clone(),
        Arc::new(FakeAssets::default()),
        clipboard.clone(),
    );
    let record = favorites.add_png(&[1, 2, 3]).await.unwrap();

    favorites
        .update_metadata(record.id, Some("note".into()), vec!["work".into()])
        .await
        .unwrap();
    favorites.copy(record.id).await.unwrap();

    let updated = repository.find(record.id).await.unwrap().unwrap();
    assert_eq!(updated.note.as_deref(), Some("note"));
    assert_eq!(updated.tags, vec!["work"]);
    assert_eq!(*clipboard.0.lock().unwrap(), vec![1, 2, 3]);
    assert_eq!(
        favorites.absolute_path(record.id).await.unwrap(),
        "/assets/screenshots/one.png"
    );
    assert!(updated.created_at <= Utc::now());
}

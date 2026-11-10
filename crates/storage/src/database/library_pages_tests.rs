use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{TimeZone, Utc};

use super::super::{
    Database, SqliteFavoriteCapacityRepository, SqliteFavoriteRepository, SqliteHistoryRepository,
    SqliteLibraryIndexRepository, SqliteScreenshotFavoriteRepository,
};
use crate::Result;
use snaplingo_core::library::capacity::FavoriteCapacity;
use snaplingo_core::library::capacity::FavoriteCapacityPolicyProvider;
use snaplingo_core::library::favorites::FavoriteAssetStore;
use snaplingo_core::library::favorites::FavoriteContent;
use snaplingo_core::library::favorites::Favorites;
use snaplingo_core::library::favorites::OcrFavoriteApplication;
use snaplingo_core::library::favorites::OcrFavoriteRecognizer;
use snaplingo_core::library::favorites::StoredFavoriteAssets;
use snaplingo_core::library::favorites::TranslationFavoriteSnapshot;
use snaplingo_core::library::history::History;
use snaplingo_core::library::history::HistoryChangeNotifier;
use snaplingo_core::library::history::OcrHistoryAssetStore;
use snaplingo_core::library::history::OcrHistoryRecognizer;
use snaplingo_core::library::history::OcrHistoryReplay;
use snaplingo_core::library::history::StoredOcrHistoryAssets;
use snaplingo_core::library::index::LibraryClipboard;
use snaplingo_core::library::index::LibraryFilter;
use snaplingo_core::library::index::LibraryIndex;
use snaplingo_core::library::index::LibraryIndexPage;
use snaplingo_core::library::index::LibraryIndexQuery;
use snaplingo_core::library::index::LibraryIndexRepository;
use snaplingo_core::library::index::LibraryKind;
use snaplingo_core::library::index::LibraryPageRequest;
use snaplingo_core::library::index::LibraryPages;
use snaplingo_core::library::index::LibraryRecord;
use snaplingo_core::library::screenshot_favorites::ScreenshotFavoriteAssetStore;
use snaplingo_core::library::screenshot_favorites::ScreenshotFavoriteClipboard;
use snaplingo_core::library::screenshot_favorites::ScreenshotFavoriteHost;
use snaplingo_core::library::screenshot_favorites::ScreenshotFavorites;
use snaplingo_core::library::screenshot_favorites::StoredScreenshotAssets;
use snaplingo_core::providers::ocr::model::OcrResult;
use snaplingo_core::providers::translation::model::TranslationResult;

#[derive(Default)]
struct Assets {
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

fn pages(
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
fn history(database: &Database, kind: &str, second: i64, text: &str) -> i64 {
    database.with_connection(|connection| {
        connection.execute("INSERT INTO history_records(kind,timestamp) VALUES (?1,?2)", rusqlite::params![kind,Utc.timestamp_opt(second,0).unwrap().to_rfc3339()]).map_err(snaplingo_core::AppError::database)?;
        let id=connection.last_insert_rowid();
        if kind=="translation" {
            let results=serde_json::to_string(&vec![TranslationResult::success("system",format!("translated {text}"),None,None)])?;
            connection.execute("INSERT INTO translation_history(history_id,source_text,source_lang,target_lang,providers_used,results,duration_ms) VALUES (?1,?2,'en','zh-CN','[\"system\"]',?3,1)",rusqlite::params![id,text,results]).map_err(snaplingo_core::AppError::database)?;
        } else {
            connection.execute("INSERT INTO ocr_history(history_id,image_hash,provider_used,recognized_text,duration_ms,source_asset_path,thumbnail_asset_path) VALUES (?1,?2,'system',?3,1,?4,?5)",rusqlite::params![id,format!("hash-{id}"),text,format!("source-{id}"),format!("thumb-{id}")]).map_err(snaplingo_core::AppError::database)?;
        }
        Ok(id)
    }).unwrap()
}
fn screenshot(database: &Database, second: i64) -> i64 {
    database.with_connection(|connection| {
        connection.execute("INSERT INTO screenshot_favorites(created_at,asset_path,thumbnail_path,width,height) VALUES (?1,?2,?3,10,20)",rusqlite::params![Utc.timestamp_opt(second,0).unwrap().to_rfc3339(),format!("image-{second}"),format!("screen-thumb-{second}")]).map_err(snaplingo_core::AppError::database)?;
        Ok(connection.last_insert_rowid())
    }).unwrap()
}

#[tokio::test]
async fn global_history_page_hydrates_only_final_ids_and_membership_without_favorite_assets() {
    let database = Arc::new(Database::in_memory().unwrap());
    let assets = Arc::new(Assets::default());
    for second in 1..=45 {
        history(
            &database,
            if second % 2 == 0 {
                "ocr"
            } else {
                "translation"
            },
            second,
            &format!("item-{second}"),
        );
    }
    let content = FavoriteContent::Translation(TranslationFavoriteSnapshot {
        source_text: "item-25".into(),
        source_lang: "en".into(),
        target_lang: "zh-CN".into(),
        result: TranslationResult::success("system", "translated item-25", None, None),
    });
    database.with_connection(|connection| {
        connection.execute("INSERT INTO favorites(kind,created_at,fingerprint,content_json) VALUES ('translation','2026-01-01','membership',?1)",[serde_json::to_string(&content)?]).map_err(snaplingo_core::AppError::database)?;
        Ok(())
    }).unwrap();
    let library = pages(
        database.clone(),
        assets.clone(),
        Arc::new(SqliteLibraryIndexRepository::new(database)),
    );
    let result = library
        .query(
            LibraryKind::History,
            &LibraryPageRequest {
                page: 1,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(result.total, 45);
    assert_eq!(
        result
            .items
            .iter()
            .map(LibraryRecord::id)
            .collect::<Vec<_>>(),
        (6..=25).rev().collect::<Vec<_>>()
    );
    assert_eq!(assets.reads.lock().unwrap().len(), 10);
    assert!(assets
        .reads
        .lock()
        .unwrap()
        .iter()
        .all(|path| (6..=24).step_by(2).any(|id| path == &format!("thumb-{id}"))));
    assert_eq!(result.favorite_keys.len(), 1);
    assert!(result
        .favorite_keys
        .contains(&result.items[0].favorite_keys()[0]));
}

#[tokio::test]
async fn screenshot_favorites_page_reads_only_twenty_thumbnails_and_routes_copy_reveal() {
    let database = Arc::new(Database::in_memory().unwrap());
    let assets = Arc::new(Assets::default());
    for second in 1..=120 {
        screenshot(&database, second);
    }
    let library = pages(
        database.clone(),
        assets.clone(),
        Arc::new(SqliteLibraryIndexRepository::new(database)),
    );
    let result = library
        .query(
            LibraryKind::Favorites,
            &LibraryPageRequest {
                page: 5,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(result.total, 120);
    assert_eq!(
        result
            .items
            .iter()
            .map(LibraryRecord::id)
            .collect::<Vec<_>>(),
        (1..=20).rev().collect::<Vec<_>>()
    );
    assert_eq!(assets.reads.lock().unwrap().len(), 20);
    library.copy_screenshot(&result.items[0]).await.unwrap();
    library.reveal_screenshot(&result.items[0]).await.unwrap();
    assert_eq!(*assets.copied.lock().unwrap(), [b"image-20".to_vec()]);
    assert_eq!(*assets.revealed.lock().unwrap(), ["/assets/image-20"]);
}

#[tokio::test]
async fn deleting_last_history_page_resolves_to_the_last_nonempty_page() {
    let database = Arc::new(Database::in_memory().unwrap());
    for second in 1..=21 {
        history(&database, "translation", second, "text");
    }
    let library = pages(
        database.clone(),
        Arc::new(Assets::default()),
        Arc::new(SqliteLibraryIndexRepository::new(database)),
    );
    let request = LibraryPageRequest {
        page: 1,
        ..Default::default()
    };
    let result = library.query(LibraryKind::History, &request).await.unwrap();
    assert_eq!(result.items.len(), 1);
    library.delete(&result.items[0]).await.unwrap();
    let result = library.query(LibraryKind::History, &request).await.unwrap();
    assert_eq!((result.page, result.total, result.items.len()), (0, 20, 20));
    library.clear_history(LibraryFilter::All).await.unwrap();
    let result = library.query(LibraryKind::History, &request).await.unwrap();
    assert_eq!((result.page, result.total, result.items.len()), (0, 0, 0));
}

struct ChangingIndex {
    database: Arc<Database>,
}
#[async_trait]
impl LibraryIndexRepository for ChangingIndex {
    async fn query_history(&self, query: &LibraryIndexQuery) -> Result<LibraryIndexPage> {
        let result = SqliteLibraryIndexRepository::new(self.database.clone())
            .query_history(query)
            .await?;
        history(&self.database, "translation", 100, "inserted after index");
        self.database.with_connection(|connection| {
            connection
                .execute("DELETE FROM history_records WHERE id=2", [])
                .map_err(snaplingo_core::AppError::database)?;
            Ok(())
        })?;
        Ok(result)
    }
    async fn query_favorites(&self, query: &LibraryIndexQuery) -> Result<LibraryIndexPage> {
        SqliteLibraryIndexRepository::new(self.database.clone())
            .query_favorites(query)
            .await
    }
}
#[tokio::test]
async fn indexed_ids_survive_concurrent_insert_and_skip_deletion_without_refilling_the_page() {
    let database = Arc::new(Database::in_memory().unwrap());
    history(&database, "translation", 1, "first");
    history(&database, "ocr", 2, "deleted");
    history(&database, "translation", 3, "last");
    let library = pages(
        database.clone(),
        Arc::new(Assets::default()),
        Arc::new(ChangingIndex { database }),
    );
    let result = library
        .query(LibraryKind::History, &LibraryPageRequest::default())
        .await
        .unwrap();
    assert_eq!(
        result
            .items
            .iter()
            .map(LibraryRecord::key)
            .collect::<Vec<_>>(),
        ["translation:3", "translation:1"]
    );
    assert_eq!(result.total, 3);
}

#[tokio::test]
async fn cross_source_favorites_keep_colliding_ids_distinct_and_search_source_metadata() {
    let database = Arc::new(Database::in_memory().unwrap());
    screenshot(&database, 2);
    screenshot(&database, 1);
    let content = FavoriteContent::Translation(TranslationFavoriteSnapshot {
        source_text: "source".into(),
        source_lang: "en".into(),
        target_lang: "zh-CN".into(),
        result: TranslationResult::success("system", "translated", None, None),
    });
    database.with_connection(|connection| {
        connection.execute("INSERT INTO favorites(kind,created_at,fingerprint,content_json) VALUES ('translation',?1,'regular',?2)",rusqlite::params![Utc.timestamp_opt(2,0).unwrap().to_rfc3339(),serde_json::to_string(&content)?]).map_err(snaplingo_core::AppError::database)?;
        Ok(())
    }).unwrap();
    let library = pages(
        database.clone(),
        Arc::new(Assets::default()),
        Arc::new(SqliteLibraryIndexRepository::new(database)),
    );
    let result = library
        .query(LibraryKind::Favorites, &LibraryPageRequest::default())
        .await
        .unwrap();
    assert_eq!(
        result
            .items
            .iter()
            .map(LibraryRecord::key)
            .collect::<Vec<_>>(),
        ["translation:1", "screenshot:1", "screenshot:2"]
    );
    library
        .update_metadata(
            &result.items[1],
            Some("snapshot note".into()),
            vec!["work".into()],
        )
        .await
        .unwrap();
    let result = library
        .query(
            LibraryKind::Favorites,
            &LibraryPageRequest {
                search: "work".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(
        result
            .items
            .iter()
            .map(LibraryRecord::key)
            .collect::<Vec<_>>(),
        ["screenshot:1"]
    );
    library.delete(&result.items[0]).await.unwrap();
    let result = library
        .query(LibraryKind::Favorites, &LibraryPageRequest::default())
        .await
        .unwrap();
    assert_eq!(
        result
            .items
            .iter()
            .map(LibraryRecord::key)
            .collect::<Vec<_>>(),
        ["translation:1", "screenshot:2"]
    );
}

#[tokio::test]
async fn history_rerun_copies_after_recognition_and_rejects_an_unrelated_record() {
    let database = Arc::new(Database::in_memory().unwrap());
    history(&database, "ocr", 1, "original");
    history(&database, "translation", 2, "unrelated");
    let assets = Arc::new(Assets::default());
    let library = pages(
        database.clone(),
        assets.clone(),
        Arc::new(SqliteLibraryIndexRepository::new(database)),
    );
    let result = library
        .query(LibraryKind::History, &LibraryPageRequest::default())
        .await
        .unwrap();
    library
        .rerun_ocr_and_copy(&result.items[1], assets.as_ref())
        .await
        .unwrap();
    assert_eq!(*assets.text.lock().unwrap(), ["rerun"]);
    assert!(assets.reads.lock().unwrap().contains(&"source-1".into()));
    assert!(library
        .rerun_ocr_and_copy(&result.items[0], assets.as_ref())
        .await
        .is_err());
    assert_eq!(assets.text.lock().unwrap().len(), 1);
}

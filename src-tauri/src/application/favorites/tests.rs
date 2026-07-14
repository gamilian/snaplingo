use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use super::*;
use crate::application::history::{EventSubscriber, History, HistoryRepository};
use crate::domain::events::DomainEvent;
use crate::infrastructure::storage::{Database, SqliteFavoriteRepository, SqliteHistoryRepository};

#[derive(Default)]
struct FakeRepository {
    records: Mutex<Vec<(String, FavoriteRecord)>>,
}

#[async_trait]
impl FavoriteRepository for FakeRepository {
    async fn find_by_fingerprint(&self, fingerprint: &str) -> Result<Option<FavoriteRecord>> {
        Ok(self
            .records
            .lock()
            .unwrap()
            .iter()
            .find(|(value, _)| value == fingerprint)
            .map(|(_, record)| record.clone()))
    }

    async fn insert(
        &self,
        fingerprint: &str,
        source_history_id: Option<i64>,
        content: &FavoriteContent,
        created_at: DateTime<Utc>,
    ) -> Result<FavoriteRecord> {
        let mut records = self.records.lock().unwrap();
        let record = FavoriteRecord {
            id: records.len() as i64 + 1,
            created_at,
            source_history_id,
            content: content.clone(),
            note: None,
            tags: Vec::new(),
            thumbnail_data_url: None,
        };
        records.push((fingerprint.to_string(), record.clone()));
        Ok(record)
    }

    async fn query(&self, query: &FavoriteQuery) -> Result<FavoritePage> {
        let items = self
            .records
            .lock()
            .unwrap()
            .iter()
            .map(|(_, record)| record.clone())
            .filter(|record| query.kind.is_none_or(|kind| record.content.kind() == kind))
            .collect::<Vec<_>>();
        Ok(FavoritePage {
            total: items.len(),
            items,
        })
    }

    async fn find(&self, id: i64) -> Result<Option<FavoriteRecord>> {
        Ok(self
            .records
            .lock()
            .unwrap()
            .iter()
            .find(|(_, record)| record.id == id)
            .map(|(_, record)| record.clone()))
    }

    async fn update_metadata(
        &self,
        _id: i64,
        _note: Option<String>,
        _tags: Vec<String>,
    ) -> Result<()> {
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<()> {
        self.records
            .lock()
            .unwrap()
            .retain(|(_, record)| record.id != id);
        Ok(())
    }

    async fn list_tags(&self, _kind: FavoriteKind) -> Result<Vec<String>> {
        Ok(Vec::new())
    }
}

#[derive(Default)]
struct FakeAssets {
    files: Mutex<HashMap<String, Vec<u8>>>,
}

impl FavoriteAssetStore for FakeAssets {
    fn store_ocr(&self, image_data: &[u8]) -> Result<StoredFavoriteAssets> {
        self.files
            .lock()
            .unwrap()
            .insert("source.png".to_string(), image_data.to_vec());
        self.files
            .lock()
            .unwrap()
            .insert("thumbnail.png".to_string(), vec![9, 8, 7]);
        Ok(StoredFavoriteAssets {
            source_path: "source.png".to_string(),
            thumbnail_path: "thumbnail.png".to_string(),
        })
    }

    fn read(&self, relative_path: &str) -> Result<Vec<u8>> {
        self.files
            .lock()
            .unwrap()
            .get(relative_path)
            .cloned()
            .ok_or_else(|| format!("missing asset {relative_path}").into())
    }

    fn delete(&self, relative_path: &str) -> Result<()> {
        self.files.lock().unwrap().remove(relative_path);
        Ok(())
    }
}

#[tokio::test]
async fn translation_favorite_is_an_independent_snapshot_and_is_deduplicated() {
    let repository = Arc::new(FakeRepository::default());
    let favorites = Favorites::new(repository.clone(), Arc::new(FakeAssets::default()));
    let request = TranslationRequest {
        text: "hello".to_string(),
        source_lang: "en".to_string(),
        target_lang: "zh-CN".to_string(),
    };
    let result = TranslationResult {
        provider_id: "google".to_string(),
        translated_text: "你好".to_string(),
        detected_language: Some("en".to_string()),
        confidence: None,
    };

    let first = favorites
        .add_translation(Some(41), request.clone(), result.clone())
        .await
        .unwrap();
    let duplicate = favorites
        .add_translation(None, request, result)
        .await
        .unwrap();

    assert_eq!(first.id, duplicate.id);
    assert_eq!(first.source_history_id, Some(41));
    assert_eq!(repository.records.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn ocr_favorite_owns_its_snapshot_and_thumbnail() {
    let favorites = Favorites::new(
        Arc::new(FakeRepository::default()),
        Arc::new(FakeAssets::default()),
    );

    let saved = favorites
        .add_ocr(
            Some(7),
            vec![1, 2, 3],
            Some("en".to_string()),
            "system-ocr".to_string(),
            OcrResult {
                text: "recognized".to_string(),
                confidence: Some(0.9),
            },
        )
        .await
        .unwrap();

    assert_eq!(saved.source_history_id, Some(7));
    assert_eq!(saved.content.kind(), FavoriteKind::Ocr);
    assert_eq!(
        saved.thumbnail_data_url.as_deref(),
        Some("data:image/png;base64,CQgH")
    );
}

#[tokio::test]
async fn equal_ocr_text_from_different_images_creates_distinct_favorites() {
    let repository = Arc::new(FakeRepository::default());
    let favorites = Favorites::new(repository.clone(), Arc::new(FakeAssets::default()));

    for image in [vec![1, 2, 3], vec![4, 5, 6]] {
        favorites
            .add_ocr(
                None,
                image,
                None,
                "system-ocr".to_string(),
                OcrResult {
                    text: "same text".to_string(),
                    confidence: None,
                },
            )
            .await
            .unwrap();
    }

    assert_eq!(repository.records.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn clearing_history_does_not_remove_an_independent_favorite_snapshot() {
    let database = Arc::new(Database::in_memory().unwrap());
    let history_repository: Arc<dyn HistoryRepository> =
        Arc::new(SqliteHistoryRepository::new(database.clone()));
    let history = History::new(history_repository);
    let favorites = Favorites::new(
        Arc::new(SqliteFavoriteRepository::new(database)),
        Arc::new(FakeAssets::default()),
    );
    let request = TranslationRequest {
        text: "hello".to_string(),
        source_lang: "en".to_string(),
        target_lang: "zh-CN".to_string(),
    };
    let result = TranslationResult {
        provider_id: "google".to_string(),
        translated_text: "你好".to_string(),
        detected_language: Some("en".to_string()),
        confidence: None,
    };
    history
        .handle(&DomainEvent::TranslationCompleted {
            request: request.clone(),
            results: vec![result.clone()],
            providers_used: vec!["google".to_string()],
            timestamp: Utc::now(),
            duration_ms: 12,
        })
        .await;
    let history_id = history.get_translation_history(1, 0).await.unwrap()[0].id;
    favorites
        .add_translation(Some(history_id), request, result)
        .await
        .unwrap();

    history.clear_all_history().await.unwrap();

    let page = favorites
        .query(FavoriteQuery {
            kind: Some(FavoriteKind::Translation),
            limit: 20,
            ..FavoriteQuery::default()
        })
        .await
        .unwrap();
    assert_eq!(page.total, 1);
    assert_eq!(page.items[0].source_history_id, Some(history_id));
}

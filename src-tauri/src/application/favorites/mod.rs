use std::sync::Arc;

use async_trait::async_trait;
use base64::Engine;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::application::favorite_capacity::FavoriteCapacity;
use crate::domain::ocr::OcrResult;
use crate::domain::translation::{TranslationRequest, TranslationResult};
use crate::Result;

mod ocr_application;

#[cfg(test)]
mod tests;

pub use ocr_application::{
    OcrFavoriteApplication, OcrFavoriteHistory, OcrFavoriteRecognizer, OcrFavoriteStore,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FavoriteKind {
    Translation,
    Ocr,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TranslationFavoriteSnapshot {
    pub source_text: String,
    pub source_lang: String,
    pub target_lang: String,
    pub result: TranslationResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OcrFavoriteSnapshot {
    pub image_hash: String,
    pub recognized_text: String,
    pub language: Option<String>,
    pub provider_used: String,
    pub confidence: Option<f32>,
    pub source_asset_path: Option<String>,
    pub thumbnail_asset_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "contentKind", content = "snapshot", rename_all = "lowercase")]
pub enum FavoriteContent {
    Translation(TranslationFavoriteSnapshot),
    Ocr(OcrFavoriteSnapshot),
}

impl FavoriteContent {
    pub fn kind(&self) -> FavoriteKind {
        match self {
            Self::Translation(_) => FavoriteKind::Translation,
            Self::Ocr(_) => FavoriteKind::Ocr,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FavoriteRecord {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub source_history_id: Option<i64>,
    pub content: FavoriteContent,
    pub note: Option<String>,
    pub tags: Vec<String>,
    pub thumbnail_data_url: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FavoriteQuery {
    pub kind: Option<FavoriteKind>,
    pub search: Option<String>,
    pub tag: Option<String>,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FavoritePage {
    pub items: Vec<FavoriteRecord>,
    pub total: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredFavoriteAssets {
    pub source_path: String,
    pub thumbnail_path: String,
}

#[async_trait]
pub trait FavoriteRepository: Send + Sync {
    async fn find_by_fingerprint(&self, fingerprint: &str) -> Result<Option<FavoriteRecord>>;
    async fn insert(
        &self,
        fingerprint: &str,
        source_history_id: Option<i64>,
        content: &FavoriteContent,
        created_at: DateTime<Utc>,
    ) -> Result<FavoriteRecord>;
    async fn query(&self, query: &FavoriteQuery) -> Result<FavoritePage>;
    async fn find(&self, id: i64) -> Result<Option<FavoriteRecord>>;
    async fn update_metadata(&self, id: i64, note: Option<String>, tags: Vec<String>)
        -> Result<()>;
    async fn delete(&self, id: i64) -> Result<()>;
    async fn list_tags(&self, kind: FavoriteKind) -> Result<Vec<String>>;
}

pub trait FavoriteAssetStore: Send + Sync {
    fn store_ocr(&self, image_data: &[u8]) -> Result<StoredFavoriteAssets>;
    fn read(&self, relative_path: &str) -> Result<Vec<u8>>;
    fn delete(&self, relative_path: &str) -> Result<()>;
}

pub trait FavoriteChangeNotifier: Send + Sync {
    fn favorites_changed(&self);
}

pub struct Favorites {
    repository: Arc<dyn FavoriteRepository>,
    assets: Arc<dyn FavoriteAssetStore>,
    notifier: Option<Arc<dyn FavoriteChangeNotifier>>,
    capacity: Arc<FavoriteCapacity>,
}

impl Favorites {
    pub fn new(
        repository: Arc<dyn FavoriteRepository>,
        assets: Arc<dyn FavoriteAssetStore>,
        capacity: Arc<FavoriteCapacity>,
    ) -> Self {
        Self {
            repository,
            assets,
            notifier: None,
            capacity,
        }
    }

    pub fn with_notifier(
        repository: Arc<dyn FavoriteRepository>,
        assets: Arc<dyn FavoriteAssetStore>,
        notifier: Arc<dyn FavoriteChangeNotifier>,
        capacity: Arc<FavoriteCapacity>,
    ) -> Self {
        Self {
            repository,
            assets,
            notifier: Some(notifier),
            capacity,
        }
    }

    pub fn with_notifier_and_capacity(
        repository: Arc<dyn FavoriteRepository>,
        assets: Arc<dyn FavoriteAssetStore>,
        notifier: Arc<dyn FavoriteChangeNotifier>,
        capacity: Arc<FavoriteCapacity>,
    ) -> Self {
        Self {
            repository,
            assets,
            notifier: Some(notifier),
            capacity,
        }
    }

    pub async fn add_translation(
        &self,
        source_history_id: Option<i64>,
        request: TranslationRequest,
        result: TranslationResult,
    ) -> Result<FavoriteRecord> {
        let content = FavoriteContent::Translation(TranslationFavoriteSnapshot {
            source_text: request.text,
            source_lang: request.source_lang,
            target_lang: request.target_lang,
            result,
        });
        self.add(source_history_id, content).await
    }

    pub async fn add_ocr(
        &self,
        source_history_id: Option<i64>,
        image_data: Vec<u8>,
        language: Option<String>,
        provider_used: String,
        result: OcrResult,
    ) -> Result<FavoriteRecord> {
        let image_hash = format!("{:x}", md5::compute(&image_data));
        let fingerprint = content_fingerprint(&FavoriteContent::Ocr(OcrFavoriteSnapshot {
            image_hash: image_hash.clone(),
            recognized_text: result.text.clone(),
            language: language.clone(),
            provider_used: provider_used.clone(),
            confidence: result.confidence,
            source_asset_path: None,
            thumbnail_asset_path: None,
        }))?;
        if let Some(record) = self.repository.find_by_fingerprint(&fingerprint).await? {
            return self.hydrate(record);
        }

        let record = self
            .capacity
            .add_idempotent(
                || self.repository.find_by_fingerprint(&fingerprint),
                || async {
                    let stored = if image_data.is_empty() {
                        None
                    } else {
                        Some(self.assets.store_ocr(&image_data)?)
                    };
                    let content = FavoriteContent::Ocr(OcrFavoriteSnapshot {
                        image_hash,
                        recognized_text: result.text,
                        language,
                        provider_used,
                        confidence: result.confidence,
                        source_asset_path: stored.as_ref().map(|assets| assets.source_path.clone()),
                        thumbnail_asset_path: stored
                            .as_ref()
                            .map(|assets| assets.thumbnail_path.clone()),
                    });
                    match self
                        .repository
                        .insert(&fingerprint, source_history_id, &content, Utc::now())
                        .await
                    {
                        Ok(record) => {
                            self.notify();
                            Ok(record)
                        }
                        Err(error) => {
                            if let Some(stored) = stored {
                                let _ = self.assets.delete(&stored.source_path);
                                let _ = self.assets.delete(&stored.thumbnail_path);
                            }
                            Err(error)
                        }
                    }
                },
            )
            .await?;
        self.hydrate(record)
    }

    pub async fn query(&self, query: FavoriteQuery) -> Result<FavoritePage> {
        let mut page = self.repository.query(&query).await?;
        page.items = page
            .items
            .into_iter()
            .map(|record| self.hydrate(record))
            .collect::<Result<Vec<_>>>()?;
        Ok(page)
    }

    pub async fn update_metadata(
        &self,
        id: i64,
        note: Option<String>,
        tags: Vec<String>,
    ) -> Result<()> {
        self.repository.update_metadata(id, note, tags).await?;
        self.notify();
        Ok(())
    }

    pub async fn delete(&self, id: i64) -> Result<()> {
        let record = self.repository.find(id).await?;
        self.repository.delete(id).await?;
        if let Some(FavoriteContent::Ocr(snapshot)) = record.map(|record| record.content) {
            if let Some(path) = snapshot.source_asset_path {
                let _ = self.assets.delete(&path);
            }
            if let Some(path) = snapshot.thumbnail_asset_path {
                let _ = self.assets.delete(&path);
            }
        }
        self.notify();
        Ok(())
    }

    pub async fn read_ocr_source(&self, id: i64) -> Result<Vec<u8>> {
        let record = self
            .repository
            .find(id)
            .await?
            .ok_or_else(|| crate::AppError::Other(format!("Favorite {id} not found")))?;
        let FavoriteContent::Ocr(snapshot) = record.content else {
            return Err(crate::AppError::Other(format!(
                "Favorite {id} is not an OCR result"
            )));
        };
        let path = snapshot.source_asset_path.ok_or_else(|| {
            crate::AppError::Other(format!("OCR favorite {id} has no source image"))
        })?;
        self.assets.read(&path)
    }

    pub async fn list_tags(&self, kind: FavoriteKind) -> Result<Vec<String>> {
        self.repository.list_tags(kind).await
    }

    async fn add(
        &self,
        source_history_id: Option<i64>,
        content: FavoriteContent,
    ) -> Result<FavoriteRecord> {
        let fingerprint = content_fingerprint(&content)?;
        if let Some(record) = self.repository.find_by_fingerprint(&fingerprint).await? {
            return Ok(record);
        }
        self.capacity
            .add_idempotent(
                || self.repository.find_by_fingerprint(&fingerprint),
                || async {
                    let record = self
                        .repository
                        .insert(&fingerprint, source_history_id, &content, Utc::now())
                        .await?;
                    self.notify();
                    Ok(record)
                },
            )
            .await
    }

    fn hydrate(&self, mut record: FavoriteRecord) -> Result<FavoriteRecord> {
        if let FavoriteContent::Ocr(snapshot) = &record.content {
            if let Some(path) = snapshot.thumbnail_asset_path.as_deref() {
                record.thumbnail_data_url = Some(format!(
                    "data:image/png;base64,{}",
                    base64::engine::general_purpose::STANDARD.encode(self.assets.read(path)?)
                ));
            }
        }
        Ok(record)
    }

    fn notify(&self) {
        if let Some(notifier) = &self.notifier {
            notifier.favorites_changed();
        }
    }
}

fn content_fingerprint(content: &FavoriteContent) -> Result<String> {
    Ok(format!("{:x}", md5::compute(serde_json::to_vec(content)?)))
}

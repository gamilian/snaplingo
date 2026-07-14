use std::sync::Arc;

use async_trait::async_trait;
use base64::Engine;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::capture::{AnnotationCommand, CaptureSessionId, LogicalRect};
use crate::Result;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotFavoriteRecord {
    pub id: i64,
    pub content_kind: String,
    pub created_at: DateTime<Utc>,
    pub asset_path: String,
    pub thumbnail_path: String,
    pub width: u32,
    pub height: u32,
    pub note: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NewScreenshotFavorite {
    pub created_at: DateTime<Utc>,
    pub asset_path: String,
    pub thumbnail_path: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotFavoriteQuery {
    pub search: Option<String>,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotFavoriteItem {
    pub id: i64,
    pub content_kind: String,
    pub created_at: DateTime<Utc>,
    pub thumbnail_data_url: String,
    pub width: u32,
    pub height: u32,
    pub note: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScreenshotFavoritePage {
    pub items: Vec<ScreenshotFavoriteItem>,
    pub total: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredScreenshotAssets {
    pub asset_path: String,
    pub thumbnail_path: String,
    pub width: u32,
    pub height: u32,
}

#[async_trait]
pub trait ScreenshotFavoriteRepository: Send + Sync {
    async fn insert(&self, favorite: &NewScreenshotFavorite) -> Result<ScreenshotFavoriteRecord>;
    async fn query(
        &self,
        query: &ScreenshotFavoriteQuery,
    ) -> Result<(Vec<ScreenshotFavoriteRecord>, usize)>;
    async fn find(&self, id: i64) -> Result<Option<ScreenshotFavoriteRecord>>;
    async fn update_metadata(&self, id: i64, note: Option<String>, tags: Vec<String>)
        -> Result<()>;
    async fn delete(&self, id: i64) -> Result<()>;
}

pub trait ScreenshotFavoriteAssetStore: Send + Sync {
    fn store(&self, png_data: &[u8]) -> Result<StoredScreenshotAssets>;
    fn read(&self, relative_path: &str) -> Result<Vec<u8>>;
    fn delete(&self, relative_path: &str) -> Result<()>;
    fn absolute_path(&self, relative_path: &str) -> Result<String>;
}

#[async_trait]
pub trait ScreenshotFavoriteClipboard: Send + Sync {
    async fn copy_png(&self, png_data: &[u8]) -> Result<()>;
}

pub trait ScreenshotFavoriteHost: Send + Sync {
    fn reveal(&self, absolute_path: &str) -> Result<()>;
}

pub trait ScreenshotFavoriteChangeNotifier: Send + Sync {
    fn screenshot_favorites_changed(&self);
}

pub trait ScreenshotFavoriteCaptureRenderer: Send + Sync {
    fn render_png(
        &self,
        session_id: &CaptureSessionId,
        rect: &LogicalRect,
        annotations: &[AnnotationCommand],
        include_cursor: bool,
    ) -> Result<Vec<u8>>;
}

pub struct ScreenshotFavorites {
    repository: Arc<dyn ScreenshotFavoriteRepository>,
    assets: Arc<dyn ScreenshotFavoriteAssetStore>,
    clipboard: Arc<dyn ScreenshotFavoriteClipboard>,
    host: Arc<dyn ScreenshotFavoriteHost>,
    change_notifier: Option<Arc<dyn ScreenshotFavoriteChangeNotifier>>,
}

pub struct ScreenshotFavoriteCapture {
    favorites: Arc<ScreenshotFavorites>,
    renderer: Arc<dyn ScreenshotFavoriteCaptureRenderer>,
}

impl ScreenshotFavoriteCapture {
    pub fn new(
        favorites: Arc<ScreenshotFavorites>,
        renderer: Arc<dyn ScreenshotFavoriteCaptureRenderer>,
    ) -> Self {
        Self {
            favorites,
            renderer,
        }
    }

    pub async fn add_selection(
        &self,
        session_id: &CaptureSessionId,
        rect: &LogicalRect,
        annotations: &[AnnotationCommand],
        include_cursor: bool,
    ) -> Result<ScreenshotFavoriteRecord> {
        let png = self
            .renderer
            .render_png(session_id, rect, annotations, include_cursor)?;
        self.favorites.add_png(&png).await
    }
}

impl ScreenshotFavorites {
    pub fn new(
        repository: Arc<dyn ScreenshotFavoriteRepository>,
        assets: Arc<dyn ScreenshotFavoriteAssetStore>,
        clipboard: Arc<dyn ScreenshotFavoriteClipboard>,
        host: Arc<dyn ScreenshotFavoriteHost>,
    ) -> Self {
        Self {
            repository,
            assets,
            clipboard,
            host,
            change_notifier: None,
        }
    }

    pub fn with_change_notifier(
        repository: Arc<dyn ScreenshotFavoriteRepository>,
        assets: Arc<dyn ScreenshotFavoriteAssetStore>,
        clipboard: Arc<dyn ScreenshotFavoriteClipboard>,
        host: Arc<dyn ScreenshotFavoriteHost>,
        change_notifier: Arc<dyn ScreenshotFavoriteChangeNotifier>,
    ) -> Self {
        Self {
            repository,
            assets,
            clipboard,
            host,
            change_notifier: Some(change_notifier),
        }
    }

    fn notify_changed(&self) {
        if let Some(notifier) = &self.change_notifier {
            notifier.screenshot_favorites_changed();
        }
    }

    pub async fn add_png(&self, png_data: &[u8]) -> Result<ScreenshotFavoriteRecord> {
        let stored = self.assets.store(png_data)?;
        let favorite = NewScreenshotFavorite {
            created_at: Utc::now(),
            asset_path: stored.asset_path.clone(),
            thumbnail_path: stored.thumbnail_path.clone(),
            width: stored.width,
            height: stored.height,
        };

        match self.repository.insert(&favorite).await {
            Ok(record) => {
                self.notify_changed();
                Ok(record)
            }
            Err(error) => {
                let _ = self.assets.delete(&stored.asset_path);
                let _ = self.assets.delete(&stored.thumbnail_path);
                Err(error)
            }
        }
    }

    pub async fn query(
        &self,
        mut query: ScreenshotFavoriteQuery,
    ) -> Result<ScreenshotFavoritePage> {
        query.limit = query.limit.clamp(1, 100);
        let (records, total) = self.repository.query(&query).await?;
        let items = records
            .into_iter()
            .map(|record| {
                let thumbnail = self.assets.read(&record.thumbnail_path)?;
                Ok(ScreenshotFavoriteItem {
                    id: record.id,
                    content_kind: record.content_kind,
                    created_at: record.created_at,
                    thumbnail_data_url: format!(
                        "data:image/png;base64,{}",
                        base64::engine::general_purpose::STANDARD.encode(thumbnail)
                    ),
                    width: record.width,
                    height: record.height,
                    note: record.note,
                    tags: record.tags,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(ScreenshotFavoritePage { items, total })
    }

    pub async fn update_metadata(
        &self,
        id: i64,
        note: Option<String>,
        tags: Vec<String>,
    ) -> Result<()> {
        self.repository.update_metadata(id, note, tags).await?;
        self.notify_changed();
        Ok(())
    }

    pub async fn delete(&self, id: i64) -> Result<()> {
        let record = self.repository.find(id).await?;
        self.repository.delete(id).await?;
        if let Some(record) = record {
            let _ = self.assets.delete(&record.asset_path);
            let _ = self.assets.delete(&record.thumbnail_path);
        }
        self.notify_changed();
        Ok(())
    }

    pub async fn copy(&self, id: i64) -> Result<()> {
        let record = self
            .repository
            .find(id)
            .await?
            .ok_or_else(|| format!("Screenshot favorite {} was not found", id))?;
        let png = self.assets.read(&record.asset_path)?;
        self.clipboard.copy_png(&png).await
    }

    pub async fn absolute_path(&self, id: i64) -> Result<String> {
        let record = self
            .repository
            .find(id)
            .await?
            .ok_or_else(|| format!("Screenshot favorite {} was not found", id))?;
        self.assets.absolute_path(&record.asset_path)
    }

    pub async fn reveal(&self, id: i64) -> Result<()> {
        let path = self.absolute_path(id).await?;
        self.host.reveal(&path)
    }
}

#[cfg(test)]
mod tests;

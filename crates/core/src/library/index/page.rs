//! Library pages hydrate only the IDs in the requested, ordered index page.
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use async_trait::async_trait;

use super::{LibraryIndex, LibraryIndexKind, LibraryIndexQuery};
use crate::library::favorites::FavoriteContent;
use crate::library::favorites::FavoriteKind;
use crate::library::favorites::FavoriteQuery;
use crate::library::favorites::FavoriteRecord;
use crate::library::favorites::Favorites;
use crate::library::favorites::OcrFavoriteApplication;
use crate::library::history::History;
use crate::library::history::HistoryKind;
use crate::library::history::HistoryQuery;
use crate::library::history::OcrHistoryEntry;
use crate::library::history::OcrHistoryReplay;
use crate::library::history::TranslationHistoryEntry;
use crate::library::screenshot_favorites::ScreenshotFavoriteItem;
use crate::library::screenshot_favorites::ScreenshotFavoriteQuery;
use crate::library::screenshot_favorites::ScreenshotFavorites;
use crate::providers::ocr::model::OcrRequest;
use crate::providers::ocr::model::OcrResult;
use crate::providers::translation::model::TranslationRequest;
use crate::Result;

pub const LIBRARY_PAGE_SIZE: usize = 20;

#[async_trait]
pub trait LibraryClipboard: Send + Sync {
    async fn write_text(&self, text: &str) -> Result<()>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibraryKind {
    History,
    Favorites,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LibraryFilter {
    #[default]
    All,
    Translation,
    Ocr,
    Screenshot,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LibraryPageRequest {
    pub filter: LibraryFilter,
    pub search: String,
    pub page: usize,
}

#[derive(Clone, Debug)]
pub enum LibraryRecord {
    TranslationHistory(TranslationHistoryEntry),
    OcrHistory(OcrHistoryEntry),
    Favorite(FavoriteRecord),
    Screenshot(ScreenshotFavoriteItem),
}

impl LibraryRecord {
    pub fn id(&self) -> i64 {
        match self {
            Self::TranslationHistory(e) => e.id,
            Self::OcrHistory(e) => e.id,
            Self::Favorite(e) => e.id,
            Self::Screenshot(e) => e.id,
        }
    }
    pub fn kind(&self) -> LibraryIndexKind {
        match self {
            Self::TranslationHistory(_) => LibraryIndexKind::Translation,
            Self::OcrHistory(_) => LibraryIndexKind::Ocr,
            Self::Screenshot(_) => LibraryIndexKind::Screenshot,
            Self::Favorite(e) => match e.content {
                FavoriteContent::Translation(_) => LibraryIndexKind::Translation,
                FavoriteContent::Ocr(_) => LibraryIndexKind::Ocr,
            },
        }
    }
    pub fn key(&self) -> String {
        record_key(self.kind(), self.id())
    }
    pub fn favorite_keys(&self) -> Vec<String> {
        match self {
            Self::TranslationHistory(e) => e
                .results
                .iter()
                .map(|r| {
                    translation_key(
                        &e.source_text,
                        &e.source_lang,
                        &e.target_lang,
                        &r.provider_id,
                        &r.translated_text,
                    )
                })
                .collect(),
            Self::OcrHistory(e) => vec![ocr_key(
                &e.recognized_text,
                e.language.as_deref(),
                &e.provider_used,
            )],
            _ => Vec::new(),
        }
    }
}

fn record_key(kind: LibraryIndexKind, id: i64) -> String {
    format!(
        "{}:{id}",
        match kind {
            LibraryIndexKind::Translation => "translation",
            LibraryIndexKind::Ocr => "ocr",
            LibraryIndexKind::Screenshot => "screenshot",
        }
    )
}

fn translation_key(text: &str, source: &str, target: &str, provider: &str, result: &str) -> String {
    serde_json::json!(["translation", text, source, target, provider, result]).to_string()
}
fn ocr_key(text: &str, language: Option<&str>, provider: &str) -> String {
    serde_json::json!(["ocr", text, language, provider]).to_string()
}

#[derive(Clone, Debug, Default)]
pub struct LibraryPageSnapshot {
    pub items: Vec<LibraryRecord>,
    pub total: usize,
    /// Resolved zero-based page after a deletion shrinks the requested range.
    pub page: usize,
    pub favorite_keys: HashSet<String>,
}

pub struct LibraryPages {
    history: Arc<History>,
    favorites: Arc<Favorites>,
    screenshots: Arc<ScreenshotFavorites>,
    index: Arc<LibraryIndex>,
    history_ocr: Arc<OcrHistoryReplay>,
    favorite_ocr: Arc<OcrFavoriteApplication>,
}

impl LibraryPages {
    pub fn new(
        history: Arc<History>,
        favorites: Arc<Favorites>,
        screenshots: Arc<ScreenshotFavorites>,
        index: Arc<LibraryIndex>,
        history_ocr: Arc<OcrHistoryReplay>,
        favorite_ocr: Arc<OcrFavoriteApplication>,
    ) -> Self {
        Self {
            history,
            favorites,
            screenshots,
            index,
            history_ocr,
            favorite_ocr,
        }
    }
    pub async fn query(
        &self,
        kind: LibraryKind,
        request: &LibraryPageRequest,
    ) -> Result<LibraryPageSnapshot> {
        let mut page = self.query_page(kind, request).await?;
        let last_page = page.total.saturating_sub(1) / LIBRARY_PAGE_SIZE;
        if request.page > last_page {
            page = self
                .query_page(
                    kind,
                    &LibraryPageRequest {
                        page: last_page,
                        ..request.clone()
                    },
                )
                .await?;
        }
        Ok(page)
    }

    async fn query_page(
        &self,
        kind: LibraryKind,
        request: &LibraryPageRequest,
    ) -> Result<LibraryPageSnapshot> {
        let offset = request.page.saturating_mul(LIBRARY_PAGE_SIZE);
        let search = Some(request.search.clone());
        let h = HistoryQuery {
            search: search.clone(),
            limit: LIBRARY_PAGE_SIZE,
            offset,
            ..Default::default()
        };
        let f = FavoriteQuery {
            search: search.clone(),
            limit: LIBRARY_PAGE_SIZE,
            offset,
            ..Default::default()
        };
        let s = ScreenshotFavoriteQuery {
            search,
            limit: LIBRARY_PAGE_SIZE,
            offset,
            ..Default::default()
        };
        let mut page = match (kind, request.filter) {
            (LibraryKind::History, LibraryFilter::Translation) => {
                let p = self.history.query_translation_history(h).await?;
                LibraryPageSnapshot {
                    items: p
                        .items
                        .into_iter()
                        .map(LibraryRecord::TranslationHistory)
                        .collect(),
                    total: p.total,
                    ..Default::default()
                }
            }
            (LibraryKind::History, LibraryFilter::Ocr) => {
                let p = self.history.query_ocr_history(h).await?;
                LibraryPageSnapshot {
                    items: p.items.into_iter().map(LibraryRecord::OcrHistory).collect(),
                    total: p.total,
                    ..Default::default()
                }
            }
            (LibraryKind::History, LibraryFilter::Screenshot) => {
                return Err("Screenshot history is not a Library source".into())
            }
            (LibraryKind::Favorites, LibraryFilter::Translation | LibraryFilter::Ocr) => {
                let p = self
                    .favorites
                    .query(FavoriteQuery {
                        kind: Some(if request.filter == LibraryFilter::Translation {
                            FavoriteKind::Translation
                        } else {
                            FavoriteKind::Ocr
                        }),
                        ..f
                    })
                    .await?;
                LibraryPageSnapshot {
                    items: p.items.into_iter().map(LibraryRecord::Favorite).collect(),
                    total: p.total,
                    ..Default::default()
                }
            }
            (LibraryKind::Favorites, LibraryFilter::Screenshot) => {
                let p = self.screenshots.query(s).await?;
                LibraryPageSnapshot {
                    items: p.items.into_iter().map(LibraryRecord::Screenshot).collect(),
                    total: p.total,
                    ..Default::default()
                }
            }
            (_, LibraryFilter::All) => {
                let query = LibraryIndexQuery {
                    search: request.search.clone(),
                    limit: LIBRARY_PAGE_SIZE,
                    offset,
                };
                let index = match kind {
                    LibraryKind::History => self.index.query_history(query).await?,
                    LibraryKind::Favorites => self.index.query_favorites(query).await?,
                };
                let ids = |source| {
                    index
                        .items
                        .iter()
                        .filter(|r| r.kind == source)
                        .map(|r| r.id)
                        .collect::<Vec<_>>()
                };
                let mut items = Vec::new();
                if kind == LibraryKind::History {
                    let translations = ids(LibraryIndexKind::Translation);
                    if !translations.is_empty() {
                        let p = self
                            .history
                            .query_translation_history(HistoryQuery {
                                limit: translations.len(),
                                ids: Some(translations),
                                offset: 0,
                                ..h.clone()
                            })
                            .await?;
                        items.extend(p.items.into_iter().map(LibraryRecord::TranslationHistory));
                    }
                    let ocr = ids(LibraryIndexKind::Ocr);
                    if !ocr.is_empty() {
                        let p = self
                            .history
                            .query_ocr_history(HistoryQuery {
                                limit: ocr.len(),
                                ids: Some(ocr),
                                offset: 0,
                                ..h
                            })
                            .await?;
                        items.extend(p.items.into_iter().map(LibraryRecord::OcrHistory));
                    }
                } else {
                    let favorites = index
                        .items
                        .iter()
                        .filter(|r| r.kind != LibraryIndexKind::Screenshot)
                        .map(|r| r.id)
                        .collect::<Vec<_>>();
                    if !favorites.is_empty() {
                        let p = self
                            .favorites
                            .query(FavoriteQuery {
                                limit: favorites.len(),
                                ids: Some(favorites),
                                offset: 0,
                                ..f
                            })
                            .await?;
                        items.extend(p.items.into_iter().map(LibraryRecord::Favorite));
                    }
                    let screenshots = ids(LibraryIndexKind::Screenshot);
                    if !screenshots.is_empty() {
                        let p = self
                            .screenshots
                            .query(ScreenshotFavoriteQuery {
                                limit: screenshots.len(),
                                ids: Some(screenshots),
                                offset: 0,
                                ..s
                            })
                            .await?;
                        items.extend(p.items.into_iter().map(LibraryRecord::Screenshot));
                    }
                }
                LibraryPageSnapshot {
                    total: index.total,
                    items: order_records(&index.items, items),
                    ..Default::default()
                }
            }
        };
        if kind == LibraryKind::History {
            let keys = page
                .items
                .iter()
                .flat_map(LibraryRecord::favorite_keys)
                .collect::<Vec<_>>();
            if !keys.is_empty() {
                page.favorite_keys = self.favorites.query_keys(keys).await?.into_iter().collect();
            }
        }
        page.page = request.page;
        Ok(page)
    }

    pub async fn delete(&self, record: &LibraryRecord) -> Result<()> {
        match record {
            LibraryRecord::TranslationHistory(e) => self.history.delete_history(e.id).await,
            LibraryRecord::OcrHistory(e) => self.history.delete_history(e.id).await,
            LibraryRecord::Favorite(e) => self.favorites.delete(e.id).await,
            LibraryRecord::Screenshot(e) => self.screenshots.delete(e.id).await,
        }
    }
    pub async fn clear_history(&self, filter: LibraryFilter) -> Result<()> {
        match filter {
            LibraryFilter::All => self.history.clear_all_history().await,
            LibraryFilter::Translation => {
                self.history.clear_history(HistoryKind::Translation).await
            }
            LibraryFilter::Ocr => self.history.clear_history(HistoryKind::Ocr).await,
            LibraryFilter::Screenshot => Err("Screenshot history is not a Library source".into()),
        }
    }
    pub async fn update_metadata(
        &self,
        record: &LibraryRecord,
        note: Option<String>,
        tags: Vec<String>,
    ) -> Result<()> {
        match record {
            LibraryRecord::Favorite(e) => self.favorites.update_metadata(e.id, note, tags).await,
            LibraryRecord::Screenshot(e) => {
                self.screenshots.update_metadata(e.id, note, tags).await
            }
            _ => Err("Only favorites have Library metadata editors".into()),
        }
    }
    pub async fn favorite(&self, record: &LibraryRecord, result_index: usize) -> Result<()> {
        match record {
            LibraryRecord::TranslationHistory(e) => {
                let result = e
                    .results
                    .get(result_index)
                    .ok_or("Translation result no longer exists")?;
                self.favorites
                    .add_translation(
                        Some(e.id),
                        TranslationRequest {
                            text: e.source_text.clone(),
                            source_lang: e.source_lang.clone(),
                            target_lang: e.target_lang.clone(),
                        },
                        result.clone(),
                    )
                    .await?;
            }
            LibraryRecord::OcrHistory(e) => {
                self.favorite_ocr
                    .favorite(
                        Some(e.id),
                        OcrRequest {
                            image_data: Vec::new(),
                            language: e.language.clone(),
                        },
                        OcrResult::from_text(
                            e.recognized_text.clone(),
                            e.confidence.map(|c| c as f32),
                        ),
                        Some(e.provider_used.clone()),
                    )
                    .await?;
            }
            _ => return Err("This Library item is already a favorite".into()),
        }
        Ok(())
    }
    pub async fn rerun_ocr(&self, record: &LibraryRecord) -> Result<String> {
        match record {
            LibraryRecord::OcrHistory(e) => Ok(self.history_ocr.run(e.id).await?.text),
            LibraryRecord::Favorite(e) if matches!(e.content, FavoriteContent::Ocr(_)) => {
                Ok(self.favorite_ocr.rerun(e.id).await?.text)
            }
            _ => Err("This Library item has no OCR source".into()),
        }
    }

    pub async fn rerun_ocr_and_copy(
        &self,
        record: &LibraryRecord,
        clipboard: &dyn LibraryClipboard,
    ) -> Result<()> {
        let text = self.rerun_ocr(record).await?;
        clipboard.write_text(&text).await
    }

    pub async fn copy_screenshot(&self, record: &LibraryRecord) -> Result<()> {
        match record {
            LibraryRecord::Screenshot(item) => self.screenshots.copy(item.id).await,
            _ => Err("This Library item is not a screenshot".into()),
        }
    }

    pub async fn reveal_screenshot(&self, record: &LibraryRecord) -> Result<()> {
        match record {
            LibraryRecord::Screenshot(item) => self.screenshots.reveal(item.id).await,
            _ => Err("This Library item is not a screenshot".into()),
        }
    }
}

fn order_records(
    index: &[super::LibraryIndexItem],
    records: Vec<LibraryRecord>,
) -> Vec<LibraryRecord> {
    let mut by_key = records
        .into_iter()
        .map(|record| (record.key(), record))
        .collect::<HashMap<_, _>>();
    index
        .iter()
        .filter_map(|entry| by_key.remove(&record_key(entry.kind, entry.id)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn translation(id: i64, source: &str) -> TranslationHistoryEntry {
        TranslationHistoryEntry {
            id,
            timestamp: chrono::Utc::now(),
            note: None,
            tags: Vec::new(),
            source_text: source.into(),
            source_lang: "en".into(),
            target_lang: "zh-CN".into(),
            providers_used: vec!["system".into()],
            results: Vec::new(),
            duration_ms: 1,
        }
    }

    fn ocr(id: i64, text: &str) -> OcrHistoryEntry {
        OcrHistoryEntry {
            id,
            timestamp: chrono::Utc::now(),
            note: None,
            tags: Vec::new(),
            image_hash: format!("hash-{id}"),
            language: Some("en".into()),
            provider_used: "system".into(),
            recognized_text: text.into(),
            confidence: None,
            duration_ms: 1,
            source_asset_path: None,
            thumbnail_asset_path: None,
            thumbnail_data_url: None,
        }
    }

    #[test]
    fn membership_keys_match_the_existing_frontend_identity() {
        assert_eq!(
            translation_key("a\n中文", "en", "zh", "system", "译文"),
            r#"["translation","a\n中文","en","zh","system","译文"]"#
        );
        assert_eq!(
            ocr_key("识别", None, "system"),
            r#"["ocr","识别",null,"system"]"#
        );
        assert_ne!(
            record_key(LibraryIndexKind::Ocr, 1),
            record_key(LibraryIndexKind::Translation, 1)
        );
    }

    #[test]
    fn indexed_page_order_is_global_and_missing_ids_are_skipped_after_deletion() {
        let index = vec![
            super::super::LibraryIndexItem {
                id: 8,
                kind: LibraryIndexKind::Ocr,
            },
            super::super::LibraryIndexItem {
                id: 2,
                kind: LibraryIndexKind::Translation,
            },
            super::super::LibraryIndexItem {
                id: 99,
                kind: LibraryIndexKind::Ocr,
            },
        ];
        let records = vec![
            LibraryRecord::TranslationHistory(translation(2, "second")),
            LibraryRecord::OcrHistory(ocr(8, "first")),
        ];
        let ordered = order_records(&index, records);
        assert_eq!(
            ordered.iter().map(LibraryRecord::key).collect::<Vec<_>>(),
            ["ocr:8", "translation:2"]
        );
    }

    #[test]
    fn history_record_exposes_only_visible_favorite_membership_identities() {
        let mut entry = translation(3, "hello");
        entry.results.push(
            crate::providers::translation::model::TranslationResult::success(
                "system", "你好", None, None,
            ),
        );
        let record = LibraryRecord::TranslationHistory(entry);
        assert_eq!(
            record.favorite_keys(),
            vec![r#"["translation","hello","en","zh-CN","system","你好"]"#]
        );
        assert!(
            LibraryRecord::OcrHistory(ocr(4, "recognized")).favorite_keys()[0]
                .contains("recognized")
        );
    }
}

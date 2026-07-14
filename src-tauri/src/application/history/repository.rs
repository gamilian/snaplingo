use crate::domain::ocr::{OcrRequest, OcrResult};
use crate::domain::translation::{TranslationRequest, TranslationResult};
use crate::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationHistoryEntry {
    pub id: i64,
    pub timestamp: DateTime<Utc>,
    pub note: Option<String>,
    pub tags: Vec<String>,
    pub source_text: String,
    pub source_lang: String,
    pub target_lang: String,
    pub providers_used: Vec<String>,
    pub results: Vec<TranslationResult>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrHistoryEntry {
    pub id: i64,
    pub timestamp: DateTime<Utc>,
    pub note: Option<String>,
    pub tags: Vec<String>,
    pub image_hash: String,
    pub language: Option<String>,
    pub provider_used: String,
    pub recognized_text: String,
    pub confidence: Option<f64>,
    pub duration_ms: u64,
    pub source_asset_path: Option<String>,
    pub thumbnail_asset_path: Option<String>,
    pub thumbnail_data_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredOcrHistoryAssets {
    pub source_path: String,
    pub thumbnail_path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HistoryCleanupPolicy {
    pub enabled: bool,
    pub retention_days: u32,
    pub maximum_records: u32,
}

pub trait HistoryPolicyProvider: Send + Sync {
    fn current_policy(&self) -> Result<HistoryCleanupPolicy>;
}

pub trait OcrHistoryAssetStore: Send + Sync {
    fn store(&self, image_data: &[u8]) -> Result<StoredOcrHistoryAssets>;
    fn read(&self, relative_path: &str) -> Result<Vec<u8>>;
    fn delete(&self, relative_path: &str) -> Result<()>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum HistoryEntry {
    Translation(TranslationHistoryEntry),
    Ocr(OcrHistoryEntry),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HistoryQuery {
    pub search: Option<String>,
    pub tag: Option<String>,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryPage<T> {
    pub items: Vec<T>,
    pub total: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HistoryKind {
    Translation,
    Ocr,
}

impl HistoryKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Translation => "translation",
            Self::Ocr => "ocr",
        }
    }
}

#[async_trait]
pub trait HistoryRepository: Send + Sync {
    async fn insert_translation(
        &self,
        request: &TranslationRequest,
        results: &[TranslationResult],
        providers_used: &[String],
        timestamp: DateTime<Utc>,
        duration_ms: u64,
    ) -> Result<()>;

    async fn insert_ocr(
        &self,
        request: &OcrRequest,
        result: &OcrResult,
        provider_used: &str,
        timestamp: DateTime<Utc>,
        duration_ms: u64,
        assets: Option<&StoredOcrHistoryAssets>,
    ) -> Result<()>;

    async fn query_translations(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<TranslationHistoryEntry>>;

    async fn query_ocr(&self, limit: usize, offset: usize) -> Result<Vec<OcrHistoryEntry>>;

    async fn query_translation_page(
        &self,
        query: &HistoryQuery,
    ) -> Result<HistoryPage<TranslationHistoryEntry>>;

    async fn query_ocr_page(&self, query: &HistoryQuery) -> Result<HistoryPage<OcrHistoryEntry>>;

    async fn search(&self, query: &str) -> Result<Vec<HistoryEntry>>;

    async fn delete(&self, id: i64) -> Result<()>;

    async fn update_note(&self, id: i64, note: Option<String>) -> Result<()>;

    async fn replace_tags(&self, id: i64, tags: Vec<String>) -> Result<()>;

    async fn clear_all(&self) -> Result<()>;

    async fn clear_kind(&self, kind: HistoryKind) -> Result<()>;

    async fn ocr_asset_paths(&self, id: Option<i64>) -> Result<Vec<(String, String)>>;

    async fn cleanup(&self, policy: HistoryCleanupPolicy)
        -> Result<(usize, Vec<(String, String)>)>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct FakeHistoryRepository {
        inserted_translation_count: Mutex<usize>,
    }

    #[async_trait]
    impl HistoryRepository for FakeHistoryRepository {
        async fn insert_translation(
            &self,
            _request: &TranslationRequest,
            _results: &[TranslationResult],
            _providers_used: &[String],
            _timestamp: DateTime<Utc>,
            _duration_ms: u64,
        ) -> Result<()> {
            *self.inserted_translation_count.lock().unwrap() += 1;
            Ok(())
        }

        async fn insert_ocr(
            &self,
            _request: &OcrRequest,
            _result: &OcrResult,
            _provider_used: &str,
            _timestamp: DateTime<Utc>,
            _duration_ms: u64,
            _assets: Option<&StoredOcrHistoryAssets>,
        ) -> Result<()> {
            Ok(())
        }

        async fn query_translations(
            &self,
            _limit: usize,
            _offset: usize,
        ) -> Result<Vec<TranslationHistoryEntry>> {
            Ok(Vec::new())
        }

        async fn query_ocr(&self, _limit: usize, _offset: usize) -> Result<Vec<OcrHistoryEntry>> {
            Ok(Vec::new())
        }

        async fn query_translation_page(
            &self,
            _query: &HistoryQuery,
        ) -> Result<HistoryPage<TranslationHistoryEntry>> {
            Ok(HistoryPage {
                items: Vec::new(),
                total: 0,
            })
        }

        async fn query_ocr_page(
            &self,
            _query: &HistoryQuery,
        ) -> Result<HistoryPage<OcrHistoryEntry>> {
            Ok(HistoryPage {
                items: Vec::new(),
                total: 0,
            })
        }

        async fn search(&self, _query: &str) -> Result<Vec<HistoryEntry>> {
            Ok(Vec::new())
        }

        async fn delete(&self, _id: i64) -> Result<()> {
            Ok(())
        }

        async fn update_note(&self, _id: i64, _note: Option<String>) -> Result<()> {
            Ok(())
        }

        async fn replace_tags(&self, _id: i64, _tags: Vec<String>) -> Result<()> {
            Ok(())
        }

        async fn clear_all(&self) -> Result<()> {
            Ok(())
        }

        async fn clear_kind(&self, _kind: HistoryKind) -> Result<()> {
            Ok(())
        }

        async fn ocr_asset_paths(&self, _id: Option<i64>) -> Result<Vec<(String, String)>> {
            Ok(Vec::new())
        }

        async fn cleanup(
            &self,
            _policy: HistoryCleanupPolicy,
        ) -> Result<(usize, Vec<(String, String)>)> {
            Ok((0, Vec::new()))
        }
    }

    #[tokio::test]
    async fn fake_history_repository_can_record_translation_port_call() {
        let repository = FakeHistoryRepository {
            inserted_translation_count: Mutex::new(0),
        };
        let request = TranslationRequest {
            text: "hello".to_string(),
            source_lang: "en".to_string(),
            target_lang: "fr".to_string(),
        };

        repository
            .insert_translation(&request, &[], &[], Utc::now(), 1)
            .await
            .unwrap();

        assert_eq!(*repository.inserted_translation_count.lock().unwrap(), 1);
    }
}

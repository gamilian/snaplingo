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
    pub favorite: bool,
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
    pub favorite: bool,
    pub note: Option<String>,
    pub tags: Vec<String>,
    pub image_hash: String,
    pub language: Option<String>,
    pub provider_used: String,
    pub recognized_text: String,
    pub confidence: Option<f64>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum HistoryEntry {
    Translation(TranslationHistoryEntry),
    Ocr(OcrHistoryEntry),
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
    ) -> Result<()>;

    async fn query_translations(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<TranslationHistoryEntry>>;

    async fn query_ocr(&self, limit: usize, offset: usize) -> Result<Vec<OcrHistoryEntry>>;

    async fn search(&self, query: &str) -> Result<Vec<HistoryEntry>>;

    async fn delete(&self, id: i64) -> Result<()>;

    async fn set_favorite(&self, id: i64, favorite: bool) -> Result<()>;

    async fn update_note(&self, id: i64, note: Option<String>) -> Result<()>;

    async fn replace_tags(&self, id: i64, tags: Vec<String>) -> Result<()>;

    async fn clear_all(&self) -> Result<()>;
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

        async fn search(&self, _query: &str) -> Result<Vec<HistoryEntry>> {
            Ok(Vec::new())
        }

        async fn delete(&self, _id: i64) -> Result<()> {
            Ok(())
        }

        async fn set_favorite(&self, _id: i64, _favorite: bool) -> Result<()> {
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

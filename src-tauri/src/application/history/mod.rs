use crate::domain::events::DomainEvent;
use crate::infrastructure::events::EventSubscriber;
use crate::infrastructure::storage::{
    HistoryDatabase, HistoryEntry, OcrHistoryEntry, TranslationHistoryEntry,
};
use crate::Result;
use async_trait::async_trait;
use std::sync::Arc;

#[cfg(test)]
mod tests;

/// Owns history recording, queries, and deletion.
///
/// History acts as an EventSubscriber, automatically recording
/// translation and OCR operations when they complete. It also provides
/// query and management APIs for history records.
pub struct History {
    db: Arc<HistoryDatabase>,
}

impl History {
    /// Create a new History module.
    pub fn new(db: Arc<HistoryDatabase>) -> Self {
        Self { db }
    }

    /// Get translation history with pagination
    pub async fn get_translation_history(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<TranslationHistoryEntry>> {
        self.db.query_translations(limit, offset).await
    }

    /// Get OCR history with pagination
    pub async fn get_ocr_history(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<OcrHistoryEntry>> {
        self.db.query_ocr(limit, offset).await
    }

    /// Search history by query string
    pub async fn search_history(&self, query: &str) -> Result<Vec<HistoryEntry>> {
        self.db.search(query).await
    }

    /// Delete a history entry by ID
    pub async fn delete_history(&self, id: i64) -> Result<()> {
        self.db.delete(id).await
    }

    /// Clear all history
    pub async fn clear_all_history(&self) -> Result<()> {
        self.db.clear_all().await
    }
}

#[async_trait]
impl EventSubscriber for History {
    async fn handle(&self, event: &DomainEvent) {
        match event {
            DomainEvent::TranslationCompleted {
                request,
                results,
                providers_used,
                timestamp,
                duration_ms,
            } => {
                if let Err(e) = self
                    .db
                    .insert_translation(request, results, providers_used, *timestamp, *duration_ms)
                    .await
                {
                    eprintln!("[History] Failed to record translation: {}", e);
                }
            }
            DomainEvent::OcrCompleted {
                request,
                result,
                provider_used,
                timestamp,
                duration_ms,
            } => {
                if let Err(e) = self
                    .db
                    .insert_ocr(request, result, provider_used, *timestamp, *duration_ms)
                    .await
                {
                    eprintln!("[History] Failed to record OCR: {}", e);
                }
            }
            DomainEvent::ProviderConfigurationFailed {
                provider_id,
                error_message,
                ..
            } => {
                // Log the error but don't persist to history database
                eprintln!(
                    "[History] Provider configuration failed: {} - {}",
                    provider_id, error_message
                );
            }
        }
    }

    fn name(&self) -> &str {
        "history"
    }
}

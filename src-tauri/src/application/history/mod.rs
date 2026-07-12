use crate::domain::events::DomainEvent;
use crate::Result;
use async_trait::async_trait;
use std::sync::Arc;

mod event_source;
mod repository;

#[cfg(test)]
mod tests;

pub use event_source::EventSubscriber;
pub use repository::{HistoryEntry, HistoryRepository, OcrHistoryEntry, TranslationHistoryEntry};

/// Owns history recording, queries, and deletion.
///
/// History acts as an EventSubscriber, automatically recording
/// translation and OCR operations when they complete. It also provides
/// query and management APIs for history records.
pub struct History {
    repository: Arc<dyn HistoryRepository>,
}

impl History {
    /// Create a new History module.
    pub fn new(repository: Arc<dyn HistoryRepository>) -> Self {
        Self { repository }
    }

    /// Get translation history with pagination
    pub async fn get_translation_history(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<TranslationHistoryEntry>> {
        self.repository.query_translations(limit, offset).await
    }

    /// Get OCR history with pagination
    pub async fn get_ocr_history(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<OcrHistoryEntry>> {
        self.repository.query_ocr(limit, offset).await
    }

    /// Search history by query string
    pub async fn search_history(&self, query: &str) -> Result<Vec<HistoryEntry>> {
        self.repository.search(query).await
    }

    /// Delete a history entry by ID
    pub async fn delete_history(&self, id: i64) -> Result<()> {
        self.repository.delete(id).await
    }

    /// Clear all history
    pub async fn clear_all_history(&self) -> Result<()> {
        self.repository.clear_all().await
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
                    .repository
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
                    .repository
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

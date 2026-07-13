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
    change_notifier: Option<Arc<dyn HistoryChangeNotifier>>,
}

pub trait HistoryChangeNotifier: Send + Sync {
    fn history_changed(&self);
}

impl History {
    /// Create a new History module.
    pub fn new(repository: Arc<dyn HistoryRepository>) -> Self {
        Self {
            repository,
            change_notifier: None,
        }
    }

    pub fn with_change_notifier(
        repository: Arc<dyn HistoryRepository>,
        change_notifier: Arc<dyn HistoryChangeNotifier>,
    ) -> Self {
        Self {
            repository,
            change_notifier: Some(change_notifier),
        }
    }

    fn notify_changed(&self) {
        if let Some(notifier) = &self.change_notifier {
            notifier.history_changed();
        }
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

    pub async fn set_history_favorite(&self, id: i64, favorite: bool) -> Result<()> {
        self.repository.set_favorite(id, favorite).await
    }

    pub async fn update_history_note(&self, id: i64, note: Option<String>) -> Result<()> {
        self.repository.update_note(id, note).await
    }

    pub async fn replace_history_tags(&self, id: i64, tags: Vec<String>) -> Result<()> {
        self.repository.replace_tags(id, tags).await
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
                match self
                    .repository
                    .insert_translation(request, results, providers_used, *timestamp, *duration_ms)
                    .await
                {
                    Ok(_) => self.notify_changed(),
                    Err(e) => eprintln!("[History] Failed to record translation: {}", e),
                }
            }
            DomainEvent::OcrCompleted {
                request,
                result,
                provider_used,
                timestamp,
                duration_ms,
            } => {
                match self
                    .repository
                    .insert_ocr(request, result, provider_used, *timestamp, *duration_ms)
                    .await
                {
                    Ok(_) => self.notify_changed(),
                    Err(e) => eprintln!("[History] Failed to record OCR: {}", e),
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

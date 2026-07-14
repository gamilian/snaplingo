use crate::domain::events::DomainEvent;
use crate::domain::ocr::OcrResult;
use crate::domain::translation::{TranslationRequest, TranslationResult};
use crate::Result;
use async_trait::async_trait;
use base64::Engine;
use chrono::{DateTime, Utc};
use std::sync::Arc;

mod event_source;
mod repository;

#[cfg(test)]
mod tests;

pub use event_source::EventSubscriber;
pub use repository::{
    HistoryCleanupPolicy, HistoryEntry, HistoryKind, HistoryPage, HistoryPolicyProvider,
    HistoryQuery, HistoryRepository, OcrHistoryAssetStore, OcrHistoryEntry, StoredOcrHistoryAssets,
    TranslationHistoryEntry,
};

/// Owns history recording, queries, and deletion.
///
/// History acts as an EventSubscriber, automatically recording
/// translation and OCR operations when they complete. It also provides
/// query and management APIs for history records.
pub struct History {
    repository: Arc<dyn HistoryRepository>,
    change_notifier: Option<Arc<dyn HistoryChangeNotifier>>,
    ocr_assets: Option<Arc<dyn OcrHistoryAssetStore>>,
    policy_provider: Option<Arc<dyn HistoryPolicyProvider>>,
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
            ocr_assets: None,
            policy_provider: None,
        }
    }

    pub fn with_change_notifier(
        repository: Arc<dyn HistoryRepository>,
        change_notifier: Arc<dyn HistoryChangeNotifier>,
    ) -> Self {
        Self {
            repository,
            change_notifier: Some(change_notifier),
            ocr_assets: None,
            policy_provider: None,
        }
    }

    pub fn with_dependencies(
        repository: Arc<dyn HistoryRepository>,
        change_notifier: Arc<dyn HistoryChangeNotifier>,
        ocr_assets: Arc<dyn OcrHistoryAssetStore>,
    ) -> Self {
        Self {
            repository,
            change_notifier: Some(change_notifier),
            ocr_assets: Some(ocr_assets),
            policy_provider: None,
        }
    }

    pub fn with_dependencies_and_policy(
        repository: Arc<dyn HistoryRepository>,
        change_notifier: Arc<dyn HistoryChangeNotifier>,
        ocr_assets: Arc<dyn OcrHistoryAssetStore>,
        policy_provider: Arc<dyn HistoryPolicyProvider>,
    ) -> Self {
        Self {
            repository,
            change_notifier: Some(change_notifier),
            ocr_assets: Some(ocr_assets),
            policy_provider: Some(policy_provider),
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

    pub async fn query_translation_history(
        &self,
        query: HistoryQuery,
    ) -> Result<HistoryPage<TranslationHistoryEntry>> {
        self.repository.query_translation_page(&query).await
    }

    pub async fn record_translation(
        &self,
        request: TranslationRequest,
        results: Vec<TranslationResult>,
        duration_ms: u64,
    ) -> Result<()> {
        let providers_used = results
            .iter()
            .map(|result| result.provider_id.clone())
            .collect::<Vec<_>>();
        self.store_translation(&request, &results, &providers_used, Utc::now(), duration_ms)
            .await
    }

    /// Get OCR history with pagination
    pub async fn get_ocr_history(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<OcrHistoryEntry>> {
        let entries = self.repository.query_ocr(limit, offset).await?;
        self.hydrate_ocr_thumbnails(entries)
    }

    pub async fn query_ocr_history(
        &self,
        query: HistoryQuery,
    ) -> Result<HistoryPage<OcrHistoryEntry>> {
        let mut page = self.repository.query_ocr_page(&query).await?;
        page.items = self.hydrate_ocr_thumbnails(page.items)?;
        Ok(page)
    }

    /// Search history by query string
    pub async fn search_history(&self, query: &str) -> Result<Vec<HistoryEntry>> {
        self.repository.search(query).await
    }

    /// Delete a history entry by ID
    pub async fn delete_history(&self, id: i64) -> Result<()> {
        let assets = self.repository.ocr_asset_paths(Some(id)).await?;
        self.repository.delete(id).await?;
        self.delete_ocr_assets(assets);
        self.notify_changed();
        Ok(())
    }

    pub async fn update_history_note(&self, id: i64, note: Option<String>) -> Result<()> {
        self.repository.update_note(id, note).await?;
        self.notify_changed();
        Ok(())
    }

    pub async fn replace_history_tags(&self, id: i64, tags: Vec<String>) -> Result<()> {
        self.repository.replace_tags(id, tags).await?;
        self.notify_changed();
        Ok(())
    }

    /// Clear all history
    pub async fn clear_all_history(&self) -> Result<()> {
        let assets = self.repository.ocr_asset_paths(None).await?;
        self.repository.clear_all().await?;
        self.delete_ocr_assets(assets);
        self.notify_changed();
        Ok(())
    }

    pub async fn clear_history(&self, kind: HistoryKind) -> Result<()> {
        let assets = if kind == HistoryKind::Ocr {
            self.repository.ocr_asset_paths(None).await?
        } else {
            Vec::new()
        };
        self.repository.clear_kind(kind).await?;
        self.delete_ocr_assets(assets);
        self.notify_changed();
        Ok(())
    }

    pub async fn read_ocr_source(&self, id: i64) -> Result<Vec<u8>> {
        let (source, _) = self
            .repository
            .ocr_asset_paths(Some(id))
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| format!("OCR history {} has no preserved source image", id))?;
        self.ocr_assets
            .as_ref()
            .ok_or_else(|| "OCR history asset storage is unavailable".to_string())?
            .read(&source)
    }

    pub async fn run_cleanup(&self) -> Result<usize> {
        let Some(provider) = &self.policy_provider else {
            return Ok(0);
        };
        let mut policy = provider.current_policy()?;
        if !policy.enabled {
            policy.retention_days = 0;
        }
        let (removed, assets) = self.repository.cleanup(policy).await?;
        self.delete_ocr_assets(assets);
        if removed > 0 {
            self.notify_changed();
        }
        Ok(removed)
    }

    fn hydrate_ocr_thumbnails(
        &self,
        mut entries: Vec<OcrHistoryEntry>,
    ) -> Result<Vec<OcrHistoryEntry>> {
        let Some(store) = &self.ocr_assets else {
            return Ok(entries);
        };
        for entry in &mut entries {
            if let Some(path) = entry.thumbnail_asset_path.as_deref() {
                let thumbnail = store.read(path)?;
                entry.thumbnail_data_url = Some(format!(
                    "data:image/png;base64,{}",
                    base64::engine::general_purpose::STANDARD.encode(thumbnail)
                ));
            }
        }
        Ok(entries)
    }

    fn delete_ocr_assets(&self, assets: Vec<(String, String)>) {
        let Some(store) = &self.ocr_assets else {
            return;
        };
        for (source, thumbnail) in assets {
            let _ = store.delete(&source);
            let _ = store.delete(&thumbnail);
        }
    }

    async fn store_translation(
        &self,
        request: &TranslationRequest,
        results: &[TranslationResult],
        providers_used: &[String],
        timestamp: DateTime<Utc>,
        duration_ms: u64,
    ) -> Result<()> {
        self.repository
            .insert_translation(request, results, providers_used, timestamp, duration_ms)
            .await?;
        self.notify_changed();
        if let Err(error) = self.run_cleanup().await {
            eprintln!("[History] Failed to run automatic cleanup: {}", error);
        }
        Ok(())
    }
}

pub struct OcrHistoryReplay {
    history: Arc<History>,
    recognizer: Arc<dyn OcrHistoryRecognizer>,
}

#[async_trait]
pub trait OcrHistoryRecognizer: Send + Sync {
    async fn recognize(&self, image: Vec<u8>) -> Result<OcrResult>;
}

impl OcrHistoryReplay {
    pub fn new(history: Arc<History>, recognizer: Arc<dyn OcrHistoryRecognizer>) -> Self {
        Self {
            history,
            recognizer,
        }
    }

    pub async fn run(&self, id: i64) -> Result<OcrResult> {
        let image = self.history.read_ocr_source(id).await?;
        self.recognizer.recognize(image).await
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
                    .store_translation(request, results, providers_used, *timestamp, *duration_ms)
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
                let assets = match self
                    .ocr_assets
                    .as_ref()
                    .map(|store| store.store(&request.image_data))
                    .transpose()
                {
                    Ok(assets) => assets,
                    Err(error) => {
                        eprintln!("[History] Failed to store OCR source image: {}", error);
                        return;
                    }
                };
                match self
                    .repository
                    .insert_ocr(
                        request,
                        result,
                        provider_used,
                        *timestamp,
                        *duration_ms,
                        assets.as_ref(),
                    )
                    .await
                {
                    Ok(_) => {
                        self.notify_changed();
                        if let Err(error) = self.run_cleanup().await {
                            eprintln!("[History] Failed to run automatic cleanup: {}", error);
                        }
                    }
                    Err(e) => {
                        if let (Some(store), Some(assets)) = (&self.ocr_assets, assets) {
                            let _ = store.delete(&assets.source_path);
                            let _ = store.delete(&assets.thumbnail_path);
                        }
                        eprintln!("[History] Failed to record OCR: {}", e)
                    }
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

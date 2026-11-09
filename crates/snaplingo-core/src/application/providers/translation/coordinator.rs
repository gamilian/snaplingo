use super::TranslationProvider;
use crate::application::providers::{ProviderConfigStore, ProviderEventSink};
use crate::domain::events::DomainEvent;
use crate::domain::translation::{TranslationRequest, TranslationResult};
use crate::Result;
use chrono::Utc;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

static TRANSLATION_REQUEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Coordinator for managing translation providers and operations.
///
/// This coordinator consolidates provider registration, activation management,
/// and translation execution with concurrent provider support. It replaces
/// the previous Registry + Service split with a single cohesive component.
///
/// Concurrency model: Uses fine-grained internal locking to allow safe &self
/// access while maintaining thread-safety. Providers use RwLock for dynamic
/// registration/removal.
pub struct TranslationCoordinator {
    /// Map of provider ID to provider instance (wrapped for dynamic registration and reconfiguration)
    providers: Arc<RwLock<HashMap<String, Arc<RwLock<dyn TranslationProvider>>>>>,
    /// List of active provider IDs (in order of activation)
    /// Wrapped in Arc<Mutex<>> for interior mutability
    active: Arc<Mutex<Vec<String>>>,
    /// Configuration store for persisting active providers
    config_store: Arc<dyn ProviderConfigStore>,
    /// Optional event sink for publishing domain events
    event_sink: Option<Arc<dyn ProviderEventSink>>,
}

impl TranslationCoordinator {
    /// Creates a new TranslationCoordinator with the given config store.
    pub fn new(config_store: Arc<dyn ProviderConfigStore>) -> Self {
        Self {
            providers: Arc::new(RwLock::new(HashMap::new())),
            active: Arc::new(Mutex::new(Vec::new())),
            config_store,
            event_sink: None,
        }
    }

    /// Attach an event sink for publishing domain events
    pub fn with_event_sink(mut self, event_sink: Arc<dyn ProviderEventSink>) -> Self {
        self.event_sink = Some(event_sink);
        self
    }

    /// Attach an event sink for publishing domain events.
    pub fn with_event_bus(self, event_sink: Arc<dyn ProviderEventSink>) -> Self {
        self.with_event_sink(event_sink)
    }

    /// Registers a new translation provider.
    ///
    /// # Arguments
    ///
    /// * `provider` - The provider to register (will be wrapped in Arc<RwLock<>>)
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Ok if successful, Err if a provider with the same ID already exists
    pub fn register<T: TranslationProvider + 'static>(&self, provider: T) -> Result<()> {
        let id = provider.id().to_string();
        let mut providers = self.providers.write();
        if providers.contains_key(&id) {
            return Err(format!("Provider already registered: {}", id).into());
        }
        providers.insert(id, Arc::new(RwLock::new(provider)));
        Ok(())
    }

    /// Replaces a registered provider instance without changing active provider order.
    pub fn replace<T: TranslationProvider + 'static>(&self, provider: T) -> Result<()> {
        let id = provider.id().to_string();
        let mut providers = self.providers.write();
        if !providers.contains_key(&id) {
            return Err(format!("Provider not found: {}", id).into());
        }
        providers.insert(id, Arc::new(RwLock::new(provider)));
        Ok(())
    }

    /// Unregisters a translation provider by ID.
    ///
    /// If the provider is active, it will be deactivated first.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the provider to unregister
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Ok if successful, Err if the provider doesn't exist
    pub fn unregister(&self, id: &str) -> Result<()> {
        // Check if provider exists first
        let providers = self.providers.read();
        if !providers.contains_key(id) {
            return Err(format!("Provider not found: {}", id).into());
        }
        drop(providers);

        // Deactivate if active: compute new active list and persist before modifying memory
        let mut active = self.active.lock().unwrap();
        let was_active = active.iter().any(|active_id| active_id == id);
        if was_active {
            let new_active: Vec<String> = active
                .iter()
                .filter(|active_id| active_id.as_str() != id)
                .cloned()
                .collect();

            // Persist first
            self.config_store
                .save_active_translation_providers(&new_active)?;

            // Only modify memory after successful persistence
            *active = new_active;
        }
        drop(active);

        // Remove from providers
        let mut providers = self.providers.write();
        providers.remove(id);
        Ok(())
    }

    /// Activates a provider by ID.
    ///
    /// If the provider is already active, this is a no-op.
    /// The updated active list is automatically persisted to the config file.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the provider to activate
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Ok if successful, Err if the provider doesn't exist or persistence fails
    pub fn activate(&self, id: &str) -> Result<()> {
        let providers = self.providers.read();
        if !providers.contains_key(id) {
            return Err(format!("Provider not found: {}", id).into());
        }
        drop(providers);

        let mut active = self.active.lock().unwrap();

        // Compute new active list
        let new_active = if active.contains(&id.to_string()) {
            // Already active, no change
            return Ok(());
        } else {
            let mut new_list = active.clone();
            new_list.push(id.to_string());
            new_list
        };

        // Persist first
        self.config_store
            .save_active_translation_providers(&new_active)?;

        // Only update memory after successful persistence
        *active = new_active;
        Ok(())
    }

    /// Deactivates a provider by ID.
    ///
    /// If the provider is not active, this is a no-op.
    /// The updated active list is automatically persisted to the config file.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the provider to deactivate
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Ok if successful, Err if the provider doesn't exist or persistence fails
    pub fn deactivate(&self, id: &str) -> Result<()> {
        let providers = self.providers.read();
        if !providers.contains_key(id) {
            return Err(format!("Provider not found: {}", id).into());
        }
        drop(providers);

        let mut active = self.active.lock().unwrap();

        // Compute new active list
        let new_active: Vec<String> = active
            .iter()
            .filter(|active_id| active_id.as_str() != id)
            .cloned()
            .collect();

        // No-op if already not active
        if new_active.len() == active.len() {
            return Ok(());
        }

        // Persist first
        self.config_store
            .save_active_translation_providers(&new_active)?;

        // Only update memory after successful persistence
        *active = new_active;
        Ok(())
    }

    /// Reorders the active providers.
    ///
    /// The provided IDs must match the current set of active providers exactly.
    /// This method only changes the order, not which providers are active.
    ///
    /// # Arguments
    ///
    /// * `ordered_ids` - The new order of active provider IDs
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Ok if successful, Err if validation fails or persistence fails
    pub fn reorder_active(&self, ordered_ids: Vec<String>) -> Result<()> {
        let mut active = self.active.lock().unwrap();

        // Validate: ordered_ids must contain exactly the same IDs as current active
        let mut current_sorted = active.clone();
        current_sorted.sort();

        let mut new_sorted = ordered_ids.clone();
        new_sorted.sort();

        if current_sorted != new_sorted {
            return Err(format!(
                "Reorder validation failed: expected {:?}, got {:?}",
                current_sorted, new_sorted
            )
            .into());
        }

        // Persist first
        self.config_store
            .save_active_translation_providers(&ordered_ids)?;

        // Only update memory after successful persistence
        *active = ordered_ids;

        Ok(())
    }

    /// Returns a list of all active providers.
    ///
    /// # Returns
    ///
    /// * `Vec<Arc<RwLock<dyn TranslationProvider>>>` - List of active providers in activation order
    pub fn get_active(&self) -> Vec<Arc<RwLock<dyn TranslationProvider>>> {
        let active = self.active.lock().unwrap();
        let providers = self.providers.read();
        active
            .iter()
            .filter_map(|id| providers.get(id).cloned())
            .collect()
    }

    /// Returns a list of all registered providers.
    ///
    /// # Returns
    ///
    /// * `Vec<Arc<RwLock<dyn TranslationProvider>>>` - List of all registered providers
    pub fn list_all(&self) -> Vec<Arc<RwLock<dyn TranslationProvider>>> {
        let providers = self.providers.read();
        providers.values().cloned().collect()
    }

    /// Gets a specific provider by ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the provider to retrieve
    ///
    /// # Returns
    ///
    /// * `Option<Arc<RwLock<dyn TranslationProvider>>>` - The provider if found, None otherwise
    pub fn get(&self, id: &str) -> Option<Arc<RwLock<dyn TranslationProvider>>> {
        let providers = self.providers.read();
        providers.get(id).cloned()
    }

    /// Restores active providers from the config file.
    ///
    /// Skips any provider IDs that are not registered.
    pub fn restore_from_config(&self) -> Result<()> {
        if let Ok(active_ids) = self.config_store.load_active_translation_providers() {
            let mut active = self.active.lock().unwrap();
            let providers = self.providers.read();
            active.clear();
            for id in active_ids {
                let id = normalize_legacy_translation_provider_id(&id);
                if providers.contains_key(id) {
                    active.push(id.to_string());
                }
            }
        }
        Ok(())
    }

    /// Translates text using all active providers concurrently.
    ///
    /// This method:
    /// 1. Gets all active providers
    /// 2. Spawns concurrent translation tasks for each provider
    /// 3. Collects and returns all successful results
    ///
    /// # Arguments
    ///
    /// * `request` - The translation request
    ///
    /// # Returns
    ///
    /// * `Result<Vec<TranslationResult>>` - Results from all active providers
    ///
    /// # Errors
    ///
    /// Returns an error if no providers are active.
    /// Individual provider failures are logged but don't fail the entire request.
    pub async fn translate(&self, request: &TranslationRequest) -> Result<Vec<TranslationResult>> {
        let start = Instant::now();
        let request_id = next_translation_request_id();

        // Get active providers (lock is released immediately after cloning)
        let active_providers = self.get_active();

        // Check if any providers are active
        if active_providers.is_empty() {
            return Err("No active translation providers".into());
        }

        // Spawn concurrent translation tasks
        let mut tasks = Vec::new();
        for provider_lock in &active_providers {
            let provider_id = provider_lock.read().id().to_string();
            let request = request.clone();
            let provider_lock = provider_lock.clone();

            let task = tokio::spawn(async move {
                let started_at = Instant::now();
                let provider = provider_lock.read();
                let result = provider.translate(&request).await;
                (result, started_at.elapsed().as_millis() as u64)
            });

            tasks.push((provider_id, task));
        }

        // Collect results
        let mut results = Vec::new();
        for (provider_id, task) in tasks {
            match task.await {
                Ok((Ok(mut result), duration_ms)) => {
                    result.request_id = Some(request_id.clone());
                    result.duration_ms = Some(duration_ms);
                    results.push(result);
                }
                Ok((Err(e), duration_ms)) => {
                    eprintln!("Translation provider error: {}", e);
                    let mut result = translation_failure(
                        provider_id,
                        "provider_error",
                        e.to_string(),
                        duration_ms,
                    );
                    result.request_id = Some(request_id.clone());
                    results.push(result);
                }
                Err(e) => {
                    eprintln!("Translation task error: {}", e);
                    let mut result = translation_failure(
                        provider_id,
                        "provider_task_failed",
                        e.to_string(),
                        start.elapsed().as_millis() as u64,
                    );
                    result.request_id = Some(request_id.clone());
                    results.push(result);
                }
            }
        }

        // Publish domain event if event sink is attached
        if let Some(event_sink) = &self.event_sink {
            let providers_used: Vec<String> = active_providers
                .iter()
                .map(|p| p.read().id().to_string())
                .collect();

            event_sink.publish(DomainEvent::TranslationCompleted {
                request: request.clone(),
                results: results.clone(),
                providers_used,
                timestamp: Utc::now(),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }

        Ok(results)
    }

    /// Translates text using one registered provider.
    pub async fn translate_with_provider(
        &self,
        provider_id: &str,
        request: &TranslationRequest,
    ) -> Result<TranslationResult> {
        let request_id = next_translation_request_id();
        let started_at = Instant::now();
        let Some(provider_lock) = self.get(provider_id) else {
            let mut result = translation_failure(
                provider_id.to_string(),
                "provider_not_found",
                format!("Provider not found: {}", provider_id),
                started_at.elapsed().as_millis() as u64,
            );
            result.request_id = Some(request_id);
            return Ok(result);
        };
        let provider = provider_lock.read();
        match provider.translate(request).await {
            Ok(mut result) => {
                result.request_id = Some(request_id);
                result.duration_ms = Some(started_at.elapsed().as_millis() as u64);
                Ok(result)
            }
            Err(error) => {
                let mut result = translation_failure(
                    provider_id.to_string(),
                    "provider_error",
                    error.to_string(),
                    started_at.elapsed().as_millis() as u64,
                );
                result.request_id = Some(request_id);
                Ok(result)
            }
        }
    }

    /// Reconfigures a provider's credentials at runtime.
    ///
    /// This allows hot-reloading of credentials without restarting the application.
    ///
    /// # Arguments
    ///
    /// * `provider_id` - The ID of the provider to reconfigure
    /// * `credentials` - HashMap mapping credential field names to their values
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Ok if successful, Err if provider not found or reconfiguration fails
    pub fn reconfigure_provider(
        &self,
        provider_id: &str,
        credentials: &HashMap<String, String>,
    ) -> Result<()> {
        let providers = self.providers.read();
        let provider_lock = providers
            .get(provider_id)
            .ok_or_else(|| crate::AppError::Other(format!("Provider not found: {}", provider_id)))?
            .clone();
        drop(providers);

        let result = {
            let mut provider = provider_lock.write();
            provider.reconfigure_credentials(credentials)
        };

        // Publish event on failure
        if let Err(ref e) = result {
            if let Some(event_sink) = &self.event_sink {
                event_sink.publish(DomainEvent::ProviderConfigurationFailed {
                    provider_id: provider_id.to_string(),
                    error_message: e.to_string(),
                    timestamp: Utc::now(),
                });
            }
        }

        result
    }
}

fn normalize_legacy_translation_provider_id(id: &str) -> &str {
    match id {
        "deepl" => "deeplx",
        _ => id,
    }
}

fn next_translation_request_id() -> String {
    let sequence = TRANSLATION_REQUEST_SEQUENCE.fetch_add(1, Ordering::Relaxed) + 1;
    format!("translation-request-{}", sequence)
}

fn translation_failure(
    provider_id: impl Into<String>,
    code: &str,
    message: String,
    duration_ms: u64,
) -> TranslationResult {
    let retryable = is_retryable_translation_error(&message);
    let mut result = TranslationResult::failure(provider_id, code, message, retryable);
    result.duration_ms = Some(duration_ms);
    result
}

fn is_retryable_translation_error(message: &str) -> bool {
    let message = message.to_ascii_lowercase();
    [
        "timeout",
        "timed out",
        "network",
        "connect",
        "connection",
        "temporarily",
        "rate limit",
        "429",
        "502",
        "503",
        "504",
    ]
    .iter()
    .any(|marker| message.contains(marker))
}

use super::TranslationProvider;
use crate::domain::translation::{TranslationRequest, TranslationResult};
use crate::domain::events::DomainEvent;
use crate::infrastructure::storage::ConfigFile;
use crate::infrastructure::events::EventBus;
use crate::Result;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use chrono::Utc;

/// Coordinator for managing translation providers and operations.
///
/// This coordinator consolidates provider registration, activation management,
/// and translation execution with concurrent provider support. It replaces
/// the previous Registry + Service split with a single cohesive component.
///
/// Concurrency model: Uses fine-grained internal locking to allow safe &self
/// access while maintaining thread-safety.
pub struct TranslationCoordinator {
    /// Map of provider ID to provider instance
    providers: HashMap<String, Arc<dyn TranslationProvider>>,
    /// List of active provider IDs (in order of activation)
    /// Wrapped in Arc<Mutex<>> for interior mutability
    active: Arc<Mutex<Vec<String>>>,
    /// Configuration file for persisting active providers
    config: Arc<ConfigFile>,
    /// Optional event bus for publishing domain events
    event_bus: Option<Arc<EventBus>>,
}

impl TranslationCoordinator {
    /// Creates a new TranslationCoordinator with the given config.
    pub fn new(config: Arc<ConfigFile>) -> Self {
        Self {
            providers: HashMap::new(),
            active: Arc::new(Mutex::new(Vec::new())),
            config,
            event_bus: None,
        }
    }

    /// Attach an event bus for publishing domain events
    pub fn with_event_bus(mut self, event_bus: Arc<EventBus>) -> Self {
        self.event_bus = Some(event_bus);
        self
    }

    /// Registers a new translation provider.
    ///
    /// # Arguments
    ///
    /// * `provider` - The provider to register
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Ok if successful, Err if a provider with the same ID already exists
    pub fn register(&mut self, provider: Arc<dyn TranslationProvider>) -> Result<()> {
        let id = provider.id().to_string();
        if self.providers.contains_key(&id) {
            return Err(format!("Provider already registered: {}", id).into());
        }
        self.providers.insert(id, provider);
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
        if !self.providers.contains_key(id) {
            return Err(format!("Provider not found: {}", id).into());
        }

        let mut active = self.active.lock().unwrap();
        if !active.contains(&id.to_string()) {
            active.push(id.to_string());
        }

        // Persist to config
        self.config.save("active_translation_providers", &*active)?;
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
        if !self.providers.contains_key(id) {
            return Err(format!("Provider not found: {}", id).into());
        }

        let mut active = self.active.lock().unwrap();
        active.retain(|active_id| active_id != id);

        // Persist to config
        self.config.save("active_translation_providers", &*active)?;
        Ok(())
    }

    /// Returns a list of all active providers.
    ///
    /// # Returns
    ///
    /// * `Vec<Arc<dyn TranslationProvider>>` - List of active providers in activation order
    pub fn get_active(&self) -> Vec<Arc<dyn TranslationProvider>> {
        let active = self.active.lock().unwrap();
        active
            .iter()
            .filter_map(|id| self.providers.get(id).cloned())
            .collect()
    }

    /// Returns a list of all registered providers.
    ///
    /// # Returns
    ///
    /// * `Vec<Arc<dyn TranslationProvider>>` - List of all registered providers
    pub fn list_all(&self) -> Vec<Arc<dyn TranslationProvider>> {
        self.providers.values().cloned().collect()
    }

    /// Gets a specific provider by ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the provider to retrieve
    ///
    /// # Returns
    ///
    /// * `Option<Arc<dyn TranslationProvider>>` - The provider if found, None otherwise
    pub fn get(&self, id: &str) -> Option<Arc<dyn TranslationProvider>> {
        self.providers.get(id).cloned()
    }

    /// Restores active providers from the config file.
    ///
    /// Skips any provider IDs that are not registered.
    pub fn restore_from_config(&mut self) -> Result<()> {
        if let Ok(active_ids) = self.config.load::<Vec<String>>("active_translation_providers") {
            let mut active = self.active.lock().unwrap();
            active.clear();
            for id in active_ids {
                if self.providers.contains_key(&id) {
                    active.push(id);
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

        // Get active providers (lock is released immediately after cloning)
        let active_providers = self.get_active();

        // Check if any providers are active
        if active_providers.is_empty() {
            return Err("No active translation providers".into());
        }

        // Spawn concurrent translation tasks
        let mut tasks = Vec::new();
        for provider in &active_providers {
            let request = request.clone();
            let provider = provider.clone();

            let task = tokio::spawn(async move {
                provider.translate(&request).await
            });

            tasks.push(task);
        }

        // Collect results
        let mut results = Vec::new();
        for task in tasks {
            match task.await {
                Ok(Ok(result)) => {
                    results.push(result);
                }
                Ok(Err(e)) => {
                    // Log provider error but continue with other providers
                    eprintln!("Translation provider error: {}", e);
                }
                Err(e) => {
                    // Log task join error but continue
                    eprintln!("Translation task error: {}", e);
                }
            }
        }

        // Publish domain event if event bus is attached
        if let Some(event_bus) = &self.event_bus {
            let providers_used: Vec<String> = active_providers
                .iter()
                .map(|p| p.id().to_string())
                .collect();

            event_bus.publish(DomainEvent::TranslationCompleted {
                request: request.clone(),
                results: results.clone(),
                providers_used,
                timestamp: Utc::now(),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }

        Ok(results)
    }
}

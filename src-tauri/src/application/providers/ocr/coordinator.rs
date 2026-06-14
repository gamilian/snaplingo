use super::OcrProvider;
use crate::domain::ocr::{OcrRequest, OcrResult};
use crate::domain::events::DomainEvent;
use crate::infrastructure::storage::ConfigFile;
use crate::infrastructure::events::EventBus;
use crate::Result;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use chrono::Utc;

/// Coordinator for managing OCR providers and operations.
///
/// This coordinator consolidates provider registration, activation management,
/// and OCR execution. It replaces the previous Registry + Service split with
/// a single cohesive component.
///
/// Unlike TranslationCoordinator, only ONE provider can be active at a time
/// (single-select pattern).
///
/// Concurrency model: Uses fine-grained internal locking to allow safe &self
/// access while maintaining thread-safety.
pub struct OcrCoordinator {
    /// Map of provider ID to provider instance
    providers: HashMap<String, Arc<dyn OcrProvider>>,
    /// Currently active provider ID (single-select)
    /// Wrapped in Arc<Mutex<>> for interior mutability
    active_provider_id: Arc<Mutex<Option<String>>>,
    /// Configuration file for persisting active provider
    config: Arc<ConfigFile>,
    /// Optional event bus for publishing domain events
    event_bus: Option<Arc<EventBus>>,
}

impl OcrCoordinator {
    /// Creates a new OcrCoordinator with the given config.
    pub fn new(config: Arc<ConfigFile>) -> Self {
        Self {
            providers: HashMap::new(),
            active_provider_id: Arc::new(Mutex::new(None)),
            config,
            event_bus: None,
        }
    }

    /// Attach an event bus for publishing domain events
    pub fn with_event_bus(mut self, event_bus: Arc<EventBus>) -> Self {
        self.event_bus = Some(event_bus);
        self
    }

    /// Registers a new OCR provider.
    ///
    /// # Arguments
    ///
    /// * `provider` - The provider to register
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Ok if successful, Err if a provider with the same ID already exists
    pub fn register(&mut self, provider: Arc<dyn OcrProvider>) -> Result<()> {
        let id = provider.id().to_string();
        if self.providers.contains_key(&id) {
            return Err(format!("Provider already registered: {}", id).into());
        }
        self.providers.insert(id, provider);
        Ok(())
    }

    /// Activates a provider by ID.
    ///
    /// This replaces any currently active provider (single-select pattern).
    /// The updated active provider is automatically persisted to the config file.
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

        let mut active = self.active_provider_id.lock().unwrap();
        *active = Some(id.to_string());

        // Persist to config
        self.config.save("active_ocr_provider", &id.to_string())?;
        Ok(())
    }

    /// Returns the currently active provider.
    ///
    /// # Returns
    ///
    /// * `Option<Arc<dyn OcrProvider>>` - The active provider if one is set, None otherwise
    pub fn get_active(&self) -> Option<Arc<dyn OcrProvider>> {
        let active_id = self.active_provider_id.lock().unwrap();
        active_id
            .as_ref()
            .and_then(|id| self.providers.get(id).cloned())
    }

    /// Returns a list of all registered providers.
    ///
    /// # Returns
    ///
    /// * `Vec<Arc<dyn OcrProvider>>` - List of all registered providers
    pub fn list_all(&self) -> Vec<Arc<dyn OcrProvider>> {
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
    /// * `Option<Arc<dyn OcrProvider>>` - The provider if found, None otherwise
    pub fn get(&self, id: &str) -> Option<Arc<dyn OcrProvider>> {
        self.providers.get(id).cloned()
    }

    /// Restores the active provider from the config file.
    ///
    /// Skips if the provider ID is not registered.
    pub fn restore_from_config(&mut self) -> Result<()> {
        if let Ok(active_id) = self.config.load::<String>("active_ocr_provider") {
            if self.providers.contains_key(&active_id) {
                let mut active = self.active_provider_id.lock().unwrap();
                *active = Some(active_id);
            }
        }
        Ok(())
    }

    /// Recognizes text using the active provider.
    ///
    /// This method:
    /// 1. Gets the active provider
    /// 2. Calls the provider's recognize method
    /// 3. Returns the result
    ///
    /// # Arguments
    ///
    /// * `request` - The OCR request
    ///
    /// # Returns
    ///
    /// * `Result<OcrResult>` - Result from the active provider
    ///
    /// # Errors
    ///
    /// Returns an error if no provider is active.
    pub async fn recognize(&self, request: &OcrRequest) -> Result<OcrResult> {
        let start = Instant::now();

        // Get active provider
        let active_provider = self.get_active();

        // Check if a provider is active
        let provider = active_provider
            .ok_or_else(|| "No active OCR provider".to_string())?;

        let provider_id = provider.id().to_string();

        // Call provider's recognize method
        let result = provider.recognize(request).await?;

        // Publish domain event if event bus is attached
        if let Some(event_bus) = &self.event_bus {
            event_bus.publish(DomainEvent::OcrCompleted {
                request: request.clone(),
                result: result.clone(),
                provider_used: provider_id,
                timestamp: Utc::now(),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }

        Ok(result)
    }
}

use super::OcrProvider;
use crate::domain::events::DomainEvent;
use crate::domain::ocr::{OcrRequest, OcrResult};
use crate::infrastructure::events::EventBus;
use crate::infrastructure::storage::ConfigFile;
use crate::Result;
use chrono::Utc;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Coordinator for managing OCR providers and operations.
///
/// This coordinator consolidates provider registration, activation management,
/// and OCR execution. It replaces the previous Registry + Service split with
/// a single cohesive component.
///
/// Unlike TranslationCoordinator, only ONE provider can be active at a time
/// (single-select pattern).
///
/// Concurrency model: Providers manage their own internal state.
/// Coordinator uses Mutex for active provider tracking.
pub struct OcrCoordinator {
    /// Map of provider ID to provider instance
    /// Providers are responsible for their own thread-safety
    providers: Mutex<HashMap<String, Arc<RwLock<dyn OcrProvider>>>>,
    /// Currently active provider ID (single-select)
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
            providers: Mutex::new(HashMap::new()),
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
    pub fn register<T: OcrProvider + 'static>(&self, provider: T) -> Result<()> {
        let id = provider.id().to_string();
        let mut providers = self.providers.lock().unwrap();
        if providers.contains_key(&id) {
            return Err(format!("Provider already registered: {}", id).into());
        }
        providers.insert(id, Arc::new(RwLock::new(provider)));
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
        // Lock active first to serialize the entire activate operation
        let mut active = self.active_provider_id.lock().unwrap();

        // Validate provider exists
        let providers = self.providers.lock().unwrap();
        if !providers.contains_key(id) {
            return Err(format!("Provider not found: {}", id).into());
        }
        drop(providers);

        // Persist to disk
        self.config.save("active_ocr_provider", &id.to_string())?;

        // Update memory only after successful persistence
        *active = Some(id.to_string());

        Ok(())
    }

    /// Returns the currently active provider.
    ///
    /// # Returns
    ///
    /// * `Option<Arc<RwLock<dyn OcrProvider>>>` - The active provider if one is set, None otherwise
    pub fn get_active(&self) -> Option<Arc<RwLock<dyn OcrProvider>>> {
        let active_id = self.active_provider_id.lock().unwrap();
        let providers = self.providers.lock().unwrap();
        active_id.as_ref().and_then(|id| providers.get(id).cloned())
    }

    /// Returns a list of all registered providers.
    ///
    /// # Returns
    ///
    /// * `Vec<Arc<RwLock<dyn OcrProvider>>>` - List of all registered providers
    pub fn list_all(&self) -> Vec<Arc<RwLock<dyn OcrProvider>>> {
        let providers = self.providers.lock().unwrap();
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
    /// * `Option<Arc<RwLock<dyn OcrProvider>>>` - The provider if found, None otherwise
    pub fn get(&self, id: &str) -> Option<Arc<RwLock<dyn OcrProvider>>> {
        let providers = self.providers.lock().unwrap();
        providers.get(id).cloned()
    }

    /// Restores the active provider from the config file.
    ///
    /// Skips if the provider ID is not registered.
    pub fn restore_from_config(&self) -> Result<()> {
        if let Ok(active_id) = self.config.load::<String>("active_ocr_provider") {
            let providers = self.providers.lock().unwrap();
            if providers.contains_key(&active_id) {
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

        // Get active provider (cloned Arc, no lock held)
        let provider_lock = self
            .get_active()
            .ok_or_else(|| "No active OCR provider".to_string())?;

        let provider_id = provider_lock.read().id().to_string();

        // Call provider's recognize method
        let result = {
            let provider = provider_lock.read();
            provider.recognize(request).await?
        };

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

    pub async fn recognize_image(&self, image_data: Vec<u8>) -> Result<OcrResult> {
        let request = OcrRequest {
            image_data,
            language: None,
        };

        self.recognize(&request).await
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
        let providers = self.providers.lock().unwrap();

        let provider_lock = providers
            .get(provider_id)
            .ok_or_else(|| format!("Provider not found: {}", provider_id))?
            .clone();
        drop(providers);

        let result = {
            let mut provider = provider_lock.write();
            provider.reconfigure_credentials(credentials)
        };

        if let Err(ref e) = result {
            if let Some(event_bus) = &self.event_bus {
                event_bus.publish(DomainEvent::ProviderConfigurationFailed {
                    provider_id: provider_id.to_string(),
                    error_message: e.to_string(),
                    timestamp: Utc::now(),
                });
            }
        }

        result
    }
}

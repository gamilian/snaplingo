use super::TranslationProvider;
use crate::infrastructure::storage::ConfigFile;
use crate::Result;
use std::collections::HashMap;
use std::sync::Arc;

/// Registry for managing translation providers.
///
/// This registry maintains a collection of translation providers and tracks which ones
/// are currently active. Multiple providers can be active simultaneously, allowing for
/// concurrent translation requests to different services.
pub struct TranslationRegistry {
    /// Map of provider ID to provider instance
    providers: HashMap<String, Arc<dyn TranslationProvider>>,
    /// List of active provider IDs (in order of activation)
    active: Vec<String>,
    /// Configuration file for persisting active providers
    config: Arc<ConfigFile>,
}

impl TranslationRegistry {
    /// Creates a new empty translation registry.
    pub fn new(config: Arc<ConfigFile>) -> Self {
        Self {
            providers: HashMap::new(),
            active: Vec::new(),
            config,
        }
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
    pub fn activate(&mut self, id: &str) -> Result<()> {
        if !self.providers.contains_key(id) {
            return Err(format!("Provider not found: {}", id).into());
        }
        if !self.active.contains(&id.to_string()) {
            self.active.push(id.to_string());
        }
        self.persist_active()?;
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
    pub fn deactivate(&mut self, id: &str) -> Result<()> {
        if !self.providers.contains_key(id) {
            return Err(format!("Provider not found: {}", id).into());
        }
        self.active.retain(|active_id| active_id != id);
        self.persist_active()?;
        Ok(())
    }

    /// Returns a list of all active providers.
    ///
    /// # Returns
    ///
    /// * `Vec<Arc<dyn TranslationProvider>>` - List of active providers in activation order
    pub fn get_active(&self) -> Vec<Arc<dyn TranslationProvider>> {
        self.active
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
            self.active.clear();
            for id in active_ids {
                if self.providers.contains_key(&id) {
                    self.active.push(id);
                }
            }
        }
        Ok(())
    }

    /// Persists the current active provider list to the config file.
    fn persist_active(&self) -> Result<()> {
        self.config.save("active_translation_providers", &self.active)
    }
}

impl Default for TranslationRegistry {
    fn default() -> Self {
        // Note: Default implementation requires a config file path.
        // In practice, always use TranslationRegistry::new(config) instead.
        panic!("TranslationRegistry::default() should not be used. Use TranslationRegistry::new(config) instead.");
    }
}

use super::OcrProvider;
use crate::infrastructure::storage::ConfigFile;
use crate::Result;
use std::collections::HashMap;
use std::sync::Arc;

/// Registry for managing OCR providers.
///
/// This registry maintains a collection of OCR providers and tracks which one
/// is currently active. Unlike TranslationRegistry, only ONE provider can be
/// active at a time (single-select pattern).
pub struct OcrRegistry {
    /// Map of provider ID to provider instance
    providers: HashMap<String, Arc<dyn OcrProvider>>,
    /// Currently active provider ID (single-select)
    active_provider_id: Option<String>,
    /// Configuration file for persisting active provider
    config: Arc<ConfigFile>,
}

impl OcrRegistry {
    /// Creates a new empty OCR registry.
    pub fn new(config: Arc<ConfigFile>) -> Self {
        Self {
            providers: HashMap::new(),
            active_provider_id: None,
            config,
        }
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
    pub fn activate(&mut self, id: &str) -> Result<()> {
        if !self.providers.contains_key(id) {
            return Err(format!("Provider not found: {}", id).into());
        }
        self.active_provider_id = Some(id.to_string());
        self.persist_active()?;
        Ok(())
    }

    /// Returns the currently active provider.
    ///
    /// # Returns
    ///
    /// * `Option<Arc<dyn OcrProvider>>` - The active provider if one is set, None otherwise
    pub fn get_active(&self) -> Option<Arc<dyn OcrProvider>> {
        self.active_provider_id
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
                self.active_provider_id = Some(active_id);
            }
        }
        Ok(())
    }

    /// Persists the current active provider to the config file.
    fn persist_active(&self) -> Result<()> {
        if let Some(ref id) = self.active_provider_id {
            self.config.save("active_ocr_provider", id)?;
        }
        Ok(())
    }
}

impl Default for OcrRegistry {
    fn default() -> Self {
        // Note: Default implementation requires a config file path.
        // In practice, always use OcrRegistry::new(config) instead.
        panic!("OcrRegistry::default() should not be used. Use OcrRegistry::new(config) instead.");
    }
}

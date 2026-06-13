use super::OcrProvider;
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
}

impl OcrRegistry {
    /// Creates a new empty OCR registry.
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            active_provider_id: None,
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
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the provider to activate
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Ok if successful, Err if the provider doesn't exist
    pub fn activate(&mut self, id: &str) -> Result<()> {
        if !self.providers.contains_key(id) {
            return Err(format!("Provider not found: {}", id).into());
        }
        self.active_provider_id = Some(id.to_string());
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
}

impl Default for OcrRegistry {
    fn default() -> Self {
        Self::new()
    }
}

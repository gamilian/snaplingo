use super::registry::TranslationRegistry;
use crate::domain::translation::{TranslationRequest, TranslationResult};
use crate::Result;
use std::sync::{Arc, Mutex};

/// Service for managing translation operations with concurrent provider execution.
///
/// This service handles translation requests by coordinating with the TranslationRegistry
/// and executing translations concurrently across all active providers.
///
/// NOTE: History recording will be added in Phase 5
pub struct TranslationService {
    registry: Arc<Mutex<TranslationRegistry>>,
}

impl TranslationService {
    /// Creates a new TranslationService with the given registry.
    pub fn new(registry: Arc<Mutex<TranslationRegistry>>) -> Self {
        Self { registry }
    }

    /// Translates text using all active providers concurrently.
    ///
    /// This method:
    /// 1. Gets all active providers from the registry
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
        // Get active providers
        let active_providers = {
            let registry = self.registry.lock().unwrap();
            registry.get_active()
        };

        // Check if any providers are active
        if active_providers.is_empty() {
            return Err("No active translation providers".into());
        }

        // Spawn concurrent translation tasks
        let mut tasks = Vec::new();
        for provider in active_providers {
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

        // TODO(Phase 5): Add history recording here

        Ok(results)
    }
}

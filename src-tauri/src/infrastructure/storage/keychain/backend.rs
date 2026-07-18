use crate::error::Result;

/// Provider credential persistence backend.
pub trait KeychainBackend: Send + Sync {
    /// Save a secret with the given key
    fn save(&self, key: &str, value: &str) -> Result<()>;

    /// Load a secret by key
    fn load(&self, key: &str) -> Result<String>;

    /// Delete a secret by key
    fn delete(&self, key: &str) -> Result<()>;
}

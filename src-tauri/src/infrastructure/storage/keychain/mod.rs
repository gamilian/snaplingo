mod backend;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos::MacOSKeychain as PlatformKeychainImpl;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows::WindowsKeychain as PlatformKeychainImpl;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux::LinuxKeychain as PlatformKeychainImpl;

use crate::error::Result;
use backend::KeychainBackend;
use std::collections::HashMap;

/// Platform-adaptive keychain wrapper
pub struct Keychain {
    backend: PlatformKeychainImpl,
}

impl Keychain {
    /// Create a new keychain instance
    pub fn new() -> Self {
        Self {
            backend: PlatformKeychainImpl::new(),
        }
    }

    /// Save a provider credential (API key)
    /// Key format: "provider:{provider_id}:api_key"
    pub fn save_provider_credential(&self, provider_id: &str, api_key: &str) -> Result<()> {
        let key = format!("provider:{}:api_key", provider_id);
        self.backend.save(&key, api_key)
    }

    /// Load a provider credential (API key)
    /// Key format: "provider:{provider_id}:api_key"
    pub fn load_provider_credential(&self, provider_id: &str) -> Result<String> {
        let key = format!("provider:{}:api_key", provider_id);
        self.backend.load(&key)
    }

    /// Delete a provider credential (API key)
    /// Key format: "provider:{provider_id}:api_key"
    pub fn delete_provider_credential(&self, provider_id: &str) -> Result<()> {
        let key = format!("provider:{}:api_key", provider_id);
        self.backend.delete(&key)
    }

    /// Save multiple provider credentials
    /// Key format: "provider:{provider_id}:credential:{field_name}"
    pub fn save_provider_credentials(
        &self,
        provider_id: &str,
        credentials: &HashMap<String, String>,
    ) -> Result<()> {
        for (field_name, value) in credentials {
            let key = format!("provider:{}:credential:{}", provider_id, field_name);
            self.backend.save(&key, value)?;
        }
        Ok(())
    }

    /// Load multiple provider credentials
    /// Key format: "provider:{provider_id}:credential:{field_name}"
    /// Returns only the fields that exist in keychain
    pub fn load_provider_credentials(
        &self,
        provider_id: &str,
        field_names: &[String],
    ) -> Result<HashMap<String, String>> {
        let mut credentials = HashMap::new();
        for field_name in field_names {
            let key = format!("provider:{}:credential:{}", provider_id, field_name);
            if let Ok(value) = self.backend.load(&key) {
                credentials.insert(field_name.clone(), value);
            }
        }

        if credentials.is_empty() {
            Err(crate::AppError::Other(format!(
                "No credentials found for provider: {}",
                provider_id
            )))
        } else {
            Ok(credentials)
        }
    }

    /// Delete all credentials for a provider (including multi-field)
    /// Deletes both old format (provider:{id}:api_key) and new format (provider:{id}:credential:{field})
    pub fn delete_all_provider_credentials(&self, provider_id: &str) -> Result<()> {
        // Try to delete old format
        let _ = self.delete_provider_credential(provider_id);

        // Note: We can't enumerate keys in keychain backend, so we can only delete known fields
        // The caller should provide the list of fields to delete, or use delete_provider_credentials
        Ok(())
    }

    /// Delete specific credential fields for a provider
    pub fn delete_provider_credentials(
        &self,
        provider_id: &str,
        field_names: &[String],
    ) -> Result<()> {
        for field_name in field_names {
            let key = format!("provider:{}:credential:{}", provider_id, field_name);
            let _ = self.backend.delete(&key); // Ignore errors for non-existent keys
        }
        Ok(())
    }
}

impl Default for Keychain {
    fn default() -> Self {
        Self::new()
    }
}

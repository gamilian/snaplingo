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
}

impl Default for Keychain {
    fn default() -> Self {
        Self::new()
    }
}

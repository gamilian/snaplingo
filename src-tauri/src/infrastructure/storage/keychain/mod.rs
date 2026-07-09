mod backend;

// Re-export KeychainBackend for testing
#[cfg(test)]
pub use backend::KeychainBackend;

#[cfg(not(test))]
use backend::KeychainBackend;

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

use crate::error::{Result, AppError};
use std::collections::HashMap;

/// Check if an error is a "not found" / "no entry" error from keychain
pub fn is_keychain_not_found(error: &crate::AppError) -> bool {
    match error {
        crate::AppError::Keychain(e) => matches!(e, keyring::Error::NoEntry),
        _ => false,
    }
}

/// Snapshot of provider credentials for rollback
#[derive(Debug, Clone)]
pub struct CredentialSnapshot {
    /// Simple API key: Present(value) or Absent
    pub api_key: Option<Option<String>>,
    /// Structured credentials: field_name -> Present(value) or Absent
    pub structured: HashMap<String, Option<String>>,
}

impl CredentialSnapshot {
    /// Create an empty snapshot
    pub fn new() -> Self {
        Self {
            api_key: None,
            structured: HashMap::new(),
        }
    }
}

/// Platform-adaptive keychain wrapper
pub struct Keychain {
    backend: Box<dyn KeychainBackend>,
}

impl Keychain {
    /// Create a new keychain instance with the platform backend
    pub fn new() -> Self {
        Self {
            backend: Box::new(PlatformKeychainImpl::new()),
        }
    }

    /// Create a keychain with a custom backend (tests only)
    #[cfg(test)]
    pub fn with_backend(backend: impl KeychainBackend + 'static) -> Self {
        Self {
            backend: Box::new(backend),
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

    /// Save provider credentials with automatic rollback on failure.
    /// Returns the fields that were successfully saved before failure (for manual cleanup).
    pub fn save_provider_credentials_transactional(
        &self,
        provider_id: &str,
        credentials: &HashMap<String, String>,
        snapshot: &CredentialSnapshot,
    ) -> Result<()> {
        let mut saved_fields: Vec<String> = Vec::new();

        for (field_name, value) in credentials {
            let key = format!("provider:{}:credential:{}", provider_id, field_name);
            if let Err(e) = self.backend.save(&key, value) {
                // Rollback: restore snapshot for all saved fields.
                // Fields not in the snapshot default to None (was absent → delete).
                let rollback_snapshot = CredentialSnapshot {
                    api_key: None, // Don't touch simple API key
                    structured: saved_fields
                        .iter()
                        .map(|f| {
                            let val = snapshot.structured.get(f).cloned().unwrap_or(None);
                            (f.clone(), val)
                        })
                        .collect(),
                };

                if let Err(rollback_err) = self.restore_provider_credentials(provider_id, &rollback_snapshot) {
                    return Err(AppError::Other(format!(
                        "Save failed: {}. Rollback also failed: {}. Credentials may be inconsistent.",
                        e, rollback_err
                    )));
                }

                return Err(e);
            }
            saved_fields.push(field_name.clone());
        }
        Ok(())
    }

    /// Snapshot provider credentials for rollback
    pub fn snapshot_provider_credentials(
        &self,
        provider_id: &str,
        field_names: &[String],
    ) -> Result<CredentialSnapshot> {
        let mut snapshot = CredentialSnapshot::new();

        // Snapshot simple API key
        match self.load_provider_credential(provider_id) {
            Ok(key) => snapshot.api_key = Some(Some(key)),
            Err(ref e) if is_keychain_not_found(e) => snapshot.api_key = Some(None),
            Err(e) => return Err(e),
        }

        // Snapshot structured credentials
        for field_name in field_names {
            let key = format!("provider:{}:credential:{}", provider_id, field_name);
            match self.backend.load(&key) {
                Ok(value) => {
                    snapshot.structured.insert(field_name.clone(), Some(value));
                }
                Err(ref e) if is_keychain_not_found(e) => {
                    snapshot.structured.insert(field_name.clone(), None);
                }
                Err(e) => return Err(e),
            }
        }

        Ok(snapshot)
    }

    /// Restore provider credentials from snapshot
    pub fn restore_provider_credentials(
        &self,
        provider_id: &str,
        snapshot: &CredentialSnapshot,
    ) -> Result<()> {
        // Restore simple API key
        if let Some(api_key_state) = &snapshot.api_key {
            match api_key_state {
                Some(key) => {
                    // Restore old value
                    self.save_provider_credential(provider_id, key)?;
                }
                None => {
                    // Was absent, delete current value
                    if let Err(e) = self.delete_provider_credential(provider_id) {
                        // Only ignore "not found", propagate real failures
                        if !is_keychain_not_found(&e) {
                            return Err(e);
                        }
                    }
                }
            }
        }

        // Restore structured credentials
        for (field_name, old_value) in &snapshot.structured {
            let key = format!("provider:{}:credential:{}", provider_id, field_name);
            match old_value {
                Some(val) => {
                    // Restore old value
                    self.backend.save(&key, val)?;
                }
                None => {
                    // Was absent, delete current value
                    if let Err(e) = self.backend.delete(&key) {
                        // Only ignore "not found", propagate real failures
                        if !is_keychain_not_found(&e) {
                            return Err(e);
                        }
                    }
                }
            }
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
    /// Only ignores "not found" errors, other errors are propagated
    pub fn delete_provider_credentials(
        &self,
        provider_id: &str,
        field_names: &[String],
    ) -> Result<()> {
        for field_name in field_names {
            let key = format!("provider:{}:credential:{}", provider_id, field_name);
            if let Err(e) = self.backend.delete(&key) {
                // Only ignore "not found" errors, propagate real failures
                if !is_keychain_not_found(&e) {
                    return Err(e);
                }
            }
        }
        Ok(())
    }
}

impl Default for Keychain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// In-memory backend used to exercise the trait-object wiring and Keychain
    /// public API without touching the real platform keychain.
    struct StubKeychainBackend {
        store: std::sync::Mutex<std::collections::HashMap<String, String>>,
    }

    impl StubKeychainBackend {
        fn new() -> Self {
            Self {
                store: std::sync::Mutex::new(std::collections::HashMap::new()),
            }
        }
    }

    impl KeychainBackend for StubKeychainBackend {
        fn save(&self, key: &str, value: &str) -> Result<()> {
            self.store
                .lock()
                .unwrap()
                .insert(key.to_string(), value.to_string());
            Ok(())
        }

        fn load(&self, key: &str) -> Result<String> {
            self.store
                .lock()
                .unwrap()
                .get(key)
                .cloned()
                .ok_or_else(|| crate::AppError::Keychain(keyring::Error::NoEntry))
        }

        fn delete(&self, key: &str) -> Result<()> {
            self.store.lock().unwrap().remove(key);
            Ok(())
        }
    }

    /// Keychain backend that can be configured to fail on save operations
    struct FailingKeychainBackend {
        store: std::sync::Mutex<std::collections::HashMap<String, String>>,
        fail_on_save_after_n: std::sync::Mutex<Option<usize>>,
        save_count: std::sync::Mutex<usize>,
    }

    impl FailingKeychainBackend {
        fn new() -> Self {
            Self {
                store: std::sync::Mutex::new(std::collections::HashMap::new()),
                fail_on_save_after_n: std::sync::Mutex::new(None),
                save_count: std::sync::Mutex::new(0),
            }
        }

        /// Fail on the Nth save operation (0-indexed), then clear the failure so subsequent saves succeed
        fn fail_on_save_after(&self, n: usize) {
            *self.fail_on_save_after_n.lock().unwrap() = Some(n);
        }
    }

    impl KeychainBackend for FailingKeychainBackend {
        fn save(&self, key: &str, value: &str) -> Result<()> {
            let mut count = self.save_count.lock().unwrap();
            let current = *count;
            *count += 1;

            // Check and clear failure flag atomically to avoid deadlock
            let should_fail = {
                let mut fail_config = self.fail_on_save_after_n.lock().unwrap();
                if let Some(fail_after) = *fail_config {
                    if current == fail_after {
                        // Clear the failure flag so rollback saves can succeed
                        *fail_config = None;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            };

            if should_fail {
                return Err(crate::AppError::Other(format!(
                    "Simulated save failure at operation {}",
                    current
                )));
            }

            self.store
                .lock()
                .unwrap()
                .insert(key.to_string(), value.to_string());
            Ok(())
        }

        fn load(&self, key: &str) -> Result<String> {
            self.store
                .lock()
                .unwrap()
                .get(key)
                .cloned()
                .ok_or_else(|| crate::AppError::Keychain(keyring::Error::NoEntry))
        }

        fn delete(&self, key: &str) -> Result<()> {
            self.store.lock().unwrap().remove(key);
            Ok(())
        }
    }

    #[test]
    fn keychain_with_backend_round_trips_provider_credential() {
        let keychain = Keychain::with_backend(StubKeychainBackend::new());

        keychain.save_provider_credential("custom-llm-1", "secret-key").unwrap();
        assert_eq!(
            keychain.load_provider_credential("custom-llm-1").unwrap(),
            "secret-key"
        );

        keychain.delete_provider_credential("custom-llm-1").unwrap();
        assert!(keychain.load_provider_credential("custom-llm-1").is_err());
    }

    #[test]
    fn save_credentials_transactional_rolls_back_on_failure() {
        // Create a keychain with stub backend for initial setup
        let keychain = Keychain::with_backend(StubKeychainBackend::new());
        keychain.save_provider_credential("test-provider", "old-simple-key").unwrap();
        let mut existing_creds = std::collections::HashMap::new();
        existing_creds.insert("field1".to_string(), "old-value1".to_string());
        keychain.save_provider_credentials("test-provider", &existing_creds).unwrap();

        // Snapshot
        let snapshot = keychain.snapshot_provider_credentials(
            "test-provider",
            &vec!["field1".to_string(), "field2".to_string()],
        ).unwrap();

        // Create failing backend and copy state
        let failing_backend = FailingKeychainBackend::new();
        failing_backend.store.lock().unwrap().insert(
            "provider:test-provider:api_key".to_string(),
            "old-simple-key".to_string(),
        );
        failing_backend.store.lock().unwrap().insert(
            "provider:test-provider:credential:field1".to_string(),
            "old-value1".to_string(),
        );

        failing_backend.fail_on_save_after(1); // Fail on 2nd save operation
        let keychain_failing = Keychain::with_backend(failing_backend);

        // Try to save 2 fields, should fail on the 2nd
        let mut new_creds = std::collections::HashMap::new();
        new_creds.insert("field1".to_string(), "new-value1".to_string());
        new_creds.insert("field2".to_string(), "new-value2".to_string());

        let result = keychain_failing.save_provider_credentials_transactional(
            "test-provider",
            &new_creds,
            &snapshot,
        );

        // Should fail
        assert!(result.is_err(), "Expected transactional save to fail");

        // Verify rollback happened: check both fields
        // Due to HashMap iteration order, we don't know which field was saved first
        // But we can verify: the field that WAS saved should be rolled back
        // and field2 (which didn't exist) should still not exist
        let field1_result = keychain_failing.backend.load("provider:test-provider:credential:field1");
        let field2_result = keychain_failing.backend.load("provider:test-provider:credential:field2");

        // field1 should be back to old value (either never changed, or rolled back)
        assert_eq!(field1_result.unwrap(), "old-value1", "field1 should be old value");

        // field2 should not exist (was absent before, either never saved or rolled back)
        assert!(field2_result.is_err(), "field2 should not exist after rollback");
    }

    #[test]
    fn restore_provider_credentials_handles_present_and_absent() {
        let keychain = Keychain::with_backend(StubKeychainBackend::new());

        // Setup: field1 exists, field2 absent
        keychain.save_provider_credential("test-provider", "simple-key").unwrap();
        let mut creds = std::collections::HashMap::new();
        creds.insert("field1".to_string(), "value1".to_string());
        keychain.save_provider_credentials("test-provider", &creds).unwrap();

        // Create snapshot
        let snapshot = keychain.snapshot_provider_credentials(
            "test-provider",
            &vec!["field1".to_string(), "field2".to_string()],
        ).unwrap();

        // Verify snapshot captured the state
        assert_eq!(snapshot.api_key, Some(Some("simple-key".to_string())));
        assert_eq!(snapshot.structured.get("field1"), Some(&Some("value1".to_string())));
        assert_eq!(snapshot.structured.get("field2"), Some(&None));

        // Modify state
        keychain.save_provider_credential("test-provider", "changed-key").unwrap();
        let mut new_creds = std::collections::HashMap::new();
        new_creds.insert("field1".to_string(), "changed1".to_string());
        new_creds.insert("field2".to_string(), "new-field2".to_string());
        keychain.save_provider_credentials("test-provider", &new_creds).unwrap();

        // Restore from snapshot
        keychain.restore_provider_credentials("test-provider", &snapshot).unwrap();

        // Verify restoration
        assert_eq!(keychain.load_provider_credential("test-provider").unwrap(), "simple-key");
        let field1 = keychain.backend.load("provider:test-provider:credential:field1").unwrap();
        assert_eq!(field1, "value1");

        // field2 should be deleted (was absent in snapshot)
        assert!(keychain.backend.load("provider:test-provider:credential:field2").is_err());
    }
}

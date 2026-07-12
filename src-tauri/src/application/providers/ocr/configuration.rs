use std::collections::HashMap;
use std::sync::Arc;

use crate::application::providers::common::CredentialField;
use crate::application::providers::ProviderCredentialStore;

use super::OcrCoordinator;

pub struct OcrProviderConfiguration {
    coordinator: Arc<OcrCoordinator>,
    credential_store: Arc<dyn ProviderCredentialStore>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::providers::common::Provider;
    use crate::application::providers::ocr::OcrProvider;
    use crate::domain::ocr::{OcrRequest, OcrResult};
    use crate::infrastructure::storage::{ConfigFile, Keychain, KeychainBackend};
    use async_trait::async_trait;
    use std::sync::Mutex;

    struct StubKeychainBackend {
        store: Mutex<HashMap<String, String>>,
    }

    impl StubKeychainBackend {
        fn new() -> Self {
            Self {
                store: Mutex::new(HashMap::new()),
            }
        }
    }

    impl KeychainBackend for StubKeychainBackend {
        fn save(&self, key: &str, value: &str) -> crate::Result<()> {
            self.store
                .lock()
                .unwrap()
                .insert(key.to_string(), value.to_string());
            Ok(())
        }

        fn load(&self, key: &str) -> crate::Result<String> {
            self.store
                .lock()
                .unwrap()
                .get(key)
                .cloned()
                .ok_or_else(|| crate::AppError::Keychain(keyring::Error::NoEntry))
        }

        fn delete(&self, key: &str) -> crate::Result<()> {
            self.store.lock().unwrap().remove(key);
            Ok(())
        }
    }

    struct RejectingOcrProvider;

    impl Provider for RejectingOcrProvider {
        fn id(&self) -> &str {
            "rejecting-ocr"
        }

        fn name(&self) -> &str {
            "Rejecting OCR"
        }

        fn is_configured(&self) -> bool {
            true
        }

        fn requires_api_key(&self) -> bool {
            true
        }

        fn validate_credentials(&self, credentials: &HashMap<String, String>) -> crate::Result<()> {
            if credentials.get("api_key").map(String::as_str) == Some("blocked") {
                return Err(crate::AppError::Other("blocked api key".to_string()));
            }

            Ok(())
        }

        fn reconfigure_credentials(
            &mut self,
            _credentials: &HashMap<String, String>,
        ) -> crate::Result<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl OcrProvider for RejectingOcrProvider {
        async fn recognize(&self, _request: &OcrRequest) -> crate::Result<OcrResult> {
            unimplemented!("credential validation test should not run OCR")
        }
    }

    #[test]
    fn save_credentials_uses_provider_validation_before_persisting() {
        let coordinator = Arc::new(OcrCoordinator::new(Arc::new(ConfigFile::new_temp())));
        coordinator.register(RejectingOcrProvider).unwrap();
        let keychain = Arc::new(Keychain::with_backend(StubKeychainBackend::new()));
        let configuration = OcrProviderConfiguration::new(coordinator, keychain.clone());
        let credentials = HashMap::from([("api_key".to_string(), "blocked".to_string())]);

        let result = configuration.save_credentials("rejecting-ocr", &credentials);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("blocked api key"));
        assert!(keychain
            .load_provider_credentials("rejecting-ocr", &["api_key".to_string()])
            .is_err());
    }
}

impl OcrProviderConfiguration {
    pub fn new(
        coordinator: Arc<OcrCoordinator>,
        credential_store: Arc<dyn ProviderCredentialStore>,
    ) -> Self {
        Self {
            coordinator,
            credential_store,
        }
    }

    pub fn credential_schema(&self, provider_id: &str) -> crate::Result<Vec<CredentialField>> {
        let provider_lock = self
            .coordinator
            .get(provider_id)
            .ok_or_else(|| format!("Provider not found: {}", provider_id))?;

        let fields = provider_lock.read().credential_fields();
        Ok(fields)
    }

    pub fn save_credentials(
        &self,
        provider_id: &str,
        credentials: &HashMap<String, String>,
    ) -> crate::Result<()> {
        let provider_lock = self
            .coordinator
            .get(provider_id)
            .ok_or_else(|| format!("Provider not found: {}", provider_id))?;
        let expected_fields = {
            let provider = provider_lock.read();
            let fields = provider.credential_fields();

            if fields.is_empty() {
                if credentials.is_empty() {
                    return Ok(());
                }

                return Err(format!("Provider {} does not accept credentials", provider_id).into());
            }

            provider.validate_credentials(credentials)?;
            fields
        };

        let field_names: Vec<String> = expected_fields
            .iter()
            .map(|field| field.name.clone())
            .collect();
        let snapshot = self
            .credential_store
            .snapshot_provider_credentials(provider_id, &field_names)
            .map_err(|e| format!("Failed to snapshot credentials: {}", e))?;

        self.credential_store
            .save_provider_credentials_transactional(provider_id, credentials, &snapshot)?;

        if let Err(e) = self
            .coordinator
            .reconfigure_provider(provider_id, credentials)
        {
            let _ = self
                .credential_store
                .restore_provider_credentials(provider_id, &snapshot);
            return Err(format!("Failed to reconfigure provider: {}", e).into());
        }

        Ok(())
    }
}

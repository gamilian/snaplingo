use std::collections::HashMap;
use std::sync::Arc;

use crate::application::providers::common::CredentialField;
use crate::application::providers::validate_required_credentials;
use crate::infrastructure::storage::Keychain;

use super::OcrCoordinator;

pub struct OcrProviderConfiguration {
    coordinator: Arc<OcrCoordinator>,
    keychain: Arc<Keychain>,
}

impl OcrProviderConfiguration {
    pub fn new(coordinator: Arc<OcrCoordinator>, keychain: Arc<Keychain>) -> Self {
        Self {
            coordinator,
            keychain,
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
        let expected_fields = self.credential_schema(provider_id)?;

        if expected_fields.is_empty() {
            if credentials.is_empty() {
                return Ok(());
            }

            return Err(format!("Provider {} does not accept credentials", provider_id).into());
        }

        validate_required_credentials(&expected_fields, credentials)?;

        let field_names: Vec<String> = expected_fields
            .iter()
            .map(|field| field.name.clone())
            .collect();
        let snapshot = self
            .keychain
            .snapshot_provider_credentials(provider_id, &field_names)
            .map_err(|e| format!("Failed to snapshot credentials: {}", e))?;

        self.keychain
            .save_provider_credentials_transactional(provider_id, credentials, &snapshot)?;

        if let Err(e) = self.coordinator.reconfigure_provider(provider_id, credentials) {
            let _ = self
                .keychain
                .restore_provider_credentials(provider_id, &snapshot);
            return Err(format!("Failed to reconfigure provider: {}", e).into());
        }

        Ok(())
    }
}

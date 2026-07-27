use std::collections::HashMap;
use std::sync::Arc;

use crate::application::providers::common::CredentialField;
use crate::application::providers::{ProviderChangeNotifier, ProviderCredentialStore};

use super::OcrCoordinator;

pub struct OcrProviderConfiguration {
    coordinator: Arc<OcrCoordinator>,
    credential_store: Arc<dyn ProviderCredentialStore>,
    change_notifier: Option<Arc<dyn ProviderChangeNotifier>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::providers::common::Provider;
    use crate::application::providers::ocr::OcrProvider;
    use crate::domain::ocr::{OcrRequest, OcrResult};
    use crate::infrastructure::storage::{Database, SqliteConfigStore, SqliteCredentialStore};
    use async_trait::async_trait;

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
        let coordinator = Arc::new(OcrCoordinator::new(Arc::new(SqliteConfigStore::new_temp())));
        coordinator.register(RejectingOcrProvider).unwrap();
        let credential_store = Arc::new(SqliteCredentialStore::new(Arc::new(
            Database::in_memory().unwrap(),
        )));
        let configuration = OcrProviderConfiguration::new(coordinator, credential_store.clone());
        let credentials = HashMap::from([("api_key".to_string(), "blocked".to_string())]);

        let result = configuration.save_credentials("rejecting-ocr", &credentials);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("blocked api key"));
        assert!(credential_store
            .load_provider_credentials("rejecting-ocr", &["api_key".to_string()])
            .is_err());
    }

    #[test]
    fn baidu_hydration_prefers_structured_credentials() {
        let credential_store = SqliteCredentialStore::new(Arc::new(Database::in_memory().unwrap()));
        credential_store
            .save_provider_credentials(
                "baidu-ocr",
                &HashMap::from([
                    ("api_key".to_string(), "structured-api".to_string()),
                    ("secret_key".to_string(), "structured-secret".to_string()),
                ]),
            )
            .unwrap();
        credential_store
            .save_provider_credential("baidu_ocr_api_key", "legacy-api")
            .unwrap();
        credential_store
            .save_provider_credential("baidu_ocr_secret_key", "legacy-secret")
            .unwrap();

        let credentials = load_baidu_ocr_credentials(&credential_store).unwrap();

        assert_eq!(credentials["api_key"], "structured-api");
        assert_eq!(credentials["secret_key"], "structured-secret");
    }

    #[test]
    fn baidu_hydration_preserves_legacy_credential_fallback() {
        let credential_store = SqliteCredentialStore::new(Arc::new(Database::in_memory().unwrap()));
        credential_store
            .save_provider_credential("baidu_ocr_api_key", "legacy-api")
            .unwrap();
        credential_store
            .save_provider_credential("baidu_ocr_secret_key", "legacy-secret")
            .unwrap();

        let credentials = load_baidu_ocr_credentials(&credential_store).unwrap();

        assert_eq!(credentials["api_key"], "legacy-api");
        assert_eq!(credentials["secret_key"], "legacy-secret");
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
            change_notifier: None,
        }
    }

    pub fn with_change_notifier(
        mut self,
        change_notifier: Arc<dyn ProviderChangeNotifier>,
    ) -> Self {
        self.change_notifier = Some(change_notifier);
        self
    }

    pub(crate) fn hydrate_credentials(&self) -> crate::Result<()> {
        if let Some(credentials) = load_baidu_ocr_credentials(self.credential_store.as_ref()) {
            self.coordinator
                .reconfigure_provider("baidu-ocr", &credentials)?;
        }
        Ok(())
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
            .save_provider_credentials(provider_id, credentials)?;

        if let Err(e) = self
            .coordinator
            .reconfigure_provider(provider_id, credentials)
        {
            let _ = self
                .credential_store
                .restore_provider_credentials(provider_id, &snapshot);
            return Err(format!("Failed to reconfigure provider: {}", e).into());
        }

        if let Some(notifier) = &self.change_notifier {
            notifier.providers_changed();
        }
        Ok(())
    }
}

fn load_baidu_ocr_credentials(
    credential_store: &dyn ProviderCredentialStore,
) -> Option<HashMap<String, String>> {
    credential_store
        .load_provider_credentials(
            "baidu-ocr",
            &["api_key".to_string(), "secret_key".to_string()],
        )
        .ok()
        .or_else(|| {
            let api_key = credential_store
                .load_provider_credential("baidu_ocr_api_key")
                .ok()?;
            let secret_key = credential_store
                .load_provider_credential("baidu_ocr_secret_key")
                .ok()?;
            Some(HashMap::from([
                ("api_key".to_string(), api_key),
                ("secret_key".to_string(), secret_key),
            ]))
        })
}

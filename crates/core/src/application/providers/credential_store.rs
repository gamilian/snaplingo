use std::collections::HashMap;

use crate::Result;

#[derive(Debug, Clone)]
pub struct CredentialSnapshot {
    pub api_key: Option<Option<String>>,
    pub structured: HashMap<String, Option<String>>,
}

pub trait ProviderCredentialStore: Send + Sync {
    fn save_provider_credential(&self, provider_id: &str, api_key: &str) -> Result<()>;
    fn load_provider_credential(&self, provider_id: &str) -> Result<String>;
    fn delete_provider_credential(&self, provider_id: &str) -> Result<()>;
    fn save_provider_credentials(
        &self,
        provider_id: &str,
        credentials: &HashMap<String, String>,
    ) -> Result<()>;
    fn snapshot_provider_credentials(
        &self,
        provider_id: &str,
        field_names: &[String],
    ) -> Result<CredentialSnapshot>;
    fn restore_provider_credentials(
        &self,
        provider_id: &str,
        snapshot: &CredentialSnapshot,
    ) -> Result<()>;
    fn load_provider_credentials(
        &self,
        provider_id: &str,
        field_names: &[String],
    ) -> Result<HashMap<String, String>>;
    fn delete_provider_credentials(&self, provider_id: &str, field_names: &[String]) -> Result<()>;
}

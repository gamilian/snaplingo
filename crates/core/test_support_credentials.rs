//! A local credential adapter for provider policy tests.
use crate::{
    application::providers::{CredentialSnapshot, ProviderCredentialStore},
    AppError, Result,
};
use std::{collections::HashMap, sync::Mutex};
#[derive(Default)]
pub(crate) struct MemoryCredentials(Mutex<HashMap<String, String>>);
fn key(provider: &str, field: &str) -> String {
    format!("{provider}:{field}")
}
impl ProviderCredentialStore for MemoryCredentials {
    fn save_provider_credential(&self, id: &str, value: &str) -> Result<()> {
        self.0
            .lock()
            .unwrap()
            .insert(key(id, "api_key"), value.into());
        Ok(())
    }
    fn load_provider_credential(&self, id: &str) -> Result<String> {
        self.0
            .lock()
            .unwrap()
            .get(&key(id, "api_key"))
            .cloned()
            .ok_or_else(|| AppError::CredentialNotFound(id.into()))
    }
    fn delete_provider_credential(&self, id: &str) -> Result<()> {
        self.0.lock().unwrap().remove(&key(id, "api_key"));
        Ok(())
    }
    fn save_provider_credentials(&self, id: &str, values: &HashMap<String, String>) -> Result<()> {
        let mut saved = self.0.lock().unwrap();
        for (field, value) in values {
            saved.insert(key(id, field), value.clone());
        }
        Ok(())
    }
    fn snapshot_provider_credentials(
        &self,
        id: &str,
        fields: &[String],
    ) -> Result<CredentialSnapshot> {
        let saved = self.0.lock().unwrap();
        Ok(CredentialSnapshot {
            api_key: Some(saved.get(&key(id, "api_key")).cloned()),
            structured: fields
                .iter()
                .map(|f| (f.clone(), saved.get(&key(id, f)).cloned()))
                .collect(),
        })
    }
    fn restore_provider_credentials(&self, id: &str, snapshot: &CredentialSnapshot) -> Result<()> {
        let mut saved = self.0.lock().unwrap();
        for (field, value) in &snapshot.structured {
            let k = key(id, field);
            if let Some(value) = value {
                saved.insert(k, value.clone());
            } else {
                saved.remove(&k);
            }
        }
        if let Some(value) = &snapshot.api_key {
            let k = key(id, "api_key");
            if let Some(value) = value {
                saved.insert(k, value.clone());
            } else {
                saved.remove(&k);
            }
        }
        Ok(())
    }
    fn load_provider_credentials(
        &self,
        id: &str,
        fields: &[String],
    ) -> Result<HashMap<String, String>> {
        let saved = self.0.lock().unwrap();
        fields
            .iter()
            .map(|field| {
                saved
                    .get(&key(id, field))
                    .cloned()
                    .map(|v| (field.clone(), v))
                    .ok_or_else(|| AppError::CredentialNotFound(id.into()))
            })
            .collect()
    }
    fn delete_provider_credentials(&self, id: &str, fields: &[String]) -> Result<()> {
        let mut saved = self.0.lock().unwrap();
        for field in fields {
            saved.remove(&key(id, field));
        }
        Ok(())
    }
}

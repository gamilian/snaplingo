use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Describes a single credential field required by a provider.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CredentialField {
    /// Internal field name (e.g., "api_key", "app_id", "secret_key")
    pub name: String,

    /// Human-readable label for the UI (e.g., "API Key", "App ID")
    pub label: String,

    /// Whether this field contains sensitive data (should be masked in UI)
    pub secret: bool,
}

impl CredentialField {
    pub fn new(name: impl Into<String>, label: impl Into<String>, secret: bool) -> Self {
        Self {
            name: name.into(),
            label: label.into(),
            secret,
        }
    }
}

/// Base trait for all provider types (OCR, Translation, TTS).
///
/// This trait defines the common interface that all providers must implement,
/// regardless of their specific functionality.
pub trait Provider: Send + Sync {
    /// Returns the unique identifier for this provider.
    ///
    /// This ID is used internally to identify and reference the provider.
    /// Example: "google-translate", "tesseract-ocr"
    fn id(&self) -> &str;

    /// Returns the human-readable display name for this provider.
    ///
    /// This name is shown to users in the UI.
    /// Example: "Google Translate", "Tesseract OCR"
    fn name(&self) -> &str;

    /// Returns whether this provider is configured and ready to use.
    ///
    /// A provider is configured if it has all the necessary settings
    /// (e.g., API keys, endpoints) to perform its function.
    fn is_configured(&self) -> bool;

    /// Returns whether this provider requires an API key.
    ///
    /// This helps the UI determine whether to prompt for credentials.
    fn requires_api_key(&self) -> bool;

    /// Returns the credential fields required by this provider.
    ///
    /// Default implementation returns a single "api_key" field for backward compatibility.
    /// Providers with multiple credentials (e.g., Baidu with app_id + secret_key) should override this.
    fn credential_fields(&self) -> Vec<CredentialField> {
        if self.requires_api_key() {
            vec![CredentialField::new("api_key", "API Key", true)]
        } else {
            vec![]
        }
    }

    /// Validates credentials before persistence or runtime reconfiguration.
    fn validate_credentials(&self, credentials: &HashMap<String, String>) -> crate::Result<()> {
        for field in self.credential_fields() {
            let value = credentials.get(&field.name).ok_or_else(|| {
                crate::AppError::Other(format!("Missing required field: {}", field.label))
            })?;

            if value.trim().is_empty() {
                return Err(crate::AppError::Other(format!(
                    "Field cannot be empty: {}",
                    field.label
                )));
            }
        }

        Ok(())
    }

    /// Reconfigures the provider's credentials at runtime.
    ///
    /// This allows hot-reloading of credentials without restarting the application.
    /// The credentials map should contain all fields returned by `credential_fields()`.
    ///
    /// # Arguments
    ///
    /// * `credentials` - HashMap mapping field names to their values
    ///
    /// # Returns
    ///
    /// * `Ok(())` if reconfiguration succeeded
    /// * `Err` if validation failed or required fields are missing
    ///
    /// # Default Implementation
    ///
    /// The default implementation returns an error. Providers that support runtime
    /// reconfiguration should override this method.
    fn reconfigure_credentials(
        &mut self,
        _credentials: &HashMap<String, String>,
    ) -> crate::Result<()> {
        Err(crate::AppError::Other(format!(
            "Provider {} does not support runtime credential reconfiguration",
            self.id()
        )))
    }
}

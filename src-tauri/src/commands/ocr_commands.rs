use crate::application::providers::common::CredentialField;
use crate::application::providers::ocr::OcrCoordinator;
use crate::application::providers::validate_required_credentials;
use crate::domain::ocr::{OcrRequest, OcrResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::State;

#[derive(Serialize, Deserialize)]
pub struct RecognizeImageRequest {
    pub image_data: Vec<u8>,
    pub language: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct OcrProviderInfo {
    pub id: String,
    pub name: String,
    pub is_configured: bool,
    pub requires_api_key: bool,
    pub is_active: bool,
}

#[tauri::command]
pub async fn recognize_image(
    request: RecognizeImageRequest,
    state: State<'_, crate::AppState>,
) -> Result<OcrResult, String> {
    let ocr_request = OcrRequest {
        image_data: request.image_data,
        language: request.language,
    };

    state
        .ocr_coordinator
        .recognize(&ocr_request)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn recognize_image_file(
    path: String,
    state: State<'_, crate::AppState>,
) -> Result<OcrResult, String> {
    let image_data = std::fs::read(path).map_err(|e| e.to_string())?;
    let ocr_request = OcrRequest {
        image_data,
        language: None,
    };

    state
        .ocr_coordinator
        .recognize(&ocr_request)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_ocr_providers(
    state: State<'_, crate::AppState>,
) -> Result<Vec<OcrProviderInfo>, String> {
    let all_providers = state.ocr_coordinator.list_all();
    let active = state.ocr_coordinator.get_active();
    let active_id = active
        .as_ref()
        .map(|active_p| active_p.read().id().to_string());

    let info: Vec<_> = all_providers
        .iter()
        .map(|p| {
            let provider = p.read();
            let id = provider.id().to_string();

            OcrProviderInfo {
                id: id.clone(),
                name: provider.name().to_string(),
                is_configured: provider.is_configured(),
                requires_api_key: provider.requires_api_key(),
                is_active: active_id.as_ref() == Some(&id),
            }
        })
        .collect();

    Ok(info)
}

#[tauri::command]
pub async fn activate_ocr_provider(
    provider_id: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state
        .ocr_coordinator
        .activate(&provider_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn configure_ocr_provider(
    provider_id: String,
    api_key: String,
    secret_key: Option<String>,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    let mut credentials = HashMap::new();
    credentials.insert("api_key".to_string(), api_key);

    if let Some(secret) = secret_key {
        credentials.insert("secret_key".to_string(), secret);
    }

    configure_ocr_provider_credentials_inner(&provider_id, &credentials, state.inner())
}

#[tauri::command]
pub async fn get_ocr_provider_credential_schema(
    provider_id: String,
    state: State<'_, crate::AppState>,
) -> Result<Vec<CredentialField>, String> {
    ocr_provider_credential_schema(&state.ocr_coordinator, &provider_id)
}

#[tauri::command]
pub async fn configure_ocr_provider_credentials(
    provider_id: String,
    credentials: HashMap<String, String>,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    configure_ocr_provider_credentials_inner(&provider_id, &credentials, state.inner())
}

fn ocr_provider_credential_schema(
    ocr: &OcrCoordinator,
    provider_id: &str,
) -> Result<Vec<CredentialField>, String> {
    let provider_lock = ocr
        .get(provider_id)
        .ok_or_else(|| format!("Provider not found: {}", provider_id))?;

    let fields = provider_lock.read().credential_fields();
    Ok(fields)
}

fn configure_ocr_provider_credentials_inner(
    provider_id: &str,
    credentials: &HashMap<String, String>,
    state: &crate::AppState,
) -> Result<(), String> {
    let provider_lock = state
        .ocr_coordinator
        .get(provider_id)
        .ok_or_else(|| format!("Provider not found: {}", provider_id))?;
    let expected_fields = provider_lock.read().credential_fields();

    if expected_fields.is_empty() {
        if credentials.is_empty() {
            return Ok(());
        }

        return Err(format!(
            "Provider {} does not accept credentials",
            provider_id
        ));
    }

    validate_required_credentials(&expected_fields, &credentials).map_err(|e| e.to_string())?;

    // Snapshot existing credentials for rollback
    let field_names: Vec<String> = expected_fields.iter().map(|f| f.name.clone()).collect();
    let snapshot = state
        .keychain
        .snapshot_provider_credentials(provider_id, &field_names);

    // Save credentials with transaction support
    state
        .keychain
        .save_provider_credentials_transactional(provider_id, credentials, &snapshot)
        .map_err(|e| e.to_string())?;

    // Reconfigure provider with complete rollback on failure
    if let Err(e) = state
        .ocr_coordinator
        .reconfigure_provider(provider_id, credentials)
    {
        // Rollback keychain changes
        let _ = state
            .keychain
            .restore_provider_credentials(provider_id, &snapshot);
        return Err(format!("Failed to reconfigure provider: {}", e));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::application::providers::ocr::impls::TesseractProvider;
    use crate::application::providers::ocr::OcrCoordinator;
    use crate::infrastructure::storage::ConfigFile;
    use std::sync::Arc;

    #[test]
    fn ocr_provider_credential_schema_returns_empty_fields_for_tesseract() {
        let coordinator = OcrCoordinator::new(Arc::new(ConfigFile::new_temp()));
        coordinator.register(TesseractProvider::new()).unwrap();

        let fields = super::ocr_provider_credential_schema(&coordinator, "tesseract").unwrap();

        assert!(fields.is_empty());
    }

    #[test]
    fn ocr_provider_credential_schema_reports_missing_ocr_provider() {
        let coordinator = OcrCoordinator::new(Arc::new(ConfigFile::new_temp()));

        let error = super::ocr_provider_credential_schema(&coordinator, "ghost").unwrap_err();

        assert_eq!(error, "Provider not found: ghost");
    }
}

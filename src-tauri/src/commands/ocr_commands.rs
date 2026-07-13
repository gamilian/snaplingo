use crate::application::providers::common::CredentialField;
use crate::domain::ocr::{OcrRequest, OcrResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::State;

#[cfg(test)]
use crate::application::providers::ocr::OcrCoordinator;

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
        .providers
        .ocr
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
        .providers
        .ocr
        .recognize(&ocr_request)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_ocr_providers(
    state: State<'_, crate::AppState>,
) -> Result<Vec<OcrProviderInfo>, String> {
    let all_providers = state.providers.ocr.list_all();
    let active = state.providers.ocr.get_active();
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
        .providers
        .ocr
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

    state
        .providers
        .ocr_configuration
        .save_credentials(&provider_id, &credentials)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_ocr_provider_credential_schema(
    provider_id: String,
    state: State<'_, crate::AppState>,
) -> Result<Vec<CredentialField>, String> {
    state
        .providers
        .ocr_configuration
        .credential_schema(&provider_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn configure_ocr_provider_credentials(
    provider_id: String,
    credentials: HashMap<String, String>,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state
        .providers
        .ocr_configuration
        .save_credentials(&provider_id, &credentials)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
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

#[cfg(test)]
mod tests {
    use crate::application::providers::ocr::impls::TesseractProvider;
    use crate::application::providers::ocr::{OcrCoordinator, TesseractEngine};
    use crate::infrastructure::storage::SqliteConfigStore;
    use std::sync::Arc;

    struct StubTesseractEngine;

    impl TesseractEngine for StubTesseractEngine {
        fn available_languages(&self) -> crate::Result<Vec<String>> {
            Ok(vec!["eng".to_string()])
        }

        fn recognize(&self, _image_data: &[u8], _language: Option<&str>) -> crate::Result<String> {
            Ok(String::new())
        }
    }

    #[test]
    fn ocr_provider_credential_schema_returns_empty_fields_for_tesseract() {
        let coordinator = OcrCoordinator::new(Arc::new(SqliteConfigStore::new_temp()));
        coordinator
            .register(TesseractProvider::new(Arc::new(StubTesseractEngine)))
            .unwrap();

        let fields = super::ocr_provider_credential_schema(&coordinator, "tesseract").unwrap();

        assert!(fields.is_empty());
    }

    #[test]
    fn ocr_provider_credential_schema_reports_missing_ocr_provider() {
        let coordinator = OcrCoordinator::new(Arc::new(SqliteConfigStore::new_temp()));

        let error = super::ocr_provider_credential_schema(&coordinator, "ghost").unwrap_err();

        assert_eq!(error, "Provider not found: ghost");
    }
}

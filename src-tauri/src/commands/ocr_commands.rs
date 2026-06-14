use crate::domain::ocr::{OcrRequest, OcrResult};
use tauri::State;
use serde::{Serialize, Deserialize};

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

    state.ocr_coordinator
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

    let info: Vec<_> = all_providers.iter().map(|p| {
        let is_active = active.as_ref().map_or(false, |active_p| {
            active_p.id() == p.id()
        });

        OcrProviderInfo {
            id: p.id().to_string(),
            name: p.name().to_string(),
            is_configured: p.is_configured(),
            requires_api_key: p.requires_api_key(),
            is_active,
        }
    }).collect();

    Ok(info)
}

#[tauri::command]
pub async fn activate_ocr_provider(
    provider_id: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state.ocr_coordinator
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
    // Build credentials map using new multi-field format
    let mut credentials = std::collections::HashMap::new();
    credentials.insert("api_key".to_string(), api_key);

    if let Some(secret) = secret_key {
        credentials.insert("secret_key".to_string(), secret);
    }

    // Save credentials using new format
    state.keychain
        .save_provider_credentials(&provider_id, &credentials)
        .map_err(|e| e.to_string())?;

    // Reconfigure provider at runtime
    state.ocr_coordinator
        .reconfigure_provider(&provider_id, &credentials)
        .map_err(|e| e.to_string())?;

    Ok(())
}

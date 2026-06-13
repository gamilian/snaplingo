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

    state.ocr_service
        .recognize(&ocr_request)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_ocr_providers(
    state: State<'_, crate::AppState>,
) -> Result<Vec<OcrProviderInfo>, String> {
    let registry = state.ocr_registry.lock().unwrap();
    let all_providers = registry.list_all();
    let active = registry.get_active();

    let info: Vec<_> = all_providers.iter().map(|p| OcrProviderInfo {
        id: p.id().to_string(),
        name: p.name().to_string(),
        is_configured: p.is_configured(),
        requires_api_key: p.requires_api_key(),
        is_active: active.as_ref().map_or(false, |active_p| active_p.id() == p.id()),
    }).collect();

    Ok(info)
}

#[tauri::command]
pub async fn activate_ocr_provider(
    provider_id: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state.ocr_registry
        .lock()
        .unwrap()
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
    // Save API key to keychain
    state.keychain
        .save_provider_credential(&provider_id, &api_key)
        .map_err(|e| e.to_string())?;

    // Save secret key to keychain if provided (for Baidu OCR)
    if let Some(secret) = secret_key {
        let secret_key_name = format!("{}_secret", provider_id);
        state.keychain
            .save_provider_credential(&secret_key_name, &secret)
            .map_err(|e| e.to_string())?;
    }

    // Note: Providers are recreated on startup with credentials from keychain
    Ok(())
}

use crate::application::providers::common::CredentialField;
use crate::application::providers::OcrProviderInfo;
use crate::domain::ocr::{OcrRequest, OcrResult};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::State;

#[derive(Serialize, Deserialize)]
pub struct RecognizeImageRequest {
    pub image_data: Vec<u8>,
    pub language: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecognizeImageFileResult {
    pub text: String,
    pub confidence: Option<f32>,
    pub image_data_url: String,
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
    language: Option<String>,
    state: State<'_, crate::AppState>,
) -> Result<RecognizeImageFileResult, String> {
    let image_data = std::fs::read(path).map_err(|e| e.to_string())?;
    let image_data_url = format!(
        "data:{};base64,{}",
        image_mime_type(&image_data),
        base64::engine::general_purpose::STANDARD.encode(&image_data)
    );
    let ocr_request = OcrRequest {
        image_data,
        language,
    };

    let result = state
        .providers
        .ocr
        .recognize(&ocr_request)
        .await
        .map_err(|e| e.to_string())?;

    Ok(RecognizeImageFileResult {
        text: result.text,
        confidence: result.confidence,
        image_data_url,
    })
}

fn image_mime_type(image_data: &[u8]) -> &'static str {
    match image::guess_format(image_data) {
        Ok(image::ImageFormat::Jpeg) => "image/jpeg",
        Ok(image::ImageFormat::WebP) => "image/webp",
        Ok(image::ImageFormat::Bmp) => "image/bmp",
        Ok(image::ImageFormat::Tiff) => "image/tiff",
        _ => "image/png",
    }
}

#[tauri::command]
pub async fn list_ocr_providers(
    state: State<'_, crate::AppState>,
) -> Result<Vec<OcrProviderInfo>, String> {
    Ok(state.providers.administration.list_ocr_providers())
}

#[tauri::command]
pub async fn activate_ocr_provider(
    provider_id: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state
        .providers
        .administration
        .activate_ocr_provider(provider_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn configure_ocr_provider(
    provider_id: String,
    api_key: String,
    secret_key: Option<String>,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state
        .providers
        .administration
        .configure_ocr_provider(provider_id, api_key, secret_key)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_ocr_provider_credential_schema(
    provider_id: String,
    state: State<'_, crate::AppState>,
) -> Result<Vec<CredentialField>, String> {
    state
        .providers
        .administration
        .ocr_credential_schema(provider_id)
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
        .administration
        .configure_ocr_provider_credentials(provider_id, credentials)
        .map_err(|e| e.to_string())
}

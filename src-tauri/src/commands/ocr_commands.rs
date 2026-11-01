use crate::application::providers::common::CredentialField;
use crate::application::providers::OcrProviderInfo;
use crate::domain::ocr::{OcrRequest, OcrResult};
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
    pub lines: Vec<crate::domain::ocr::OcrLine>,
    #[serde(rename = "detected_language")]
    pub detected_language: Option<String>,
    #[serde(rename = "provider_id")]
    pub provider_id: Option<String>,
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
    let output = state
        .providers
        .file_ocr
        .recognize(&path, language)
        .await
        .map_err(|error| error.to_string())?;
    let result = output.result;
    let image_data_url = output.image_data_url;

    Ok(RecognizeImageFileResult {
        text: result.text,
        confidence: result.confidence,
        image_data_url,
        lines: result.lines,
        detected_language: result.detected_language,
        provider_id: result.provider_id,
    })
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

#[cfg(test)]
mod tests {
    use super::RecognizeImageFileResult;

    #[test]
    fn file_result_keeps_ocr_metadata_names_consistent_with_ocr_result() {
        let value = serde_json::to_value(RecognizeImageFileResult {
            text: "bonjour".into(),
            confidence: Some(0.9),
            image_data_url: "data:image/png;base64,aW1hZ2U=".into(),
            lines: vec![],
            detected_language: Some("fr".into()),
            provider_id: Some("system-ocr".into()),
        })
        .unwrap();

        assert_eq!(value["imageDataUrl"], "data:image/png;base64,aW1hZ2U=");
        assert_eq!(value["detected_language"], "fr");
        assert_eq!(value["provider_id"], "system-ocr");
    }
}

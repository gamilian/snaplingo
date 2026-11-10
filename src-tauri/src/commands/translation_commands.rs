use crate::domain::translation::{TranslationRequest, TranslationResult};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Serialize, Deserialize)]
pub struct TranslateTextRequest {
    pub text: String,
    pub source_lang: Option<String>,
    pub target_lang: String,
}

#[tauri::command]
pub async fn translate_text_v2(
    request: TranslateTextRequest,
    state: State<'_, crate::AppState>,
) -> Result<Vec<TranslationResult>, String> {
    let translation_request = TranslationRequest {
        text: request.text,
        source_lang: request.source_lang.unwrap_or_else(|| "auto".to_string()),
        target_lang: request.target_lang,
    };

    state
        .providers
        .translation
        .translate(&translation_request)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn translate_text_with_provider(
    provider_id: String,
    request: TranslateTextRequest,
    state: State<'_, crate::AppState>,
) -> Result<TranslationResult, String> {
    let translation_request = TranslationRequest {
        text: request.text,
        source_lang: request.source_lang.unwrap_or_else(|| "auto".to_string()),
        target_lang: request.target_lang,
    };

    state
        .providers
        .translation
        .translate_with_provider(&provider_id, &translation_request)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn record_translation_history(
    request: TranslateTextRequest,
    results: Vec<TranslationResult>,
    duration_ms: u64,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    let translation_request = TranslationRequest {
        text: request.text,
        source_lang: request.source_lang.unwrap_or_else(|| "auto".to_string()),
        target_lang: request.target_lang,
    };

    state
        .history
        .history
        .record_translation(translation_request, results, duration_ms)
        .await
        .map_err(|error| error.to_string())
}

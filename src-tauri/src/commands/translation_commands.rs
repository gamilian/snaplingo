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
        .translation_coordinator
        .translate(&translation_request)
        .await
        .map_err(|e| e.to_string())
}

use crate::domain::translation::{TranslationRequest, TranslationResult};
use tauri::State;
use serde::{Serialize, Deserialize};

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
        provider: String::new(), // Not used by new service
    };

    state.translation_service
        .translate(&translation_request)
        .await
        .map_err(|e| e.to_string())
}

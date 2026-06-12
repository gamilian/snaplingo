use crate::translate::TranslationResult;
use crate::AppState;

#[tauri::command]
pub async fn translate_text(
    text: String,
    from: String,
    to: String,
    provider_ids: Vec<String>,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<TranslationResult>, String> {
    let mut results = Vec::new();

    for provider_id in provider_ids {
        if let Some(provider) = state.get_translation_provider(&provider_id) {
            match provider.translate(&text, &from, &to).await {
                Ok(result) => results.push(result),
                Err(e) => eprintln!("Provider {} failed: {}", provider_id, e),
            }
        }
    }

    if results.is_empty() {
        Err("All providers failed".to_string())
    } else {
        Ok(results)
    }
}

#[tauri::command]
pub fn detect_language(
    text: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    state.language_detector.detect(&text)
        .map_err(|e| e.to_string())
}

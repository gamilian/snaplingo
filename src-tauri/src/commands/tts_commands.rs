use tauri::State;

use crate::application::SystemTtsVoice;

#[tauri::command]
pub async fn list_system_tts_voices(
    state: State<'_, crate::AppState>,
) -> Result<Vec<SystemTtsVoice>, String> {
    state
        .tts
        .list_voices()
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn speak_text(
    text: String,
    language: Option<String>,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state
        .tts
        .speak(&text, language.as_deref())
        .await
        .map_err(|error| error.to_string())
}

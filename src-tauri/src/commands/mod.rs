mod capture_session_commands;
mod history_commands;
mod hotkey_commands;
mod ocr_commands;
mod pinned_image_commands;
mod provider_commands;
mod settings_commands;
mod translation_commands;

pub use capture_session_commands::*;
pub use history_commands::*;
pub use hotkey_commands::*;
pub use ocr_commands::*;
pub use pinned_image_commands::*;
pub use provider_commands::*;
pub use settings_commands::*;
pub use translation_commands::*;

use std::sync::{LazyLock, Mutex};

use serde::Serialize;
use tauri::{Emitter, State};

static CAPTURE_RESULT_WINDOW_PAYLOAD: LazyLock<Mutex<Option<CaptureResultWindowPayload>>> =
    LazyLock::new(|| Mutex::new(None));

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum CaptureResultWindowMode {
    Translation,
    Ocr,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum CaptureResultWindowOcrIntent {
    Show,
    DisplayText,
    File,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureResultWindowPayload {
    mode: CaptureResultWindowMode,
    text: String,
    auto_translate: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    ocr_intent: Option<CaptureResultWindowOcrIntent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    image_base64: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ResultWindowEntrypoint {
    ManualTranslation,
    AutoTranslation,
    InputTranslation,
    ShowTranslation,
    Ocr,
    ShowOcr,
    FileOcr,
}

#[tauri::command]
pub fn open_result_window(text: String, app: tauri::AppHandle) -> Result<(), String> {
    open_capture_result_window(
        result_window_payload_for_entrypoint(ResultWindowEntrypoint::ManualTranslation, text),
        app,
    )
}

pub fn open_input_translation_window(app: tauri::AppHandle) -> Result<(), String> {
    let text = read_clipboard_text().unwrap_or_default();
    open_capture_result_window(
        result_window_payload_for_entrypoint(ResultWindowEntrypoint::InputTranslation, text),
        app,
    )
}

#[tauri::command]
pub fn open_ocr_result_window(text: String, app: tauri::AppHandle) -> Result<(), String> {
    open_capture_result_window(
        result_window_payload_for_entrypoint(ResultWindowEntrypoint::Ocr, text),
        app,
    )
}

#[tauri::command]
pub fn open_capture_ocr_result_window(
    text: String,
    image_base64: Option<String>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    log::info!(
        "Opening capture OCR result window: text_chars={} has_image={}",
        text.chars().count(),
        image_base64.as_ref().is_some_and(|image| !image.is_empty())
    );
    open_capture_result_window(capture_ocr_result_payload(text, image_base64), app)
}

#[tauri::command]
pub fn open_capture_translation_result_window(
    text: String,
    app: tauri::AppHandle,
) -> Result<(), String> {
    open_capture_result_window(capture_translation_result_payload(text), app)
}

#[tauri::command]
pub fn take_capture_result_window_payload() -> Result<Option<CaptureResultWindowPayload>, String> {
    let mut payload = CAPTURE_RESULT_WINDOW_PAYLOAD
        .lock()
        .map_err(|_| "Capture result window payload lock poisoned".to_string())?;

    Ok(payload.take())
}

fn open_capture_result_window(
    payload: CaptureResultWindowPayload,
    app: tauri::AppHandle,
) -> Result<(), String> {
    {
        let mut pending_payload = CAPTURE_RESULT_WINDOW_PAYLOAD
            .lock()
            .map_err(|_| "Capture result window payload lock poisoned".to_string())?;
        *pending_payload = Some(payload);
    }

    let window = crate::infrastructure::system::result_window::show_or_create_result_window(&app)?;
    window
        .emit("capture-result-payload-ready", ())
        .map_err(|e| e.to_string())
}

fn capture_translation_result_payload(text: String) -> CaptureResultWindowPayload {
    translation_result_payload(text, true)
}

fn capture_ocr_result_payload(
    text: String,
    image_base64: Option<String>,
) -> CaptureResultWindowPayload {
    ocr_result_payload(
        text,
        CaptureResultWindowOcrIntent::DisplayText,
        image_base64,
    )
}

fn translation_result_payload(text: String, auto_translate: bool) -> CaptureResultWindowPayload {
    CaptureResultWindowPayload {
        mode: CaptureResultWindowMode::Translation,
        text,
        auto_translate,
        ocr_intent: None,
        image_base64: None,
    }
}

fn ocr_result_payload(
    text: String,
    ocr_intent: CaptureResultWindowOcrIntent,
    image_base64: Option<String>,
) -> CaptureResultWindowPayload {
    CaptureResultWindowPayload {
        mode: CaptureResultWindowMode::Ocr,
        text,
        auto_translate: false,
        ocr_intent: Some(ocr_intent),
        image_base64,
    }
}

fn result_window_payload_for_entrypoint(
    entrypoint: ResultWindowEntrypoint,
    text: String,
) -> CaptureResultWindowPayload {
    match entrypoint {
        ResultWindowEntrypoint::ManualTranslation => translation_result_payload(text, false),
        ResultWindowEntrypoint::AutoTranslation | ResultWindowEntrypoint::InputTranslation => {
            translation_result_payload(text, true)
        }
        ResultWindowEntrypoint::ShowTranslation => translation_result_payload(String::new(), false),
        ResultWindowEntrypoint::Ocr => {
            ocr_result_payload(text, CaptureResultWindowOcrIntent::DisplayText, None)
        }
        ResultWindowEntrypoint::ShowOcr => {
            ocr_result_payload(text, CaptureResultWindowOcrIntent::Show, None)
        }
        ResultWindowEntrypoint::FileOcr => {
            ocr_result_payload(String::new(), CaptureResultWindowOcrIntent::File, None)
        }
    }
}

fn read_clipboard_text() -> Result<String, String> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| format!("Failed to open clipboard: {}", e))?;
    clipboard
        .get_text()
        .map_err(|e| format!("Failed to read clipboard text: {}", e))
}

#[tauri::command]
pub fn copy_text_to_clipboard(text: String) -> Result<(), String> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| format!("Failed to open clipboard: {}", e))?;
    clipboard
        .set_text(text)
        .map_err(|e| format!("Failed to write text to clipboard: {}", e))
}

#[tauri::command]
pub fn open_translation_result_window(text: String, app: tauri::AppHandle) -> Result<(), String> {
    open_capture_result_window(
        result_window_payload_for_entrypoint(ResultWindowEntrypoint::AutoTranslation, text),
        app,
    )
}

pub fn show_translation_window(app: tauri::AppHandle) -> Result<(), String> {
    open_capture_result_window(
        result_window_payload_for_entrypoint(
            ResultWindowEntrypoint::ShowTranslation,
            String::new(),
        ),
        app,
    )
}

pub fn show_ocr_window(app: tauri::AppHandle) -> Result<(), String> {
    open_capture_result_window(
        result_window_payload_for_entrypoint(ResultWindowEntrypoint::ShowOcr, String::new()),
        app,
    )
}

pub fn start_file_ocr(app: tauri::AppHandle) -> Result<(), String> {
    open_capture_result_window(
        result_window_payload_for_entrypoint(ResultWindowEntrypoint::FileOcr, String::new()),
        app,
    )
}

pub async fn open_selection_translation_window_for_state(
    app: tauri::AppHandle,
    state: &crate::AppState,
) -> Result<(), String> {
    let snapshot = state
        .selection
        .acquirer
        .acquire()
        .await
        .map_err(|e| e.to_string())?;
    open_translation_result_window(snapshot.text, app)
}

#[tauri::command]
pub async fn open_selection_translation_window(
    app: tauri::AppHandle,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    open_selection_translation_window_for_state(app, state.inner()).await
}

#[tauri::command]
pub fn configure_hotkey(
    category: String,
    action: String,
    hotkey: String,
    app: tauri::AppHandle,
    state: State<'_, crate::AppState>,
) -> Result<Option<String>, String> {
    let outcome = state
        .settings
        .hotkeys
        .update_hotkey(&app, category, action, hotkey)
        .map_err(|e| e.to_string())?;
    Ok(outcome.accelerator)
}

#[tauri::command]
pub fn configure_translation_hotkey(
    action: String,
    hotkey: String,
    app: tauri::AppHandle,
    state: State<'_, crate::AppState>,
) -> Result<Option<String>, String> {
    let outcome = state
        .settings
        .hotkeys
        .update_hotkey(&app, "translation".to_string(), action, hotkey)
        .map_err(|e| e.to_string())?;
    Ok(outcome.accelerator)
}

#[tauri::command]
pub async fn trigger_screenshot(
    app: tauri::AppHandle,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    let _ = app;
    open_capture_window_for_mode(state.inner(), "screenshot").await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_translation_payload_requests_auto_translation() {
        let payload = capture_translation_result_payload("hello".to_string());

        assert_eq!(payload.mode, CaptureResultWindowMode::Translation);
        assert_eq!(payload.text, "hello");
        assert!(payload.auto_translate);
    }

    #[test]
    fn capture_ocr_payload_does_not_request_auto_translation() {
        let payload = capture_ocr_result_payload("hello".to_string(), None);

        assert_eq!(payload.mode, CaptureResultWindowMode::Ocr);
        assert_eq!(payload.text, "hello");
        assert!(!payload.auto_translate);
        assert_eq!(
            payload.ocr_intent,
            Some(CaptureResultWindowOcrIntent::DisplayText)
        );
        assert_eq!(payload.image_base64, None);
    }

    #[test]
    fn capture_ocr_payload_can_include_source_image() {
        let payload =
            capture_ocr_result_payload("hello".to_string(), Some("image-base64".to_string()));

        assert_eq!(payload.mode, CaptureResultWindowMode::Ocr);
        assert_eq!(payload.image_base64, Some("image-base64".to_string()));
    }

    #[test]
    fn result_entrypoints_use_dedicated_result_window_payloads() {
        assert_eq!(
            result_window_payload_for_entrypoint(
                ResultWindowEntrypoint::ManualTranslation,
                "hello".to_string(),
            ),
            CaptureResultWindowPayload {
                mode: CaptureResultWindowMode::Translation,
                text: "hello".to_string(),
                auto_translate: false,
                ocr_intent: None,
                image_base64: None,
            }
        );
        assert_eq!(
            result_window_payload_for_entrypoint(
                ResultWindowEntrypoint::AutoTranslation,
                "hello".to_string(),
            )
            .auto_translate,
            true
        );
        assert_eq!(
            result_window_payload_for_entrypoint(
                ResultWindowEntrypoint::InputTranslation,
                "clipboard text".to_string(),
            ),
            CaptureResultWindowPayload {
                mode: CaptureResultWindowMode::Translation,
                text: "clipboard text".to_string(),
                auto_translate: true,
                ocr_intent: None,
                image_base64: None,
            }
        );
        assert_eq!(
            result_window_payload_for_entrypoint(
                ResultWindowEntrypoint::ShowTranslation,
                String::new()
            ),
            CaptureResultWindowPayload {
                mode: CaptureResultWindowMode::Translation,
                text: String::new(),
                auto_translate: false,
                ocr_intent: None,
                image_base64: None,
            }
        );
        assert_eq!(
            result_window_payload_for_entrypoint(ResultWindowEntrypoint::Ocr, "hello".to_string()),
            CaptureResultWindowPayload {
                mode: CaptureResultWindowMode::Ocr,
                text: "hello".to_string(),
                auto_translate: false,
                ocr_intent: Some(CaptureResultWindowOcrIntent::DisplayText),
                image_base64: None,
            }
        );
        assert_eq!(
            result_window_payload_for_entrypoint(ResultWindowEntrypoint::ShowOcr, String::new()),
            CaptureResultWindowPayload {
                mode: CaptureResultWindowMode::Ocr,
                text: String::new(),
                auto_translate: false,
                ocr_intent: Some(CaptureResultWindowOcrIntent::Show),
                image_base64: None,
            }
        );
        assert_eq!(
            result_window_payload_for_entrypoint(ResultWindowEntrypoint::FileOcr, String::new()),
            CaptureResultWindowPayload {
                mode: CaptureResultWindowMode::Ocr,
                text: String::new(),
                auto_translate: false,
                ocr_intent: Some(CaptureResultWindowOcrIntent::File),
                image_base64: None,
            }
        );
    }
}

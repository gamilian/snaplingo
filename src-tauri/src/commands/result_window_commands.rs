use crate::application::result_window::{
    ResultWindowOpenRequest, ResultWindowPayload, ResultWindowRequestId, ResultWindowRuntime,
};

#[tauri::command]
pub async fn open_result_window(
    text: String,
    state: tauri::State<'_, crate::AppState>,
) -> Result<(), String> {
    open_result_window_for_runtime(&state.result_window, text).await
}

pub(crate) async fn open_result_window_for_runtime(
    runtime: &ResultWindowRuntime,
    text: String,
) -> Result<(), String> {
    open_result_window_request(runtime, ResultWindowOpenRequest::manual_translation(text)).await
}

#[tauri::command]
pub async fn open_ocr_result_window(
    text: String,
    state: tauri::State<'_, crate::AppState>,
) -> Result<(), String> {
    open_ocr_result_window_for_runtime(&state.result_window, text).await
}

pub(crate) async fn open_ocr_result_window_for_runtime(
    runtime: &ResultWindowRuntime,
    text: String,
) -> Result<(), String> {
    open_result_window_request(runtime, ResultWindowOpenRequest::display_ocr(text)).await
}

#[tauri::command]
pub async fn open_capture_ocr_result_window(
    text: String,
    image_base64: Option<String>,
    confidence: Option<f32>,
    state: tauri::State<'_, crate::AppState>,
) -> Result<(), String> {
    open_capture_ocr_result_window_for_runtime(&state.result_window, text, image_base64, confidence)
        .await
}

pub(crate) async fn open_capture_ocr_result_window_for_runtime(
    runtime: &ResultWindowRuntime,
    text: String,
    image_base64: Option<String>,
    confidence: Option<f32>,
) -> Result<(), String> {
    open_result_window_request(
        runtime,
        ResultWindowOpenRequest::capture_ocr(text, image_base64, confidence),
    )
    .await
}

#[tauri::command]
pub async fn open_capture_translation_result_window(
    text: String,
    state: tauri::State<'_, crate::AppState>,
) -> Result<(), String> {
    open_capture_translation_result_window_for_runtime(&state.result_window, text).await
}

pub(crate) async fn open_capture_translation_result_window_for_runtime(
    runtime: &ResultWindowRuntime,
    text: String,
) -> Result<(), String> {
    open_result_window_request(
        runtime,
        ResultWindowOpenRequest::screenshot_translation(text),
    )
    .await
}

#[tauri::command]
pub async fn open_translation_result_window(
    text: String,
    state: tauri::State<'_, crate::AppState>,
) -> Result<(), String> {
    open_translation_result_window_for_runtime(&state.result_window, text).await
}

pub(crate) async fn open_translation_result_window_for_runtime(
    runtime: &ResultWindowRuntime,
    text: String,
) -> Result<(), String> {
    open_result_window_request(
        runtime,
        ResultWindowOpenRequest::selection_translation(text),
    )
    .await
}

#[tauri::command]
pub fn current_capture_result_window_request_id(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Option<String>, String> {
    current_capture_result_window_request_id_for_runtime(&state.result_window)
}

pub(crate) fn current_capture_result_window_request_id_for_runtime(
    runtime: &ResultWindowRuntime,
) -> Result<Option<String>, String> {
    runtime
        .current_request_id()
        .map(|request_id| request_id.map(|request_id| request_id.0.to_string()))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn take_capture_result_window_payload(
    request_id: String,
    state: tauri::State<'_, crate::AppState>,
) -> Result<Option<ResultWindowPayload>, String> {
    take_capture_result_window_payload_for_runtime(&state.result_window, request_id)
}

pub(crate) fn take_capture_result_window_payload_for_runtime(
    runtime: &ResultWindowRuntime,
    request_id: String,
) -> Result<Option<ResultWindowPayload>, String> {
    runtime
        .take_if_current(parse_request_id(&request_id)?)
        .map_err(|error| error.to_string())
}

pub(crate) async fn open_selection_translation_window_for_state(
    state: &crate::AppState,
) -> Result<(), String> {
    let settings = state
        .settings
        .configuration
        .snapshot()
        .map_err(|error| error.to_string())?;
    let snapshot = state
        .selection
        .acquirer
        .acquire_with_mode(crate::application::SelectionTextMode::from_setting(
            &settings.translation.selection_text_mode,
        ))
        .await
        .map_err(|error| error.to_string())?;
    open_translation_result_window_for_runtime(&state.result_window, snapshot.text).await
}

#[tauri::command]
pub async fn open_selection_translation_window(
    state: tauri::State<'_, crate::AppState>,
) -> Result<(), String> {
    open_selection_translation_window_for_state(state.inner()).await
}

async fn open_result_window_request(
    runtime: &ResultWindowRuntime,
    request: ResultWindowOpenRequest,
) -> Result<(), String> {
    runtime
        .open(request)
        .await
        .map_err(|error| error.to_string())
}

fn parse_request_id(request_id: &str) -> Result<ResultWindowRequestId, String> {
    if request_id.is_empty() || !request_id.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(
            "Invalid result window request ID: expected an unsigned decimal integer".into(),
        );
    }

    request_id.parse().map(ResultWindowRequestId).map_err(|_| {
        "Invalid result window request ID: expected an unsigned decimal integer".into()
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;

    use crate::application::result_window::{
        ResultWindowMode, ResultWindowNotifierPort, ResultWindowOcrIntent, ResultWindowRequestId,
        ResultWindowRuntime, ResultWindowWindowPort,
    };

    struct Window;

    #[async_trait]
    impl ResultWindowWindowPort for Window {
        async fn show_or_create(&self) -> crate::Result<()> {
            Ok(())
        }
    }

    struct Notifier;

    #[async_trait]
    impl ResultWindowNotifierPort for Notifier {
        async fn notify_payload_ready(&self, _: ResultWindowRequestId) -> crate::Result<()> {
            Ok(())
        }
    }

    fn runtime() -> ResultWindowRuntime {
        ResultWindowRuntime::new(Arc::new(Window), Arc::new(Notifier))
    }

    #[tokio::test]
    async fn commands_delegate_requests_to_the_result_window_runtime() {
        let runtime = runtime();

        super::open_capture_translation_result_window_for_runtime(&runtime, "translated".into())
            .await
            .unwrap();

        let request_id = super::current_capture_result_window_request_id_for_runtime(&runtime)
            .unwrap()
            .unwrap();
        let payload = super::take_capture_result_window_payload_for_runtime(&runtime, request_id)
            .unwrap()
            .unwrap();

        assert_eq!(payload.mode, ResultWindowMode::Translation);
        assert_eq!(payload.text, "translated");
        assert!(payload.auto_translate);
    }

    #[tokio::test]
    async fn command_payload_mapping_preserves_ocr_image_and_intent() {
        let runtime = runtime();

        super::open_capture_ocr_result_window_for_runtime(
            &runtime,
            "recognized".into(),
            Some("image-base64".into()),
            Some(0.94),
        )
        .await
        .unwrap();

        let request_id = super::current_capture_result_window_request_id_for_runtime(&runtime)
            .unwrap()
            .unwrap();
        let payload = super::take_capture_result_window_payload_for_runtime(&runtime, request_id)
            .unwrap()
            .unwrap();

        assert_eq!(payload.mode, ResultWindowMode::Ocr);
        assert_eq!(payload.text, "recognized");
        assert!(!payload.auto_translate);
        assert_eq!(payload.ocr_intent, Some(ResultWindowOcrIntent::DisplayText));
        assert_eq!(payload.image_base64.as_deref(), Some("image-base64"));
        assert_eq!(
            serde_json::to_value(payload).unwrap(),
            serde_json::json!({
                "mode": "ocr",
                "origin": "ocr",
                "text": "recognized",
                "autoTranslate": false,
                "ocrIntent": "display-text",
                "imageBase64": "image-base64",
                "confidence": 0.94_f32,
            })
        );
    }

    #[tokio::test]
    async fn bootstrap_request_id_can_take_the_current_payload_but_not_a_stale_one() {
        let runtime = runtime();

        super::open_result_window_for_runtime(&runtime, "older".into())
            .await
            .unwrap();
        let stale_id = super::current_capture_result_window_request_id_for_runtime(&runtime)
            .unwrap()
            .unwrap();

        super::open_result_window_for_runtime(&runtime, "newer".into())
            .await
            .unwrap();

        assert_eq!(
            super::take_capture_result_window_payload_for_runtime(&runtime, stale_id).unwrap(),
            None
        );

        let current_id = super::current_capture_result_window_request_id_for_runtime(&runtime)
            .unwrap()
            .unwrap();
        assert_eq!(
            super::take_capture_result_window_payload_for_runtime(&runtime, current_id)
                .unwrap()
                .unwrap()
                .text,
            "newer"
        );
    }

    #[test]
    fn malformed_or_out_of_range_request_ids_do_not_take_a_payload() {
        let runtime = runtime();

        for request_id in ["not-a-number", "18446744073709551616"] {
            assert_eq!(
                super::take_capture_result_window_payload_for_runtime(&runtime, request_id.into())
                    .unwrap_err(),
                "Invalid result window request ID: expected an unsigned decimal integer"
            );
        }
    }
}

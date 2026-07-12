use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::application::result_window::{
    ResultWindowNotifierPort, ResultWindowRequestId, ResultWindowWindowPort,
};
use crate::Result;

use super::backend::RESULT_WINDOW_LABEL;
use super::show_or_create_result_window;

pub(super) const RESULT_WINDOW_PAYLOAD_READY_EVENT: &str = "capture-result-payload-ready";

pub(crate) struct TauriResultWindowRuntimeHost {
    app: AppHandle,
}

impl TauriResultWindowRuntimeHost {
    pub(crate) fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

#[async_trait::async_trait]
impl ResultWindowWindowPort for TauriResultWindowRuntimeHost {
    async fn show_or_create(&self) -> Result<()> {
        run_on_main_thread(&self.app, "show result window", |app| {
            show_or_create_result_window(&app).map(|_| ())
        })
        .await
    }
}

pub(crate) struct TauriResultWindowNotifier {
    app: AppHandle,
}

impl TauriResultWindowNotifier {
    pub(crate) fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

#[async_trait::async_trait]
impl ResultWindowNotifierPort for TauriResultWindowNotifier {
    async fn notify_payload_ready(&self, request_id: ResultWindowRequestId) -> Result<()> {
        run_on_main_thread(
            &self.app,
            "notify result window payload readiness",
            move |app| {
                let window = app
                    .get_webview_window(RESULT_WINDOW_LABEL)
                    .ok_or("Result window is not available")?;
                window
                    .emit(
                        RESULT_WINDOW_PAYLOAD_READY_EVENT,
                        payload_ready_event(request_id),
                    )
                    .map_err(|error| error.to_string())
            },
        )
        .await
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ResultWindowPayloadReadyEvent {
    request_id: u64,
}

fn payload_ready_event(request_id: ResultWindowRequestId) -> ResultWindowPayloadReadyEvent {
    ResultWindowPayloadReadyEvent {
        request_id: request_id.0,
    }
}

async fn run_on_main_thread<T, F>(
    app: &AppHandle,
    operation_name: &'static str,
    operation: F,
) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce(AppHandle) -> std::result::Result<T, String> + Send + 'static,
{
    let (sender, receiver) = tokio::sync::oneshot::channel();
    app.run_on_main_thread({
        let app = app.clone();
        move || {
            let _ = sender.send(operation(app));
        }
    })
    .map_err(|error| format!("Failed to dispatch {operation_name}: {error}"))?;

    receiver
        .await
        .map_err(|error| format!("Failed to receive {operation_name} result: {error}"))?
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use crate::application::result_window::ResultWindowRequestId;

    #[test]
    fn payload_ready_notification_uses_existing_event_and_serializes_request_id() {
        assert_eq!(
            super::RESULT_WINDOW_PAYLOAD_READY_EVENT,
            "capture-result-payload-ready"
        );
        assert_eq!(
            serde_json::to_value(super::payload_ready_event(ResultWindowRequestId(42))).unwrap(),
            serde_json::json!({ "requestId": 42 })
        );
    }
}

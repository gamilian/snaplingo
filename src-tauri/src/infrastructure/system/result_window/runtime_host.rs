use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::application::result_window::{
    ResultWindowNotifierPort, ResultWindowRequestId, ResultWindowWindowPort,
};
use crate::Result;

use super::backend::RESULT_WINDOW_LABEL;
use super::tauri::show_or_create_result_window_without_context;

pub(super) const RESULT_WINDOW_PAYLOAD_READY_EVENT: &str = "capture-result-payload-ready";

pub(crate) struct TauriResultWindowRuntimeHost {
    platform: Arc<dyn ResultWindowTauriPlatform>,
}

impl TauriResultWindowRuntimeHost {
    pub(crate) fn new(app: AppHandle) -> Self {
        Self::with_platform(Arc::new(TauriResultWindowPlatform { app }))
    }

    fn with_platform(platform: Arc<dyn ResultWindowTauriPlatform>) -> Self {
        Self { platform }
    }
}

#[async_trait::async_trait]
impl ResultWindowWindowPort for TauriResultWindowRuntimeHost {
    async fn show_or_create(&self) -> Result<()> {
        self.platform
            .show_or_create_on_main_thread()
            .await
            .map_err(|error| format!("Failed to show result window: {error}").into())
    }
}

pub(crate) struct TauriResultWindowNotifier {
    platform: Arc<dyn ResultWindowTauriPlatform>,
}

impl TauriResultWindowNotifier {
    pub(crate) fn new(app: AppHandle) -> Self {
        Self::with_platform(Arc::new(TauriResultWindowPlatform { app }))
    }

    fn with_platform(platform: Arc<dyn ResultWindowTauriPlatform>) -> Self {
        Self { platform }
    }
}

#[async_trait::async_trait]
impl ResultWindowNotifierPort for TauriResultWindowNotifier {
    async fn notify_payload_ready(&self, request_id: ResultWindowRequestId) -> Result<()> {
        self.platform
            .emit_to_result_window_on_main_thread(
                RESULT_WINDOW_LABEL,
                RESULT_WINDOW_PAYLOAD_READY_EVENT,
                payload_ready_event(request_id),
            )
            .await
            .map_err(|error| {
                format!("Failed to notify result window payload readiness: {error}").into()
            })
    }
}

/// Event-boundary DTO: request IDs are decimal strings so JavaScript never loses u64 precision.
/// Commands receiving this value must parse it before constructing `ResultWindowRequestId`.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ResultWindowPayloadReadyEvent {
    request_id: String,
}

fn payload_ready_event(request_id: ResultWindowRequestId) -> ResultWindowPayloadReadyEvent {
    ResultWindowPayloadReadyEvent {
        request_id: request_id.0.to_string(),
    }
}

#[async_trait::async_trait]
trait ResultWindowTauriPlatform: Send + Sync {
    async fn show_or_create_on_main_thread(&self) -> std::result::Result<(), String>;
    async fn emit_to_result_window_on_main_thread(
        &self,
        target: &str,
        event: &str,
        payload: ResultWindowPayloadReadyEvent,
    ) -> std::result::Result<(), String>;
}

struct TauriResultWindowPlatform {
    app: AppHandle,
}

#[async_trait::async_trait]
impl ResultWindowTauriPlatform for TauriResultWindowPlatform {
    async fn show_or_create_on_main_thread(&self) -> std::result::Result<(), String> {
        run_on_main_thread(&self.app, "show result window", |app| {
            show_or_create_result_window_without_context(&app).map(|_| ())
        })
        .await
        .map_err(|error| error.to_string())
    }

    async fn emit_to_result_window_on_main_thread(
        &self,
        target: &str,
        event: &str,
        payload: ResultWindowPayloadReadyEvent,
    ) -> std::result::Result<(), String> {
        let target = target.to_string();
        let event = event.to_string();
        run_on_main_thread(
            &self.app,
            "notify result window payload readiness",
            move |app| {
                let window = app
                    .get_webview_window(&target)
                    .ok_or("Result window is not available")?;
                window
                    .emit(&event, payload)
                    .map_err(|error| error.to_string())
            },
        )
        .await
        .map_err(|error| error.to_string())
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
    use std::sync::{Arc, Mutex};

    use crate::application::result_window::ResultWindowRequestId;
    use crate::application::result_window::{ResultWindowNotifierPort, ResultWindowWindowPort};

    struct FakeTauriPlatform {
        show_error: Option<&'static str>,
        notify_error: Option<&'static str>,
        show_dispatches: Mutex<u8>,
        notifications: Mutex<Vec<(String, String, serde_json::Value)>>,
    }

    #[async_trait::async_trait]
    impl super::ResultWindowTauriPlatform for FakeTauriPlatform {
        async fn show_or_create_on_main_thread(&self) -> Result<(), String> {
            *self.show_dispatches.lock().unwrap() += 1;
            self.show_error
                .map_or(Ok(()), |error| Err(error.to_string()))
        }

        async fn emit_to_result_window_on_main_thread(
            &self,
            target: &str,
            event: &str,
            payload: super::ResultWindowPayloadReadyEvent,
        ) -> Result<(), String> {
            self.notifications.lock().unwrap().push((
                target.to_string(),
                event.to_string(),
                serde_json::to_value(payload).unwrap(),
            ));
            self.notify_error
                .map_or(Ok(()), |error| Err(error.to_string()))
        }
    }

    fn platform(
        show_error: Option<&'static str>,
        notify_error: Option<&'static str>,
    ) -> Arc<FakeTauriPlatform> {
        Arc::new(FakeTauriPlatform {
            show_error,
            notify_error,
            show_dispatches: Mutex::new(0),
            notifications: Mutex::new(Vec::new()),
        })
    }

    #[test]
    fn payload_ready_notification_uses_existing_event_and_serializes_request_id() {
        assert_eq!(
            super::RESULT_WINDOW_PAYLOAD_READY_EVENT,
            "capture-result-payload-ready"
        );
        assert_eq!(
            serde_json::to_value(super::payload_ready_event(ResultWindowRequestId(42))).unwrap(),
            serde_json::json!({ "requestId": "42" })
        );
    }

    #[test]
    fn payload_ready_notification_serializes_all_request_ids_as_exact_decimal_strings() {
        for request_id in [
            ResultWindowRequestId(9_007_199_254_740_991),
            ResultWindowRequestId(9_007_199_254_740_992),
            ResultWindowRequestId(u64::MAX),
        ] {
            assert_eq!(
                serde_json::to_value(super::payload_ready_event(request_id)).unwrap(),
                serde_json::json!({ "requestId": request_id.0.to_string() })
            );
        }
    }

    #[tokio::test]
    async fn host_dispatches_show_create_through_the_main_thread_platform() {
        let platform = platform(None, None);
        let host = super::TauriResultWindowRuntimeHost::with_platform(platform.clone());

        host.show_or_create().await.unwrap();

        assert_eq!(*platform.show_dispatches.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn notifier_emits_to_the_result_window_with_the_exact_event_and_safe_request_id() {
        let platform = platform(None, None);
        let notifier = super::TauriResultWindowNotifier::with_platform(platform.clone());

        notifier
            .notify_payload_ready(ResultWindowRequestId(u64::MAX))
            .await
            .unwrap();

        assert_eq!(
            *platform.notifications.lock().unwrap(),
            vec![(
                "capture-result".to_string(),
                "capture-result-payload-ready".to_string(),
                serde_json::json!({ "requestId": u64::MAX.to_string() }),
            )]
        );
    }

    #[tokio::test]
    async fn host_and_notifier_add_operation_context_to_platform_failures() {
        let host = super::TauriResultWindowRuntimeHost::with_platform(platform(
            Some("create failed"),
            None,
        ));
        assert_eq!(
            host.show_or_create().await.unwrap_err().to_string(),
            "Failed to show result window: create failed"
        );

        let notifier =
            super::TauriResultWindowNotifier::with_platform(platform(None, Some("emit failed")));
        assert_eq!(
            notifier
                .notify_payload_ready(ResultWindowRequestId(1))
                .await
                .unwrap_err()
                .to_string(),
            "Failed to notify result window payload readiness: emit failed"
        );
    }

    #[tokio::test]
    async fn notifier_reports_when_the_result_window_closes_before_notification() {
        let notifier = super::TauriResultWindowNotifier::with_platform(platform(
            None,
            Some("Result window is not available"),
        ));

        assert_eq!(
            notifier
                .notify_payload_ready(ResultWindowRequestId(1))
                .await
                .unwrap_err()
                .to_string(),
            "Failed to notify result window payload readiness: Result window is not available"
        );
    }
}

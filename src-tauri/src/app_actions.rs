use crate::application::result_window::{ResultWindowOpenRequest, ResultWindowRuntime};
use crate::{commands, settings_window, AppState};
use tauri::Manager;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CaptureLaunchMode {
    Screenshot,
    ScreenshotCopy,
    ScreenshotTranslate,
    ScreenshotOcr,
    SilentScreenshotOcr,
}

impl CaptureLaunchMode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Screenshot => "screenshot",
            Self::ScreenshotCopy => "screenshot-copy",
            Self::ScreenshotTranslate => "screenshot-translate",
            Self::ScreenshotOcr => "screenshot-ocr",
            Self::SilentScreenshotOcr => "silent-screenshot-ocr",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AppAction {
    OpenCapture(CaptureLaunchMode),
    TranslateSelection,
    OpenInputTranslation,
    OpenTranslationWindow,
    RunFileOcr,
    OpenOcrWindow,
    PinClipboardImage,
    TogglePinnedImagesVisibility,
    SwitchPinnedImageGroup,
    OpenSettings,
    OpenAbout,
    Quit,
}

pub(crate) fn dispatch_app_action(app: tauri::AppHandle, action: AppAction) {
    match action {
        AppAction::OpenCapture(mode) => {
            tauri::async_runtime::spawn(commands::open_capture_window_from_shortcut(
                app,
                mode.as_str(),
            ));
        }
        AppAction::TranslateSelection => {
            tauri::async_runtime::spawn(async move {
                let state = app.state::<AppState>();
                let snapshot = match state.selection.acquirer.acquire().await {
                    Ok(snapshot) => snapshot,
                    Err(err) => {
                        log::error!("Failed to acquire selected text: {}", err);
                        return;
                    }
                };
                if let Err(err) = open_result_window_request(
                    &state.result_window,
                    ResultWindowOpenRequest::automatic_translation(snapshot.text),
                )
                .await
                {
                    log::error!("Failed to open selection translation window: {}", err);
                }
            });
        }
        AppAction::OpenInputTranslation => {
            dispatch_result_window_open(app, ResultWindowOpenRequest::input_translation());
        }
        AppAction::OpenTranslationWindow => {
            dispatch_result_window_open(app, ResultWindowOpenRequest::show_translation());
        }
        AppAction::RunFileOcr => {
            dispatch_result_window_open(app, ResultWindowOpenRequest::file_ocr());
        }
        AppAction::OpenOcrWindow => {
            dispatch_result_window_open(app, ResultWindowOpenRequest::show_ocr());
        }
        AppAction::PinClipboardImage => {
            tauri::async_runtime::spawn(async move {
                let state = app.state::<AppState>();
                if let Err(err) = commands::pin_clipboard_image_for_state(state.inner()).await {
                    log::error!("Failed to pin clipboard image: {}", err);
                }
            });
        }
        AppAction::TogglePinnedImagesVisibility => {
            tauri::async_runtime::spawn(async move {
                let state = app.state::<AppState>();
                if let Err(err) =
                    commands::toggle_pinned_images_visibility_for_state(state.inner()).await
                {
                    log::error!("Failed to toggle pinned images: {}", err);
                }
            });
        }
        AppAction::SwitchPinnedImageGroup => {
            tauri::async_runtime::spawn(async move {
                let state = app.state::<AppState>();
                if let Err(err) = commands::switch_pinned_image_group_for_state(state.inner()).await
                {
                    log::error!("Failed to switch pinned image group: {}", err);
                }
            });
        }
        AppAction::OpenSettings | AppAction::OpenAbout => {
            if let Err(err) = settings_window::show_settings_window(&app) {
                log::error!("Failed to show settings window: {}", err);
            }
        }
        AppAction::Quit => {
            app.exit(0);
        }
    }
}

fn dispatch_result_window_open(app: tauri::AppHandle, request: ResultWindowOpenRequest) {
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        if let Err(err) = open_result_window_request(&state.result_window, request).await {
            log::error!("Failed to open result window: {}", err);
        }
    });
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;

    use crate::application::result_window::{
        ResultWindowClipboardPort, ResultWindowMode, ResultWindowNotifierPort,
        ResultWindowOpenRequest, ResultWindowRequestId, ResultWindowRuntime,
        ResultWindowWindowPort,
    };

    use super::{open_result_window_request, CaptureLaunchMode};

    #[test]
    fn capture_launch_modes_keep_existing_ipc_strings() {
        assert_eq!(CaptureLaunchMode::Screenshot.as_str(), "screenshot");
        assert_eq!(
            CaptureLaunchMode::ScreenshotCopy.as_str(),
            "screenshot-copy"
        );
        assert_eq!(
            CaptureLaunchMode::ScreenshotTranslate.as_str(),
            "screenshot-translate"
        );
        assert_eq!(CaptureLaunchMode::ScreenshotOcr.as_str(), "screenshot-ocr");
        assert_eq!(
            CaptureLaunchMode::SilentScreenshotOcr.as_str(),
            "silent-screenshot-ocr"
        );
    }

    struct Window;

    #[async_trait]
    impl ResultWindowWindowPort for Window {
        async fn show_or_create(&self) -> crate::Result<()> {
            Ok(())
        }
    }

    struct Clipboard;

    #[async_trait]
    impl ResultWindowClipboardPort for Clipboard {
        async fn read_text(&self) -> crate::Result<String> {
            Ok(String::new())
        }
    }

    struct Notifier;

    #[async_trait]
    impl ResultWindowNotifierPort for Notifier {
        async fn notify_payload_ready(&self, _: ResultWindowRequestId) -> crate::Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn result_window_actions_delegate_open_requests_to_the_runtime() {
        let runtime =
            ResultWindowRuntime::new(Arc::new(Window), Arc::new(Clipboard), Arc::new(Notifier));

        open_result_window_request(
            &runtime,
            ResultWindowOpenRequest::automatic_translation("selection".into()),
        )
        .await
        .unwrap();

        let request_id = runtime.current_request_id().unwrap().unwrap();
        let payload = runtime.take_if_current(request_id).unwrap().unwrap();
        assert_eq!(payload.mode, ResultWindowMode::Translation);
        assert_eq!(payload.text, "selection");
        assert!(payload.auto_translate);
    }
}

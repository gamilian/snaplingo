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
                if let Err(err) = commands::open_selection_translation_window_for_state(
                    app.clone(),
                    state.inner(),
                )
                .await
                {
                    log::error!("Failed to open selection translation window: {}", err);
                }
            });
        }
        AppAction::OpenInputTranslation => {
            if let Err(err) = commands::open_input_translation_window(app) {
                log::error!("Failed to open input translation window: {}", err);
            }
        }
        AppAction::OpenTranslationWindow => {
            if let Err(err) = commands::show_translation_window(app) {
                log::error!("Failed to show translation window: {}", err);
            }
        }
        AppAction::RunFileOcr => {
            if let Err(err) = commands::start_file_ocr(app) {
                log::error!("Failed to start file OCR: {}", err);
            }
        }
        AppAction::OpenOcrWindow => {
            if let Err(err) = commands::show_ocr_window(app) {
                log::error!("Failed to show OCR window: {}", err);
            }
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

#[cfg(test)]
mod tests {
    use super::CaptureLaunchMode;

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
}

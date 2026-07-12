use crate::app_actions::{dispatch_app_action, AppAction, CaptureLaunchMode};
use crate::domain::hotkey_config::{
    FILE_OCR_ACTION, INPUT_TRANSLATE_ACTION, OCR_CATEGORY, PIN_ACTION, PIN_SWITCH_GROUP_ACTION,
    PIN_TOGGLE_ALL_ACTION, SCREENSHOT_ACTION, SCREENSHOT_CATEGORY, SCREENSHOT_COPY_ACTION,
    SCREENSHOT_CUSTOM_ACTION, SCREENSHOT_OCR_ACTION, SCREENSHOT_TRANSLATE_ACTION,
    SELECTION_TRANSLATE_ACTION, SHOW_OCR_WINDOW_ACTION, SHOW_TRANSLATION_WINDOW_ACTION,
    SILENT_SCREENSHOT_OCR_ACTION, TRANSLATION_CATEGORY,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct HotkeyActionBinding {
    pub action: AppAction,
}

pub(crate) fn hotkey_action_binding(
    category: &str,
    action_key: &str,
) -> Option<HotkeyActionBinding> {
    let action = match (category, action_key) {
        (SCREENSHOT_CATEGORY, SCREENSHOT_ACTION | SCREENSHOT_CUSTOM_ACTION) => {
            AppAction::OpenCapture(CaptureLaunchMode::Screenshot)
        }
        (SCREENSHOT_CATEGORY, SCREENSHOT_COPY_ACTION) => {
            AppAction::OpenCapture(CaptureLaunchMode::ScreenshotCopy)
        }
        (SCREENSHOT_CATEGORY, PIN_ACTION) => AppAction::PinClipboardImage,
        (SCREENSHOT_CATEGORY, PIN_TOGGLE_ALL_ACTION) => AppAction::TogglePinnedImagesVisibility,
        (SCREENSHOT_CATEGORY, PIN_SWITCH_GROUP_ACTION) => AppAction::SwitchPinnedImageGroup,
        (TRANSLATION_CATEGORY, SCREENSHOT_TRANSLATE_ACTION) => {
            AppAction::OpenCapture(CaptureLaunchMode::ScreenshotTranslate)
        }
        (TRANSLATION_CATEGORY, SELECTION_TRANSLATE_ACTION) => AppAction::TranslateSelection,
        (TRANSLATION_CATEGORY, INPUT_TRANSLATE_ACTION) => AppAction::OpenInputTranslation,
        (TRANSLATION_CATEGORY, SHOW_TRANSLATION_WINDOW_ACTION) => AppAction::OpenTranslationWindow,
        (OCR_CATEGORY, SCREENSHOT_OCR_ACTION) => {
            AppAction::OpenCapture(CaptureLaunchMode::ScreenshotOcr)
        }
        (OCR_CATEGORY, SILENT_SCREENSHOT_OCR_ACTION) => {
            AppAction::OpenCapture(CaptureLaunchMode::SilentScreenshotOcr)
        }
        (OCR_CATEGORY, FILE_OCR_ACTION) => AppAction::RunFileOcr,
        (OCR_CATEGORY, SHOW_OCR_WINDOW_ACTION) => AppAction::OpenOcrWindow,
        _ => return None,
    };

    Some(HotkeyActionBinding { action })
}

pub(crate) fn trigger_hotkey_action(app: tauri::AppHandle, category: String, action_key: String) {
    let Some(binding) = hotkey_action_binding(&category, &action_key) else {
        log::warn!("Unknown hotkey action: {}:{}", category, action_key);
        return;
    };

    dispatch_app_action(app, binding.action);
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::hotkey_action_binding;
    use crate::app_actions::{AppAction, CaptureLaunchMode};
    use crate::domain::hotkey_config::{
        DEFAULT_HOTKEYS, FILE_OCR_ACTION, INPUT_TRANSLATE_ACTION, OCR_CATEGORY, PIN_ACTION,
        PIN_SWITCH_GROUP_ACTION, PIN_TOGGLE_ALL_ACTION, SCREENSHOT_ACTION, SCREENSHOT_CATEGORY,
        SCREENSHOT_COPY_ACTION, SCREENSHOT_CUSTOM_ACTION, SCREENSHOT_OCR_ACTION,
        SCREENSHOT_TRANSLATE_ACTION, SELECTION_TRANSLATE_ACTION, SHOW_OCR_WINDOW_ACTION,
        SHOW_TRANSLATION_WINDOW_ACTION, SILENT_SCREENSHOT_OCR_ACTION, TRANSLATION_CATEGORY,
    };

    #[test]
    fn maps_hotkey_keys_to_app_actions() {
        let cases = [
            (
                SCREENSHOT_CATEGORY,
                SCREENSHOT_ACTION,
                AppAction::OpenCapture(CaptureLaunchMode::Screenshot),
            ),
            (
                SCREENSHOT_CATEGORY,
                SCREENSHOT_COPY_ACTION,
                AppAction::OpenCapture(CaptureLaunchMode::ScreenshotCopy),
            ),
            (
                SCREENSHOT_CATEGORY,
                SCREENSHOT_CUSTOM_ACTION,
                AppAction::OpenCapture(CaptureLaunchMode::Screenshot),
            ),
            (
                SCREENSHOT_CATEGORY,
                PIN_ACTION,
                AppAction::PinClipboardImage,
            ),
            (
                SCREENSHOT_CATEGORY,
                PIN_TOGGLE_ALL_ACTION,
                AppAction::TogglePinnedImagesVisibility,
            ),
            (
                SCREENSHOT_CATEGORY,
                PIN_SWITCH_GROUP_ACTION,
                AppAction::SwitchPinnedImageGroup,
            ),
            (
                TRANSLATION_CATEGORY,
                SELECTION_TRANSLATE_ACTION,
                AppAction::TranslateSelection,
            ),
            (
                TRANSLATION_CATEGORY,
                SCREENSHOT_TRANSLATE_ACTION,
                AppAction::OpenCapture(CaptureLaunchMode::ScreenshotTranslate),
            ),
            (
                TRANSLATION_CATEGORY,
                INPUT_TRANSLATE_ACTION,
                AppAction::OpenInputTranslation,
            ),
            (
                TRANSLATION_CATEGORY,
                SHOW_TRANSLATION_WINDOW_ACTION,
                AppAction::OpenTranslationWindow,
            ),
            (
                OCR_CATEGORY,
                SCREENSHOT_OCR_ACTION,
                AppAction::OpenCapture(CaptureLaunchMode::ScreenshotOcr),
            ),
            (
                OCR_CATEGORY,
                SILENT_SCREENSHOT_OCR_ACTION,
                AppAction::OpenCapture(CaptureLaunchMode::SilentScreenshotOcr),
            ),
            (OCR_CATEGORY, FILE_OCR_ACTION, AppAction::RunFileOcr),
            (
                OCR_CATEGORY,
                SHOW_OCR_WINDOW_ACTION,
                AppAction::OpenOcrWindow,
            ),
        ];

        for (category, action_key, expected_action) in cases {
            let binding = hotkey_action_binding(category, action_key).unwrap();

            assert_eq!(binding.action, expected_action);
        }

        let covered: HashSet<_> = cases
            .iter()
            .map(|(category, action, _)| (*category, *action))
            .collect();
        let defaults: HashSet<_> = DEFAULT_HOTKEYS
            .iter()
            .map(|hotkey| (hotkey.category, hotkey.action))
            .collect();

        assert_eq!(cases.len(), 14);
        assert_eq!(cases.len(), DEFAULT_HOTKEYS.len());
        assert_eq!(covered, defaults);
        assert!(hotkey_action_binding("unknown", "unknown").is_none());
    }
}

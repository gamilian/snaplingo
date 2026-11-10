use crate::shell::actions::AppAction;
use crate::shell::actions::CaptureLaunchMode;
use snaplingo_core::hotkeys::model::FILE_OCR_ACTION;
use snaplingo_core::hotkeys::model::OCR_CATEGORY;
use snaplingo_core::hotkeys::model::PIN_ACTION;
use snaplingo_core::hotkeys::model::PIN_SWITCH_GROUP_ACTION;
use snaplingo_core::hotkeys::model::PIN_TOGGLE_ALL_ACTION;
use snaplingo_core::hotkeys::model::SCREENSHOT_ACTION;
use snaplingo_core::hotkeys::model::SCREENSHOT_CATEGORY;
use snaplingo_core::hotkeys::model::SCREENSHOT_COPY_ACTION;
use snaplingo_core::hotkeys::model::SCREENSHOT_OCR_ACTION;
use snaplingo_core::hotkeys::model::SCREENSHOT_TRANSLATE_ACTION;
use snaplingo_core::hotkeys::model::SELECTION_TRANSLATE_ACTION;
use snaplingo_core::hotkeys::model::SHOW_TRANSLATION_WINDOW_ACTION;
use snaplingo_core::hotkeys::model::SILENT_SCREENSHOT_OCR_ACTION;
use snaplingo_core::hotkeys::model::TRANSLATION_CATEGORY;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct HotkeyActionBinding {
    pub action: AppAction,
}

pub(crate) fn hotkey_action_binding(
    category: &str,
    action_key: &str,
) -> Option<HotkeyActionBinding> {
    let action = match (category, action_key) {
        (SCREENSHOT_CATEGORY, SCREENSHOT_ACTION) => {
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
        (TRANSLATION_CATEGORY, SHOW_TRANSLATION_WINDOW_ACTION) => AppAction::OpenTranslationWindow,
        (OCR_CATEGORY, SCREENSHOT_OCR_ACTION) => {
            AppAction::OpenCapture(CaptureLaunchMode::ScreenshotOcr)
        }
        (OCR_CATEGORY, SILENT_SCREENSHOT_OCR_ACTION) => {
            AppAction::OpenCapture(CaptureLaunchMode::SilentScreenshotOcr)
        }
        (OCR_CATEGORY, FILE_OCR_ACTION) => AppAction::RunFileOcr,
        _ => return None,
    };

    Some(HotkeyActionBinding { action })
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::hotkey_action_binding;
    use crate::shell::actions::AppAction;
    use crate::shell::actions::CaptureLaunchMode;
    use snaplingo_core::hotkeys::model::DEFAULT_HOTKEYS;
    use snaplingo_core::hotkeys::model::FILE_OCR_ACTION;
    use snaplingo_core::hotkeys::model::OCR_CATEGORY;
    use snaplingo_core::hotkeys::model::PIN_ACTION;
    use snaplingo_core::hotkeys::model::PIN_SWITCH_GROUP_ACTION;
    use snaplingo_core::hotkeys::model::PIN_TOGGLE_ALL_ACTION;
    use snaplingo_core::hotkeys::model::SCREENSHOT_ACTION;
    use snaplingo_core::hotkeys::model::SCREENSHOT_CATEGORY;
    use snaplingo_core::hotkeys::model::SCREENSHOT_COPY_ACTION;
    use snaplingo_core::hotkeys::model::SCREENSHOT_OCR_ACTION;
    use snaplingo_core::hotkeys::model::SCREENSHOT_TRANSLATE_ACTION;
    use snaplingo_core::hotkeys::model::SELECTION_TRANSLATE_ACTION;
    use snaplingo_core::hotkeys::model::SHOW_TRANSLATION_WINDOW_ACTION;
    use snaplingo_core::hotkeys::model::SILENT_SCREENSHOT_OCR_ACTION;
    use snaplingo_core::hotkeys::model::TRANSLATION_CATEGORY;

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

        assert_eq!(cases.len(), 11);
        assert_eq!(cases.len(), DEFAULT_HOTKEYS.len());
        assert_eq!(covered, defaults);
        assert!(hotkey_action_binding("unknown", "unknown").is_none());
    }
}

use crate::app_actions::{dispatch_app_action, AppAction, CaptureLaunchMode};
use crate::application::hotkeys::{HotkeyRegistrar, HotkeyRegistration, HotkeyTriggerTiming};
use crate::domain::hotkey_config::{
    FILE_OCR_ACTION, INPUT_TRANSLATE_ACTION, OCR_CATEGORY, PIN_ACTION, PIN_SWITCH_GROUP_ACTION,
    PIN_TOGGLE_ALL_ACTION, SCREENSHOT_ACTION, SCREENSHOT_CATEGORY, SCREENSHOT_COPY_ACTION,
    SCREENSHOT_CUSTOM_ACTION, SCREENSHOT_OCR_ACTION, SCREENSHOT_TRANSLATE_ACTION,
    SELECTION_TRANSLATE_ACTION, SHOW_OCR_WINDOW_ACTION, SHOW_TRANSLATION_WINDOW_ACTION,
    SILENT_SCREENSHOT_OCR_ACTION, TRANSLATION_CATEGORY,
};
use crate::Result;

pub(crate) struct TauriHotkeyRegistrar {
    app: tauri::AppHandle,
}

impl TauriHotkeyRegistrar {
    pub(crate) fn new(app: tauri::AppHandle) -> Self {
        Self { app }
    }
}

impl HotkeyRegistrar for TauriHotkeyRegistrar {
    fn register(&self, registration: HotkeyRegistration) -> Result<()> {
        let category = registration.category.clone();
        let action = registration.action.clone();
        let app = self.app.clone();

        if registration.timing == HotkeyTriggerTiming::Released {
            return crate::infrastructure::system::register_shortcut_on_release(
                &self.app,
                &registration.accelerator,
                move || trigger_hotkey_action(app.clone(), category.clone(), action.clone()),
            );
        }

        crate::infrastructure::system::register_shortcut(
            &self.app,
            &registration.accelerator,
            move || trigger_hotkey_action(app.clone(), category.clone(), action.clone()),
        )
    }

    fn unregister(&self, accelerator: &str) -> Result<()> {
        crate::infrastructure::system::unregister_shortcut(&self.app, accelerator)
    }
}

#[cfg(test)]
fn resolve_hotkey_accelerator(
    category: &str,
    action: &str,
    hotkey: &str,
) -> Result<Option<String>> {
    crate::domain::hotkey_config::validate_hotkey_action(category, action)?;

    let next_accelerator = display_hotkey_to_accelerator(hotkey)?;
    Ok(next_accelerator)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct HotkeyActionBinding {
    pub action: AppAction,
    pub trigger_on_release: bool,
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

    let trigger_on_release = matches!(
        action,
        AppAction::OpenCapture(_) | AppAction::TranslateSelection
    );

    Some(HotkeyActionBinding {
        action,
        trigger_on_release,
    })
}

pub(crate) fn trigger_hotkey_action(app: tauri::AppHandle, category: String, action_key: String) {
    let Some(binding) = hotkey_action_binding(&category, &action_key) else {
        log::warn!("Unknown hotkey action: {}:{}", category, action_key);
        return;
    };

    dispatch_app_action(app, binding.action);
}

pub(crate) fn should_register_hotkey_on_release(category: &str, action_key: &str) -> bool {
    hotkey_action_binding(category, action_key).is_some_and(|binding| binding.trigger_on_release)
}

pub(crate) fn display_hotkey_to_accelerator(hotkey: &str) -> Result<Option<String>> {
    let hotkey = hotkey.trim();
    if hotkey.is_empty() || hotkey == "未设置" {
        return Ok(None);
    }

    let mut modifiers = Vec::new();
    let mut main_key = String::new();

    for ch in hotkey.chars() {
        match ch {
            '⇧' => modifiers.push("Shift"),
            '⌥' => modifiers.push("Alt"),
            '⌘' => modifiers.push("CmdOrCtrl"),
            '⌃' => modifiers.push("Ctrl"),
            _ if !ch.is_whitespace() => main_key.push(ch),
            _ => {}
        }
    }

    if main_key.is_empty() {
        return Err(crate::AppError::Other(format!(
            "Shortcut '{}' has no main key",
            hotkey
        )));
    }

    let accelerator_key = accelerator_key(&main_key)?;
    modifiers.push(accelerator_key.as_str());
    Ok(Some(modifiers.join("+")))
}

fn accelerator_key(main_key: &str) -> Result<String> {
    let upper = main_key.to_ascii_uppercase();
    if upper.len() == 1 {
        let ch = upper.chars().next().unwrap();
        if ch.is_ascii_alphabetic() {
            return Ok(format!("Key{}", ch));
        }
        if ch.is_ascii_digit() {
            return Ok(format!("Digit{}", ch));
        }
    }

    if matches!(
        upper.as_str(),
        "F1" | "F2"
            | "F3"
            | "F4"
            | "F5"
            | "F6"
            | "F7"
            | "F8"
            | "F9"
            | "F10"
            | "F11"
            | "F12"
            | "F13"
            | "F14"
            | "F15"
            | "F16"
            | "F17"
            | "F18"
            | "F19"
            | "F20"
    ) {
        return Ok(upper);
    }

    Err(crate::AppError::Other(format!(
        "Unsupported shortcut key '{}'",
        main_key
    )))
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::str::FromStr;

    use crate::app_actions::{AppAction, CaptureLaunchMode};
    use crate::domain::hotkey_config::{
        DEFAULT_HOTKEYS, FILE_OCR_ACTION, INPUT_TRANSLATE_ACTION, OCR_CATEGORY, PIN_ACTION,
        PIN_SWITCH_GROUP_ACTION, PIN_TOGGLE_ALL_ACTION, SCREENSHOT_ACTION, SCREENSHOT_CATEGORY,
        SCREENSHOT_COPY_ACTION, SCREENSHOT_CUSTOM_ACTION, SCREENSHOT_OCR_ACTION,
        SCREENSHOT_TRANSLATE_ACTION, SELECTION_TRANSLATE_ACTION, SHOW_OCR_WINDOW_ACTION,
        SHOW_TRANSLATION_WINDOW_ACTION, SILENT_SCREENSHOT_OCR_ACTION, TRANSLATION_CATEGORY,
    };
    use tauri_plugin_global_shortcut::Shortcut;

    use super::{
        display_hotkey_to_accelerator, hotkey_action_binding, resolve_hotkey_accelerator,
        should_register_hotkey_on_release,
    };

    #[test]
    fn maps_hotkey_keys_to_app_actions_and_trigger_timing() {
        let cases = [
            (
                SCREENSHOT_CATEGORY,
                SCREENSHOT_ACTION,
                AppAction::OpenCapture(CaptureLaunchMode::Screenshot),
                true,
            ),
            (
                SCREENSHOT_CATEGORY,
                SCREENSHOT_COPY_ACTION,
                AppAction::OpenCapture(CaptureLaunchMode::ScreenshotCopy),
                true,
            ),
            (
                SCREENSHOT_CATEGORY,
                SCREENSHOT_CUSTOM_ACTION,
                AppAction::OpenCapture(CaptureLaunchMode::Screenshot),
                true,
            ),
            (
                SCREENSHOT_CATEGORY,
                PIN_ACTION,
                AppAction::PinClipboardImage,
                false,
            ),
            (
                SCREENSHOT_CATEGORY,
                PIN_TOGGLE_ALL_ACTION,
                AppAction::TogglePinnedImagesVisibility,
                false,
            ),
            (
                SCREENSHOT_CATEGORY,
                PIN_SWITCH_GROUP_ACTION,
                AppAction::SwitchPinnedImageGroup,
                false,
            ),
            (
                TRANSLATION_CATEGORY,
                SELECTION_TRANSLATE_ACTION,
                AppAction::TranslateSelection,
                true,
            ),
            (
                TRANSLATION_CATEGORY,
                SCREENSHOT_TRANSLATE_ACTION,
                AppAction::OpenCapture(CaptureLaunchMode::ScreenshotTranslate),
                true,
            ),
            (
                TRANSLATION_CATEGORY,
                INPUT_TRANSLATE_ACTION,
                AppAction::OpenInputTranslation,
                false,
            ),
            (
                TRANSLATION_CATEGORY,
                SHOW_TRANSLATION_WINDOW_ACTION,
                AppAction::OpenTranslationWindow,
                false,
            ),
            (
                OCR_CATEGORY,
                SCREENSHOT_OCR_ACTION,
                AppAction::OpenCapture(CaptureLaunchMode::ScreenshotOcr),
                true,
            ),
            (
                OCR_CATEGORY,
                SILENT_SCREENSHOT_OCR_ACTION,
                AppAction::OpenCapture(CaptureLaunchMode::SilentScreenshotOcr),
                true,
            ),
            (OCR_CATEGORY, FILE_OCR_ACTION, AppAction::RunFileOcr, false),
            (
                OCR_CATEGORY,
                SHOW_OCR_WINDOW_ACTION,
                AppAction::OpenOcrWindow,
                false,
            ),
        ];

        for (category, action_key, expected_action, expected_trigger_on_release) in cases {
            let binding = hotkey_action_binding(category, action_key).unwrap();

            assert_eq!(binding.action, expected_action);
            assert_eq!(binding.trigger_on_release, expected_trigger_on_release);
        }

        let covered: HashSet<_> = cases
            .iter()
            .map(|(category, action, _, _)| (*category, *action))
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

    #[test]
    fn converts_display_hotkeys_to_tauri_accelerators() {
        assert_eq!(
            display_hotkey_to_accelerator("⌥D").unwrap(),
            Some("Alt+KeyD".to_string())
        );
        assert_eq!(
            display_hotkey_to_accelerator("⇧⌥S").unwrap(),
            Some("Shift+Alt+KeyS".to_string())
        );
        assert_eq!(
            display_hotkey_to_accelerator("⌘F3").unwrap(),
            Some("CmdOrCtrl+F3".to_string())
        );
    }

    #[test]
    fn converted_accelerators_parse_as_tauri_shortcuts() {
        for hotkey in ["⌥D", "⇧⌥S", "⌘F3"] {
            let accelerator = display_hotkey_to_accelerator(hotkey).unwrap().unwrap();
            Shortcut::from_str(&accelerator).unwrap();
        }
    }

    #[test]
    fn converts_recorded_multi_modifier_hotkeys() {
        assert_eq!(
            display_hotkey_to_accelerator("⇧⌥⌘⌃D").unwrap(),
            Some("Shift+Alt+CmdOrCtrl+Ctrl+KeyD".to_string())
        );
        let accelerator = display_hotkey_to_accelerator("⇧⌥⌘⌃D").unwrap().unwrap();
        Shortcut::from_str(&accelerator).unwrap();
    }

    #[test]
    fn treats_unset_display_hotkeys_as_unregistered() {
        assert_eq!(display_hotkey_to_accelerator("未设置").unwrap(), None);
        assert_eq!(display_hotkey_to_accelerator("  ").unwrap(), None);
    }

    #[test]
    fn rejects_modifier_only_display_hotkeys() {
        let err = display_hotkey_to_accelerator("⌘").unwrap_err();
        assert!(err.to_string().contains("has no main key"));
    }

    #[test]
    fn resolves_implemented_hotkey_actions() {
        assert_eq!(
            resolve_hotkey_accelerator(SCREENSHOT_CATEGORY, "pin", "F3").unwrap(),
            Some("F3".to_string())
        );
        assert_eq!(
            resolve_hotkey_accelerator(OCR_CATEGORY, "screenshot-ocr", "⇧⌥S").unwrap(),
            Some("Shift+Alt+KeyS".to_string())
        );
    }

    #[test]
    fn resolves_ocr_hotkey_actions() {
        assert_eq!(
            resolve_hotkey_accelerator(OCR_CATEGORY, SILENT_SCREENSHOT_OCR_ACTION, "⌘F5").unwrap(),
            Some("CmdOrCtrl+F5".to_string())
        );
        assert_eq!(
            resolve_hotkey_accelerator(OCR_CATEGORY, FILE_OCR_ACTION, "⌘F").unwrap(),
            Some("CmdOrCtrl+KeyF".to_string())
        );
        assert_eq!(
            resolve_hotkey_accelerator(OCR_CATEGORY, SHOW_OCR_WINDOW_ACTION, "⌘O").unwrap(),
            Some("CmdOrCtrl+KeyO".to_string())
        );
    }

    #[test]
    fn capture_hotkeys_trigger_after_the_key_combo_is_released() {
        assert!(should_register_hotkey_on_release(
            SCREENSHOT_CATEGORY,
            SCREENSHOT_ACTION
        ));
        assert!(should_register_hotkey_on_release(
            SCREENSHOT_CATEGORY,
            SCREENSHOT_COPY_ACTION
        ));
        assert!(should_register_hotkey_on_release(
            OCR_CATEGORY,
            SCREENSHOT_OCR_ACTION
        ));
        assert!(should_register_hotkey_on_release(
            OCR_CATEGORY,
            SILENT_SCREENSHOT_OCR_ACTION
        ));
        assert!(should_register_hotkey_on_release(
            TRANSLATION_CATEGORY,
            SCREENSHOT_TRANSLATE_ACTION
        ));
    }
}

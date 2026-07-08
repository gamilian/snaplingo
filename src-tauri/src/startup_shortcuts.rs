use tauri::Manager;

use crate::domain::hotkey_config::{
    FILE_OCR_ACTION, INPUT_TRANSLATE_ACTION, OCR_CATEGORY, PIN_ACTION, PIN_SWITCH_GROUP_ACTION,
    PIN_TOGGLE_ALL_ACTION, SCREENSHOT_ACTION, SCREENSHOT_CATEGORY, SCREENSHOT_COPY_ACTION,
    SCREENSHOT_CUSTOM_ACTION, SCREENSHOT_OCR_ACTION, SCREENSHOT_TRANSLATE_ACTION,
    SELECTION_TRANSLATE_ACTION, SHOW_OCR_WINDOW_ACTION, SHOW_TRANSLATION_WINDOW_ACTION,
    SILENT_SCREENSHOT_OCR_ACTION, TRANSLATION_CATEGORY,
};
use crate::{commands, AppState, Result};

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

pub(crate) fn trigger_hotkey_action(app: tauri::AppHandle, category: String, action: String) {
    if category == SCREENSHOT_CATEGORY {
        if let Some(mode) = capture_mode_for_screenshot_hotkey_action(&action) {
            tauri::async_runtime::spawn(commands::open_capture_window_from_shortcut(app, mode));
            return;
        }
    }

    match (category.as_str(), action.as_str()) {
        (SCREENSHOT_CATEGORY, PIN_ACTION) => {
            let state = app.state::<AppState>();
            if let Err(err) = commands::pin_clipboard_image_for_state(&app, state.inner()) {
                log::error!("Failed to pin clipboard image: {}", err);
            }
        }
        (SCREENSHOT_CATEGORY, PIN_TOGGLE_ALL_ACTION) => {
            if let Err(err) = commands::toggle_pinned_images_visibility(app) {
                log::error!("Failed to toggle pinned images: {}", err);
            }
        }
        (SCREENSHOT_CATEGORY, PIN_SWITCH_GROUP_ACTION) => {
            let state = app.state::<AppState>();
            if let Err(err) = commands::switch_pinned_image_group_for_state(&app, state.inner()) {
                log::error!("Failed to switch pinned image group: {}", err);
            }
        }
        (TRANSLATION_CATEGORY, SCREENSHOT_TRANSLATE_ACTION) => {
            tauri::async_runtime::spawn(commands::open_capture_window_from_shortcut(
                app,
                "screenshot-translate",
            ));
        }
        (TRANSLATION_CATEGORY, SELECTION_TRANSLATE_ACTION) => {
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
        (TRANSLATION_CATEGORY, INPUT_TRANSLATE_ACTION) => {
            if let Err(err) = commands::open_input_translation_window(app) {
                log::error!("Failed to open input translation window: {}", err);
            }
        }
        (TRANSLATION_CATEGORY, SHOW_TRANSLATION_WINDOW_ACTION) => {
            if let Err(err) = commands::show_translation_window(app) {
                log::error!("Failed to show translation window: {}", err);
            }
        }
        (OCR_CATEGORY, SCREENSHOT_OCR_ACTION) => {
            tauri::async_runtime::spawn(commands::open_capture_window_from_shortcut(
                app,
                "screenshot-ocr",
            ));
        }
        (OCR_CATEGORY, SILENT_SCREENSHOT_OCR_ACTION) => {
            tauri::async_runtime::spawn(commands::open_capture_window_from_shortcut(
                app,
                "silent-screenshot-ocr",
            ));
        }
        (OCR_CATEGORY, FILE_OCR_ACTION) => {
            if let Err(err) = commands::start_file_ocr(app) {
                log::error!("Failed to start file OCR: {}", err);
            }
        }
        (OCR_CATEGORY, SHOW_OCR_WINDOW_ACTION) => {
            if let Err(err) = commands::show_ocr_window(app) {
                log::error!("Failed to show OCR window: {}", err);
            }
        }
        _ => {
            log::warn!("Unknown hotkey action: {}:{}", category, action);
        }
    }
}

fn capture_mode_for_screenshot_hotkey_action(action: &str) -> Option<&'static str> {
    match action {
        SCREENSHOT_ACTION | SCREENSHOT_CUSTOM_ACTION => Some("screenshot"),
        SCREENSHOT_COPY_ACTION => Some("screenshot-copy"),
        _ => None,
    }
}

pub(crate) fn should_register_hotkey_on_release(category: &str, action: &str) -> bool {
    match (category, action) {
        (SCREENSHOT_CATEGORY, action) => {
            capture_mode_for_screenshot_hotkey_action(action).is_some()
        }
        (TRANSLATION_CATEGORY, SELECTION_TRANSLATE_ACTION | SCREENSHOT_TRANSLATE_ACTION) => true,
        (OCR_CATEGORY, SCREENSHOT_OCR_ACTION | SILENT_SCREENSHOT_OCR_ACTION) => true,
        _ => false,
    }
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
    use super::{
        capture_mode_for_screenshot_hotkey_action, display_hotkey_to_accelerator,
        resolve_hotkey_accelerator, should_register_hotkey_on_release, FILE_OCR_ACTION,
        OCR_CATEGORY, SCREENSHOT_ACTION, SCREENSHOT_CATEGORY, SCREENSHOT_COPY_ACTION,
        SCREENSHOT_CUSTOM_ACTION, SCREENSHOT_OCR_ACTION, SCREENSHOT_TRANSLATE_ACTION,
        SHOW_OCR_WINDOW_ACTION, SILENT_SCREENSHOT_OCR_ACTION, TRANSLATION_CATEGORY,
    };
    use std::str::FromStr;
    use tauri_plugin_global_shortcut::Shortcut;

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
    fn resolves_capture_modes_for_screenshot_hotkey_actions() {
        assert_eq!(
            capture_mode_for_screenshot_hotkey_action(SCREENSHOT_ACTION),
            Some("screenshot")
        );
        assert_eq!(
            capture_mode_for_screenshot_hotkey_action(SCREENSHOT_COPY_ACTION),
            Some("screenshot-copy")
        );
        assert_eq!(
            capture_mode_for_screenshot_hotkey_action(SCREENSHOT_CUSTOM_ACTION),
            Some("screenshot")
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

use crate::domain::hotkey_config::{
    OCR_CATEGORY, SCREENSHOT_ACTION, SCREENSHOT_CATEGORY, SCREENSHOT_COPY_ACTION,
    SCREENSHOT_OCR_ACTION, SCREENSHOT_TRANSLATE_ACTION, SELECTION_TRANSLATE_ACTION,
    SILENT_SCREENSHOT_OCR_ACTION, TRANSLATION_CATEGORY,
};
use crate::{AppError, Result};

pub(crate) fn should_register_hotkey_on_release(category: &str, action: &str) -> bool {
    matches!(
        (category, action),
        (
            SCREENSHOT_CATEGORY,
            SCREENSHOT_ACTION | SCREENSHOT_COPY_ACTION
        ) | (
            TRANSLATION_CATEGORY,
            SCREENSHOT_TRANSLATE_ACTION | SELECTION_TRANSLATE_ACTION
        ) | (
            OCR_CATEGORY,
            SCREENSHOT_OCR_ACTION | SILENT_SCREENSHOT_OCR_ACTION
        )
    )
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
        return Err(AppError::Other(format!(
            "Shortcut '{}' has no main key",
            hotkey
        )));
    }

    let accelerator_key = accelerator_key(&main_key)?;
    modifiers.push(accelerator_key.as_str());
    let accelerator = modifiers.join("+");
    if is_reserved_system_accelerator(&accelerator) {
        return Err(AppError::Other(format!(
            "Shortcut '{}' is reserved by the operating system",
            hotkey
        )));
    }
    Ok(Some(accelerator))
}

fn is_reserved_system_accelerator(accelerator: &str) -> bool {
    matches!(
        accelerator,
        "CmdOrCtrl+KeyA"
            | "CmdOrCtrl+KeyC"
            | "CmdOrCtrl+KeyV"
            | "CmdOrCtrl+KeyX"
            | "CmdOrCtrl+KeyZ"
            | "Shift+CmdOrCtrl+KeyZ"
            | "Ctrl+KeyA"
            | "Ctrl+KeyC"
            | "Ctrl+KeyV"
            | "Ctrl+KeyX"
            | "Ctrl+KeyZ"
            | "Shift+Ctrl+KeyZ"
    )
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

    Err(AppError::Other(format!(
        "Unsupported shortcut key '{}'",
        main_key
    )))
}

#[cfg(test)]
mod tests {
    use super::{display_hotkey_to_accelerator, should_register_hotkey_on_release};
    use crate::domain::hotkey_config::{
        OCR_CATEGORY, SCREENSHOT_ACTION, SCREENSHOT_CATEGORY, SCREENSHOT_COPY_ACTION,
        SCREENSHOT_OCR_ACTION, SCREENSHOT_TRANSLATE_ACTION, SELECTION_TRANSLATE_ACTION,
        SILENT_SCREENSHOT_OCR_ACTION, TRANSLATION_CATEGORY,
    };

    #[test]
    fn converts_display_hotkeys_to_accelerators() {
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
    fn treats_unset_display_hotkeys_as_unregistered() {
        assert_eq!(display_hotkey_to_accelerator("未设置").unwrap(), None);
        assert_eq!(display_hotkey_to_accelerator("  ").unwrap(), None);
    }

    #[test]
    fn rejects_modifier_only_display_hotkeys() {
        assert!(display_hotkey_to_accelerator("⌘")
            .unwrap_err()
            .to_string()
            .contains("has no main key"));
    }

    #[test]
    fn rejects_system_edit_shortcuts_that_would_break_other_apps() {
        for hotkey in ["⌘C", "⌘V", "⌘X", "⌘A", "⌘Z", "⇧⌘Z", "⌃C"] {
            assert!(display_hotkey_to_accelerator(hotkey)
                .unwrap_err()
                .to_string()
                .contains("reserved by the operating system"));
        }
    }

    #[test]
    fn capture_hotkeys_trigger_after_release() {
        for (category, action) in [
            (SCREENSHOT_CATEGORY, SCREENSHOT_ACTION),
            (SCREENSHOT_CATEGORY, SCREENSHOT_COPY_ACTION),
            (OCR_CATEGORY, SCREENSHOT_OCR_ACTION),
            (OCR_CATEGORY, SILENT_SCREENSHOT_OCR_ACTION),
            (TRANSLATION_CATEGORY, SCREENSHOT_TRANSLATE_ACTION),
            (TRANSLATION_CATEGORY, SELECTION_TRANSLATE_ACTION),
        ] {
            assert!(should_register_hotkey_on_release(category, action));
        }
    }
}

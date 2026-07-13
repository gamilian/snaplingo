use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::Result;

pub const HOTKEY_UNSET: &str = "未设置";

pub const SCREENSHOT_CATEGORY: &str = "screenshot";
pub const TRANSLATION_CATEGORY: &str = "translation";
pub const OCR_CATEGORY: &str = "ocr";

pub const SCREENSHOT_ACTION: &str = "screenshot";
pub const SCREENSHOT_COPY_ACTION: &str = "screenshot-copy";
pub const PIN_ACTION: &str = "pin";
pub const PIN_TOGGLE_ALL_ACTION: &str = "pin-toggle-all";
pub const PIN_SWITCH_GROUP_ACTION: &str = "pin-switch-group";
pub const SELECTION_TRANSLATE_ACTION: &str = "selection-translate";
pub const SCREENSHOT_TRANSLATE_ACTION: &str = "screenshot-translate";
pub const INPUT_TRANSLATE_ACTION: &str = "input-translate";
pub const SHOW_TRANSLATION_WINDOW_ACTION: &str = "show-window";
pub const SCREENSHOT_OCR_ACTION: &str = "screenshot-ocr";
pub const SILENT_SCREENSHOT_OCR_ACTION: &str = "silent-screenshot-ocr";
pub const FILE_OCR_ACTION: &str = "file-ocr";
pub const SHOW_OCR_WINDOW_ACTION: &str = "show-window";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DefaultHotkey {
    pub category: &'static str,
    pub action: &'static str,
    pub hotkey: &'static str,
}

pub const DEFAULT_HOTKEYS: &[DefaultHotkey] = &[
    DefaultHotkey {
        category: SCREENSHOT_CATEGORY,
        action: SCREENSHOT_ACTION,
        hotkey: "⇧⌘R",
    },
    DefaultHotkey {
        category: SCREENSHOT_CATEGORY,
        action: SCREENSHOT_COPY_ACTION,
        hotkey: "⌘F1",
    },
    DefaultHotkey {
        category: SCREENSHOT_CATEGORY,
        action: PIN_ACTION,
        hotkey: "F3",
    },
    DefaultHotkey {
        category: SCREENSHOT_CATEGORY,
        action: PIN_TOGGLE_ALL_ACTION,
        hotkey: "⇧F3",
    },
    DefaultHotkey {
        category: SCREENSHOT_CATEGORY,
        action: PIN_SWITCH_GROUP_ACTION,
        hotkey: "⌘F3",
    },
    DefaultHotkey {
        category: TRANSLATION_CATEGORY,
        action: SELECTION_TRANSLATE_ACTION,
        hotkey: "⌥D",
    },
    DefaultHotkey {
        category: TRANSLATION_CATEGORY,
        action: SCREENSHOT_TRANSLATE_ACTION,
        hotkey: "⌥S",
    },
    DefaultHotkey {
        category: TRANSLATION_CATEGORY,
        action: INPUT_TRANSLATE_ACTION,
        hotkey: "⌥A",
    },
    DefaultHotkey {
        category: TRANSLATION_CATEGORY,
        action: SHOW_TRANSLATION_WINDOW_ACTION,
        hotkey: HOTKEY_UNSET,
    },
    DefaultHotkey {
        category: OCR_CATEGORY,
        action: SCREENSHOT_OCR_ACTION,
        hotkey: "⇧⌥S",
    },
    DefaultHotkey {
        category: OCR_CATEGORY,
        action: SILENT_SCREENSHOT_OCR_ACTION,
        hotkey: HOTKEY_UNSET,
    },
    DefaultHotkey {
        category: OCR_CATEGORY,
        action: FILE_OCR_ACTION,
        hotkey: HOTKEY_UNSET,
    },
    DefaultHotkey {
        category: OCR_CATEGORY,
        action: SHOW_OCR_WINDOW_ACTION,
        hotkey: HOTKEY_UNSET,
    },
];

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotkeySettingsSnapshot {
    #[serde(default)]
    pub screenshot: HashMap<String, String>,
    #[serde(default)]
    pub translation: HashMap<String, String>,
    #[serde(default)]
    pub ocr: HashMap<String, String>,
}

pub fn default_hotkey_snapshot() -> HotkeySettingsSnapshot {
    let mut snapshot = HotkeySettingsSnapshot::default();

    for hotkey in DEFAULT_HOTKEYS {
        if let Ok(category_hotkeys) = hotkey_category_mut(&mut snapshot, hotkey.category) {
            category_hotkeys.insert(hotkey.action.to_string(), hotkey.hotkey.to_string());
        }
    }

    snapshot
}

pub fn validate_hotkey_action(category: &str, action: &str) -> Result<()> {
    if hotkey_category(&default_hotkey_snapshot(), category).is_none() {
        return Err(crate::AppError::Other(format!(
            "Unknown hotkey category '{}'",
            category
        )));
    }

    if !DEFAULT_HOTKEYS
        .iter()
        .any(|hotkey| hotkey.category == category && hotkey.action == action)
    {
        return Err(crate::AppError::Other(format!(
            "Unknown hotkey action '{}:{}'",
            category, action
        )));
    }

    Ok(())
}

pub fn hotkey_category<'a>(
    snapshot: &'a HotkeySettingsSnapshot,
    category: &str,
) -> Option<&'a HashMap<String, String>> {
    match category {
        SCREENSHOT_CATEGORY => Some(&snapshot.screenshot),
        TRANSLATION_CATEGORY => Some(&snapshot.translation),
        OCR_CATEGORY => Some(&snapshot.ocr),
        _ => None,
    }
}

pub fn hotkey_category_mut<'a>(
    snapshot: &'a mut HotkeySettingsSnapshot,
    category: &str,
) -> Result<&'a mut HashMap<String, String>> {
    match category {
        SCREENSHOT_CATEGORY => Ok(&mut snapshot.screenshot),
        TRANSLATION_CATEGORY => Ok(&mut snapshot.translation),
        OCR_CATEGORY => Ok(&mut snapshot.ocr),
        _ => Err(crate::AppError::Other(format!(
            "Unknown hotkey category '{}'",
            category
        ))),
    }
}

#[cfg(test)]
mod hotkey_config_tests {
    use super::{default_hotkey_snapshot, validate_hotkey_action, HOTKEY_UNSET};

    #[test]
    fn hotkey_config_defaults_are_sectioned_by_feature() {
        let snapshot = default_hotkey_snapshot();

        assert_eq!(
            snapshot.screenshot.get("screenshot"),
            Some(&"⇧⌘R".to_string())
        );
        assert_eq!(
            snapshot.screenshot.get("screenshot-copy"),
            Some(&"⌘F1".to_string())
        );
        assert_eq!(snapshot.screenshot.get("pin"), Some(&"F3".to_string()));
        assert_eq!(
            snapshot.screenshot.get("pin-toggle-all"),
            Some(&"⇧F3".to_string())
        );
        assert_eq!(
            snapshot.screenshot.get("pin-switch-group"),
            Some(&"⌘F3".to_string())
        );

        assert_eq!(
            snapshot.translation.get("selection-translate"),
            Some(&"⌥D".to_string())
        );
        assert_eq!(
            snapshot.translation.get("screenshot-translate"),
            Some(&"⌥S".to_string())
        );
        assert_eq!(
            snapshot.translation.get("input-translate"),
            Some(&"⌥A".to_string())
        );
        assert_eq!(
            snapshot.translation.get("show-window"),
            Some(&HOTKEY_UNSET.to_string())
        );

        assert_eq!(snapshot.ocr.get("screenshot-ocr"), Some(&"⇧⌥S".to_string()));
        assert_eq!(
            snapshot.ocr.get("silent-screenshot-ocr"),
            Some(&HOTKEY_UNSET.to_string())
        );
        assert_eq!(
            snapshot.ocr.get("file-ocr"),
            Some(&HOTKEY_UNSET.to_string())
        );
        assert_eq!(
            snapshot.ocr.get("show-window"),
            Some(&HOTKEY_UNSET.to_string())
        );
    }

    #[test]
    fn hotkey_config_rejects_unknown_categories_and_actions() {
        let unknown_category = validate_hotkey_action("unknown", "screenshot").unwrap_err();
        assert!(unknown_category
            .to_string()
            .contains("Unknown hotkey category 'unknown'"));

        let unknown_action = validate_hotkey_action("screenshot", "missing").unwrap_err();
        assert!(unknown_action
            .to_string()
            .contains("Unknown hotkey action 'screenshot:missing'"));
    }
}

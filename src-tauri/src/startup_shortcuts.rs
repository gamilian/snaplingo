use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use tauri::Manager;

use crate::{commands, infrastructure, AppState, Result};

const SCREENSHOT_CATEGORY: &str = "screenshot";
const TRANSLATION_CATEGORY: &str = "translation";
const OCR_CATEGORY: &str = "ocr";

const SCREENSHOT_ACTION: &str = "screenshot";
const SCREENSHOT_COPY_ACTION: &str = "screenshot-copy";
const SCREENSHOT_CUSTOM_ACTION: &str = "screenshot-custom";
const PIN_ACTION: &str = "pin";
const PIN_TOGGLE_ALL_ACTION: &str = "pin-toggle-all";
const PIN_SWITCH_GROUP_ACTION: &str = "pin-switch-group";
const SELECTION_TRANSLATE_ACTION: &str = "selection-translate";
const SCREENSHOT_TRANSLATE_ACTION: &str = "screenshot-translate";
const INPUT_TRANSLATE_ACTION: &str = "input-translate";
const SHOW_TRANSLATION_WINDOW_ACTION: &str = "show-window";
const SCREENSHOT_OCR_ACTION: &str = "screenshot-ocr";
const SILENT_SCREENSHOT_OCR_ACTION: &str = "silent-screenshot-ocr";
const FILE_OCR_ACTION: &str = "file-ocr";
const SHOW_OCR_WINDOW_ACTION: &str = "show-window";

static HOTKEY_REGISTRATIONS: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

const STARTUP_HOTKEYS: &[(&str, &str, &str)] = &[
    (SCREENSHOT_CATEGORY, SCREENSHOT_ACTION, "⇧⌘R"),
    (SCREENSHOT_CATEGORY, SCREENSHOT_COPY_ACTION, "⌘F1"),
    (SCREENSHOT_CATEGORY, SCREENSHOT_CUSTOM_ACTION, "⇧F1"),
    (SCREENSHOT_CATEGORY, PIN_ACTION, "F3"),
    (SCREENSHOT_CATEGORY, PIN_TOGGLE_ALL_ACTION, "⇧F3"),
    (SCREENSHOT_CATEGORY, PIN_SWITCH_GROUP_ACTION, "⌘F3"),
    (TRANSLATION_CATEGORY, SELECTION_TRANSLATE_ACTION, "⌥D"),
    (TRANSLATION_CATEGORY, SCREENSHOT_TRANSLATE_ACTION, "⌥S"),
    (TRANSLATION_CATEGORY, INPUT_TRANSLATE_ACTION, "⌥A"),
    (TRANSLATION_CATEGORY, SHOW_TRANSLATION_WINDOW_ACTION, "未设置"),
    (OCR_CATEGORY, SCREENSHOT_OCR_ACTION, "⇧⌥S"),
    (OCR_CATEGORY, SILENT_SCREENSHOT_OCR_ACTION, "未设置"),
    (OCR_CATEGORY, FILE_OCR_ACTION, "未设置"),
    (OCR_CATEGORY, SHOW_OCR_WINDOW_ACTION, "未设置"),
];

pub(crate) async fn register_startup_shortcuts(app: tauri::AppHandle) {
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    for (category, action, hotkey) in STARTUP_HOTKEYS {
        match configure_hotkey(&app, category, action, hotkey) {
            Ok(Some(accelerator)) => {
                log::info!(
                    "Hotkey registered: {}:{} -> {}",
                    category,
                    action,
                    accelerator
                );
            }
            Ok(None) => {}
            Err(e) => {
                log::error!("Failed to register hotkey {}:{}: {}", category, action, e);
            }
        }
    }
}

pub(crate) fn configure_hotkey(
    app: &tauri::AppHandle,
    category: &str,
    action: &str,
    hotkey: &str,
) -> Result<Option<String>> {
    let next_accelerator = resolve_hotkey_accelerator(category, action, hotkey)?;
    let registration_key = hotkey_registration_key(category, action);
    let registry = HOTKEY_REGISTRATIONS.get_or_init(|| Mutex::new(HashMap::new()));

    let previous_accelerator = {
        let registrations = registry
            .lock()
            .map_err(|e| crate::AppError::Other(format!("Shortcut registry lock poisoned: {e}")))?;
        registrations.get(&registration_key).cloned()
    };

    if next_accelerator == previous_accelerator {
        return Ok(next_accelerator);
    }

    if let Some(accelerator) = &next_accelerator {
        register_hotkey_action(app, category, action, accelerator)?;
    }

    if let Some(accelerator) = previous_accelerator {
        if let Err(e) = infrastructure::system::unregister_shortcut(app, &accelerator) {
            log::warn!(
                "Failed to unregister previous hotkey {} for {}:{}: {}",
                accelerator,
                category,
                action,
                e
            );
        }
    }

    let mut registrations = registry
        .lock()
        .map_err(|e| crate::AppError::Other(format!("Shortcut registry lock poisoned: {e}")))?;
    match &next_accelerator {
        Some(accelerator) => {
            registrations.insert(registration_key, accelerator.clone());
        }
        None => {
            registrations.remove(&registration_key);
        }
    }

    Ok(next_accelerator)
}

pub(crate) fn configure_translation_shortcut(
    app: &tauri::AppHandle,
    action: &str,
    hotkey: &str,
) -> Result<Option<String>> {
    configure_hotkey(app, TRANSLATION_CATEGORY, action, hotkey)
}

fn resolve_hotkey_accelerator(
    category: &str,
    action: &str,
    hotkey: &str,
) -> Result<Option<String>> {
    if !is_known_hotkey_action(category, action) {
        return Err(crate::AppError::Other(format!(
            "Unknown hotkey action '{}:{}'",
            category,
            action
        )));
    }

    let next_accelerator = display_hotkey_to_accelerator(hotkey)?;
    if next_accelerator.is_some() && !is_implemented_hotkey_action(category, action) {
        return Err(crate::AppError::Other(format!(
            "Hotkey action '{}:{}' is not implemented",
            category, action
        )));
    }

    Ok(next_accelerator)
}

fn hotkey_registration_key(category: &str, action: &str) -> String {
    format!("{category}:{action}")
}

fn is_known_hotkey_action(category: &str, action: &str) -> bool {
    is_implemented_hotkey_action(category, action)
}

fn is_implemented_hotkey_action(category: &str, action: &str) -> bool {
    matches!(
        (category, action),
        (SCREENSHOT_CATEGORY, SCREENSHOT_ACTION)
            | (SCREENSHOT_CATEGORY, SCREENSHOT_COPY_ACTION)
            | (SCREENSHOT_CATEGORY, SCREENSHOT_CUSTOM_ACTION)
            | (SCREENSHOT_CATEGORY, PIN_ACTION)
            | (SCREENSHOT_CATEGORY, PIN_TOGGLE_ALL_ACTION)
            | (SCREENSHOT_CATEGORY, PIN_SWITCH_GROUP_ACTION)
            | (TRANSLATION_CATEGORY, SELECTION_TRANSLATE_ACTION)
            | (TRANSLATION_CATEGORY, SCREENSHOT_TRANSLATE_ACTION)
            | (TRANSLATION_CATEGORY, INPUT_TRANSLATE_ACTION)
            | (TRANSLATION_CATEGORY, SHOW_TRANSLATION_WINDOW_ACTION)
            | (OCR_CATEGORY, SCREENSHOT_OCR_ACTION)
            | (OCR_CATEGORY, SILENT_SCREENSHOT_OCR_ACTION)
            | (OCR_CATEGORY, FILE_OCR_ACTION)
            | (OCR_CATEGORY, SHOW_OCR_WINDOW_ACTION)
    )
}

fn register_hotkey_action(
    app: &tauri::AppHandle,
    category: &str,
    action: &str,
    accelerator: &str,
) -> Result<()> {
    let category = category.to_string();
    let action = action.to_string();
    let app_clone = app.clone();

    infrastructure::system::register_shortcut(app, accelerator, move || {
        trigger_hotkey_action(app_clone.clone(), category.clone(), action.clone());
    })
}

fn trigger_hotkey_action(app: tauri::AppHandle, category: String, action: String) {
    match (category.as_str(), action.as_str()) {
        (SCREENSHOT_CATEGORY, SCREENSHOT_ACTION)
        | (SCREENSHOT_CATEGORY, SCREENSHOT_COPY_ACTION)
        | (SCREENSHOT_CATEGORY, SCREENSHOT_CUSTOM_ACTION) => {
            tauri::async_runtime::spawn(commands::open_capture_window_from_shortcut(
                app,
                "screenshot",
            ));
        }
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
                if let Err(err) = commands::open_selection_translation_window(app).await {
                    log::error!("Failed to open selection translation window: {}", err);
                }
            });
        }
        (TRANSLATION_CATEGORY, INPUT_TRANSLATE_ACTION) => {
            if let Err(err) = commands::open_result_window(String::new(), app) {
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
        display_hotkey_to_accelerator, resolve_hotkey_accelerator, FILE_OCR_ACTION, OCR_CATEGORY,
        SCREENSHOT_CATEGORY, SHOW_OCR_WINDOW_ACTION, SILENT_SCREENSHOT_OCR_ACTION,
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
            resolve_hotkey_accelerator(OCR_CATEGORY, SILENT_SCREENSHOT_OCR_ACTION, "⌘F5")
                .unwrap(),
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
}

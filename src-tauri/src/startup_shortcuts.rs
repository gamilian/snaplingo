use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use tauri::Manager;

use crate::infrastructure::storage::ConfigFile;
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
const HOTKEY_CONFIG_KEY: &str = "hotkeys";

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
    (
        TRANSLATION_CATEGORY,
        SHOW_TRANSLATION_WINDOW_ACTION,
        "未设置",
    ),
    (OCR_CATEGORY, SCREENSHOT_OCR_ACTION, "⇧⌥S"),
    (OCR_CATEGORY, SILENT_SCREENSHOT_OCR_ACTION, "未设置"),
    (OCR_CATEGORY, FILE_OCR_ACTION, "未设置"),
    (OCR_CATEGORY, SHOW_OCR_WINDOW_ACTION, "未设置"),
];

pub(crate) async fn register_startup_shortcuts(app: tauri::AppHandle) {
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let startup_hotkeys = {
        let state = app.state::<AppState>();
        startup_hotkeys_from_config_file(&state.config_file)
    };

    for hotkey in startup_hotkeys {
        match configure_hotkey(&app, &hotkey.category, &hotkey.action, &hotkey.hotkey) {
            Ok(Some(accelerator)) => {
                log::info!(
                    "Hotkey registered: {}:{} -> {}",
                    hotkey.category,
                    hotkey.action,
                    accelerator
                );
            }
            Ok(None) => {}
            Err(e) => {
                log::error!(
                    "Failed to register hotkey {}:{}: {}",
                    hotkey.category,
                    hotkey.action,
                    e
                );
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StartupHotkey {
    category: String,
    action: String,
    hotkey: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct StoredHotkeyConfig {
    #[serde(default)]
    screenshot: HashMap<String, String>,
    #[serde(default)]
    translation: HashMap<String, String>,
    #[serde(default)]
    ocr: HashMap<String, String>,
}

pub(crate) fn save_hotkey_config(
    config_file: &ConfigFile,
    category: &str,
    action: &str,
    hotkey: &str,
) -> Result<()> {
    resolve_hotkey_accelerator(category, action, hotkey)?;

    let mut config = config_file
        .load::<StoredHotkeyConfig>(HOTKEY_CONFIG_KEY)
        .unwrap_or_else(|_| default_hotkey_config());
    hotkey_category_mut(&mut config, category)?.insert(action.to_string(), hotkey.to_string());
    config_file.save(HOTKEY_CONFIG_KEY, &config)
}

fn startup_hotkeys_from_config_file(config_file: &ConfigFile) -> Vec<StartupHotkey> {
    let saved_config = match config_file.load::<StoredHotkeyConfig>(HOTKEY_CONFIG_KEY) {
        Ok(config) => config,
        Err(err) => {
            log::info!(
                "No backend hotkey config loaded, trying legacy WebKit settings: {}",
                err
            );
            load_legacy_webkit_hotkey_config()
                .inspect(|legacy_config| {
                    let migrated =
                        merge_saved_hotkeys(default_hotkey_config(), legacy_config.clone());
                    if let Err(save_err) = config_file.save(HOTKEY_CONFIG_KEY, &migrated) {
                        log::warn!("Failed to migrate legacy hotkey config: {}", save_err);
                    } else {
                        log::info!("Migrated legacy hotkey config to backend config file");
                    }
                })
                .unwrap_or_default()
        }
    };
    let merged_config = merge_saved_hotkeys(default_hotkey_config(), saved_config);
    startup_hotkeys_from_config(&merged_config)
}

fn load_legacy_webkit_hotkey_config() -> Option<StoredHotkeyConfig> {
    let root = dirs::home_dir()?.join("Library/WebKit/com.snaplingo.app");
    let storage_paths = find_legacy_local_storage_paths(&root);

    for path in storage_paths {
        match legacy_hotkey_config_from_local_storage_path(&path) {
            Some(config) => return Some(config),
            None => continue,
        }
    }

    None
}

fn find_legacy_local_storage_paths(root: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    collect_legacy_local_storage_paths(root, &mut paths);
    paths
}

fn collect_legacy_local_storage_paths(path: &Path, paths: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(path) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.file_name().and_then(|name| name.to_str()) == Some("localstorage.sqlite3") {
            paths.push(path);
            continue;
        }

        if path.is_dir() {
            collect_legacy_local_storage_paths(&path, paths);
        }
    }
}

fn legacy_hotkey_config_from_local_storage_path(path: &Path) -> Option<StoredHotkeyConfig> {
    let connection =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .ok()?;
    let value = connection
        .query_row(
            "SELECT value FROM ItemTable WHERE key = ?1",
            ["snaplingo-settings"],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .ok()?;

    legacy_hotkey_config_from_local_storage_value(&value)
}

fn legacy_hotkey_config_from_local_storage_value(value: &[u8]) -> Option<StoredHotkeyConfig> {
    let json = decode_local_storage_value(value)?;
    let settings = serde_json::from_str::<serde_json::Value>(&json).ok()?;
    let hotkeys = settings.get("state")?.get("hotkeys")?.clone();
    serde_json::from_value(hotkeys).ok()
}

fn decode_local_storage_value(value: &[u8]) -> Option<String> {
    if let Ok(text) = std::str::from_utf8(value) {
        if text.trim_start().starts_with('{') {
            return Some(text.to_string());
        }
    }

    if value.len() % 2 != 0 {
        return None;
    }

    let units = value
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect::<Vec<_>>();
    String::from_utf16(&units).ok()
}

fn default_hotkey_config() -> StoredHotkeyConfig {
    let mut config = StoredHotkeyConfig::default();
    for (category, action, hotkey) in STARTUP_HOTKEYS {
        if let Ok(category_hotkeys) = hotkey_category_mut(&mut config, category) {
            category_hotkeys.insert((*action).to_string(), (*hotkey).to_string());
        }
    }
    config
}

fn merge_saved_hotkeys(
    mut default_config: StoredHotkeyConfig,
    saved_config: StoredHotkeyConfig,
) -> StoredHotkeyConfig {
    merge_saved_category(
        &mut default_config.screenshot,
        &saved_config.screenshot,
        SCREENSHOT_CATEGORY,
    );
    merge_saved_category(
        &mut default_config.translation,
        &saved_config.translation,
        TRANSLATION_CATEGORY,
    );
    merge_saved_category(&mut default_config.ocr, &saved_config.ocr, OCR_CATEGORY);
    default_config
}

fn merge_saved_category(
    default_category: &mut HashMap<String, String>,
    saved_category: &HashMap<String, String>,
    category: &str,
) {
    for (action, hotkey) in saved_category {
        if !default_category.contains_key(action) {
            log::warn!(
                "Ignoring unknown saved hotkey action {}:{}",
                category,
                action
            );
            continue;
        }

        if let Err(err) = resolve_hotkey_accelerator(category, action, hotkey) {
            log::warn!(
                "Ignoring invalid saved hotkey {}:{}='{}': {}",
                category,
                action,
                hotkey,
                err
            );
            continue;
        }

        default_category.insert(action.clone(), hotkey.clone());
    }
}

fn startup_hotkeys_from_config(config: &StoredHotkeyConfig) -> Vec<StartupHotkey> {
    STARTUP_HOTKEYS
        .iter()
        .filter_map(|(category, action, _)| {
            hotkey_category(config, category)
                .and_then(|category_hotkeys| category_hotkeys.get(*action))
                .map(|hotkey| StartupHotkey {
                    category: (*category).to_string(),
                    action: (*action).to_string(),
                    hotkey: hotkey.clone(),
                })
        })
        .collect()
}

fn hotkey_category<'a>(
    config: &'a StoredHotkeyConfig,
    category: &str,
) -> Option<&'a HashMap<String, String>> {
    match category {
        SCREENSHOT_CATEGORY => Some(&config.screenshot),
        TRANSLATION_CATEGORY => Some(&config.translation),
        OCR_CATEGORY => Some(&config.ocr),
        _ => None,
    }
}

fn hotkey_category_mut<'a>(
    config: &'a mut StoredHotkeyConfig,
    category: &str,
) -> Result<&'a mut HashMap<String, String>> {
    match category {
        SCREENSHOT_CATEGORY => Ok(&mut config.screenshot),
        TRANSLATION_CATEGORY => Ok(&mut config.translation),
        OCR_CATEGORY => Ok(&mut config.ocr),
        _ => Err(crate::AppError::Other(format!(
            "Unknown hotkey category '{}'",
            category
        ))),
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
            category, action
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

    if should_register_hotkey_on_release(&category, &action) {
        return infrastructure::system::register_shortcut_on_release(app, accelerator, move || {
            trigger_hotkey_action(app_clone.clone(), category.clone(), action.clone());
        });
    }

    infrastructure::system::register_shortcut(app, accelerator, move || {
        trigger_hotkey_action(app_clone.clone(), category.clone(), action.clone());
    })
}

fn trigger_hotkey_action(app: tauri::AppHandle, category: String, action: String) {
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

fn should_register_hotkey_on_release(category: &str, action: &str) -> bool {
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
        legacy_hotkey_config_from_local_storage_value, resolve_hotkey_accelerator,
        save_hotkey_config, should_register_hotkey_on_release, startup_hotkeys_from_config_file,
        FILE_OCR_ACTION, OCR_CATEGORY, SCREENSHOT_ACTION, SCREENSHOT_CATEGORY,
        SCREENSHOT_COPY_ACTION, SCREENSHOT_CUSTOM_ACTION, SCREENSHOT_OCR_ACTION,
        SCREENSHOT_TRANSLATE_ACTION, SELECTION_TRANSLATE_ACTION, SHOW_OCR_WINDOW_ACTION,
        SILENT_SCREENSHOT_OCR_ACTION, TRANSLATION_CATEGORY,
    };
    use crate::infrastructure::storage::ConfigFile;
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

    #[test]
    fn startup_hotkeys_use_saved_selection_translate_shortcut() {
        let config_file = ConfigFile::new_temp();
        save_hotkey_config(
            &config_file,
            TRANSLATION_CATEGORY,
            SELECTION_TRANSLATE_ACTION,
            "⇧⌥D",
        )
        .unwrap();

        let startup_hotkeys = startup_hotkeys_from_config_file(&config_file);
        let saved = startup_hotkeys
            .iter()
            .find(|hotkey| {
                hotkey.category == TRANSLATION_CATEGORY
                    && hotkey.action == SELECTION_TRANSLATE_ACTION
            })
            .unwrap();

        assert_eq!(saved.hotkey, "⇧⌥D");
        assert_eq!(
            resolve_hotkey_accelerator(&saved.category, &saved.action, &saved.hotkey).unwrap(),
            Some("Shift+Alt+KeyD".to_string())
        );
    }

    #[test]
    fn startup_hotkeys_keep_unset_saved_shortcuts_unregistered() {
        let config_file = ConfigFile::new_temp();
        save_hotkey_config(
            &config_file,
            TRANSLATION_CATEGORY,
            SELECTION_TRANSLATE_ACTION,
            "未设置",
        )
        .unwrap();

        let startup_hotkeys = startup_hotkeys_from_config_file(&config_file);
        let saved = startup_hotkeys
            .iter()
            .find(|hotkey| {
                hotkey.category == TRANSLATION_CATEGORY
                    && hotkey.action == SELECTION_TRANSLATE_ACTION
            })
            .unwrap();

        assert_eq!(saved.hotkey, "未设置");
        assert_eq!(
            resolve_hotkey_accelerator(&saved.category, &saved.action, &saved.hotkey).unwrap(),
            None
        );
    }

    #[test]
    fn startup_hotkeys_ignore_invalid_saved_shortcuts() {
        let config_file = ConfigFile::new_temp();
        config_file
            .save(
                "hotkeys",
                &serde_json::json!({
                    "translation": {
                        "selection-translate": "⌘"
                    }
                }),
            )
            .unwrap();

        let startup_hotkeys = startup_hotkeys_from_config_file(&config_file);
        let saved = startup_hotkeys
            .iter()
            .find(|hotkey| {
                hotkey.category == TRANSLATION_CATEGORY
                    && hotkey.action == SELECTION_TRANSLATE_ACTION
            })
            .unwrap();

        assert_eq!(saved.hotkey, "⌥D");
    }

    #[test]
    fn reads_hotkeys_from_legacy_webkit_local_storage_value() {
        let json = r#"{"state":{"hotkeys":{"translation":{"selection-translate":"⇧⌥D"}}}}"#;
        let blob = json
            .encode_utf16()
            .flat_map(|unit| unit.to_le_bytes())
            .collect::<Vec<_>>();

        let config = legacy_hotkey_config_from_local_storage_value(&blob).unwrap();

        assert_eq!(
            config.translation.get(SELECTION_TRANSLATE_ACTION),
            Some(&"⇧⌥D".to_string())
        );
    }
}

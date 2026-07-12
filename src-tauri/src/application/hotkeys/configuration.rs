use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Deserialize;

use crate::application::hotkeys::HotkeyStore;
use crate::domain::hotkey_config::{
    default_hotkey_snapshot, hotkey_category_mut, validate_hotkey_action, HotkeySettingsSnapshot,
};
use crate::{AppError, Result};

const LEGACY_LOCAL_STORAGE_KEY: &str = "snaplingo-settings";

pub struct HotkeyConfiguration {
    store: Arc<dyn HotkeyStore>,
    legacy_local_storage_root: Option<PathBuf>,
}

impl HotkeyConfiguration {
    pub fn new(store: Arc<dyn HotkeyStore>) -> Self {
        Self::with_legacy_root(
            store,
            dirs::home_dir().map(|path| path.join("Library/WebKit/com.snaplingo.app")),
        )
    }

    pub(crate) fn with_legacy_root(
        store: Arc<dyn HotkeyStore>,
        legacy_local_storage_root: Option<PathBuf>,
    ) -> Self {
        Self {
            store,
            legacy_local_storage_root,
        }
    }

    pub fn snapshot(&self) -> Result<HotkeySettingsSnapshot> {
        match self.store.load_hotkeys() {
            Ok(snapshot) => Ok(normalized_snapshot(snapshot)),
            Err(AppError::Config(_)) => {
                if let Some(snapshot) = self.migrate_legacy_hotkeys()? {
                    return Ok(snapshot);
                }

                Ok(default_hotkey_snapshot())
            }
            Err(err) => Err(err),
        }
    }

    pub fn update_hotkey(
        &self,
        category: &str,
        action: &str,
        hotkey: &str,
    ) -> Result<HotkeySettingsSnapshot> {
        validate_hotkey_action(category, action)?;
        crate::startup_shortcuts::display_hotkey_to_accelerator(hotkey)?;

        let mut snapshot = self.snapshot()?;
        hotkey_category_mut(&mut snapshot, category)?
            .insert(action.to_string(), hotkey.to_string());
        self.save_snapshot(snapshot)
    }

    fn save_snapshot(&self, snapshot: HotkeySettingsSnapshot) -> Result<HotkeySettingsSnapshot> {
        let snapshot = normalized_snapshot(snapshot);
        self.store.save_hotkeys(&snapshot)?;
        Ok(snapshot)
    }

    fn migrate_legacy_hotkeys(&self) -> Result<Option<HotkeySettingsSnapshot>> {
        let Some(snapshot) = self.load_legacy_hotkeys() else {
            return Ok(None);
        };

        self.save_snapshot(snapshot).map(Some)
    }

    fn load_legacy_hotkeys(&self) -> Option<HotkeySettingsSnapshot> {
        let root = self.legacy_local_storage_root.as_deref()?;

        for path in find_legacy_local_storage_paths(root) {
            if let Some(snapshot) = legacy_hotkey_config_from_local_storage_path(&path) {
                return Some(snapshot);
            }
        }

        None
    }
}

fn normalized_snapshot(snapshot: HotkeySettingsSnapshot) -> HotkeySettingsSnapshot {
    merge_saved_hotkeys(default_hotkey_snapshot(), snapshot)
}

fn merge_saved_hotkeys(
    mut default_config: HotkeySettingsSnapshot,
    saved_config: HotkeySettingsSnapshot,
) -> HotkeySettingsSnapshot {
    merge_saved_category(
        &mut default_config.screenshot,
        &saved_config.screenshot,
        crate::domain::hotkey_config::SCREENSHOT_CATEGORY,
    );
    merge_saved_category(
        &mut default_config.translation,
        &saved_config.translation,
        crate::domain::hotkey_config::TRANSLATION_CATEGORY,
    );
    merge_saved_category(
        &mut default_config.ocr,
        &saved_config.ocr,
        crate::domain::hotkey_config::OCR_CATEGORY,
    );
    default_config
}

fn merge_saved_category(
    default_category: &mut std::collections::HashMap<String, String>,
    saved_category: &std::collections::HashMap<String, String>,
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

        if let Err(err) = validate_saved_hotkey(category, action, hotkey) {
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

fn validate_saved_hotkey(category: &str, action: &str, hotkey: &str) -> Result<()> {
    validate_hotkey_action(category, action)?;
    crate::startup_shortcuts::display_hotkey_to_accelerator(hotkey)?;
    Ok(())
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct LegacyFrontendSettingsDocument {
    state: LegacyFrontendHotkeyState,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct LegacyFrontendHotkeyState {
    hotkeys: Option<HotkeySettingsSnapshot>,
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

fn legacy_hotkey_config_from_local_storage_path(path: &Path) -> Option<HotkeySettingsSnapshot> {
    let connection =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .ok()?;
    let value = connection
        .query_row(
            "SELECT value FROM ItemTable WHERE key = ?1",
            [LEGACY_LOCAL_STORAGE_KEY],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .ok()?;

    legacy_hotkey_config_from_local_storage_value(&value)
}

fn legacy_hotkey_config_from_local_storage_value(value: &[u8]) -> Option<HotkeySettingsSnapshot> {
    let json = decode_local_storage_value(value)?;
    serde_json::from_str::<LegacyFrontendSettingsDocument>(&json)
        .ok()?
        .state
        .hotkeys
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

#[cfg(test)]
mod hotkey_configuration_tests {
    use std::path::Path;
    use std::sync::Arc;

    use rusqlite::Connection;
    use tempfile::tempdir;

    use super::HotkeyConfiguration;
    use crate::domain::hotkey_config::{
        default_hotkey_snapshot, HotkeySettingsSnapshot, SELECTION_TRANSLATE_ACTION,
        TRANSLATION_CATEGORY,
    };
    use crate::infrastructure::storage::ConfigFile;

    #[test]
    fn hotkey_configuration_snapshot_returns_defaults_when_no_backend_config_exists() {
        let config_file = Arc::new(ConfigFile::new_temp());
        let configuration = HotkeyConfiguration::with_legacy_root(config_file, None);

        let snapshot = configuration.snapshot().unwrap();

        assert_eq!(snapshot, default_hotkey_snapshot());
    }

    #[test]
    fn hotkey_configuration_update_preserves_other_actions() {
        let config_file = Arc::new(ConfigFile::new_temp());
        let configuration = HotkeyConfiguration::with_legacy_root(config_file.clone(), None);

        let updated = configuration
            .update_hotkey(TRANSLATION_CATEGORY, SELECTION_TRANSLATE_ACTION, "⇧⌥D")
            .unwrap();

        assert_eq!(
            updated.translation.get(SELECTION_TRANSLATE_ACTION),
            Some(&"⇧⌥D".to_string())
        );
        assert_eq!(updated.screenshot, default_hotkey_snapshot().screenshot);
        assert_eq!(updated.ocr, default_hotkey_snapshot().ocr);
        assert_eq!(
            updated.translation.get("screenshot-translate"),
            default_hotkey_snapshot()
                .translation
                .get("screenshot-translate")
        );

        let persisted: HotkeySettingsSnapshot = config_file.load("hotkeys").unwrap();
        assert_eq!(persisted, updated);
    }

    #[test]
    fn hotkey_configuration_invalid_saved_shortcuts_fall_back_to_defaults() {
        let config_file = Arc::new(ConfigFile::new_temp());
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
        let configuration = HotkeyConfiguration::with_legacy_root(config_file, None);

        let snapshot = configuration.snapshot().unwrap();

        assert_eq!(
            snapshot.translation.get(SELECTION_TRANSLATE_ACTION),
            default_hotkey_snapshot()
                .translation
                .get(SELECTION_TRANSLATE_ACTION)
        );
    }

    #[test]
    fn hotkey_configuration_ignores_unknown_saved_actions() {
        let config_file = Arc::new(ConfigFile::new_temp());
        config_file
            .save(
                "hotkeys",
                &serde_json::json!({
                    "translation": {
                        "selection-translate": "⇧⌥D",
                        "surprise": "⌘U"
                    }
                }),
            )
            .unwrap();
        let configuration = HotkeyConfiguration::with_legacy_root(config_file, None);

        let snapshot = configuration.snapshot().unwrap();

        assert_eq!(
            snapshot.translation.get(SELECTION_TRANSLATE_ACTION),
            Some(&"⇧⌥D".to_string())
        );
        assert!(!snapshot.translation.contains_key("surprise"));
    }

    #[test]
    fn hotkey_configuration_migrates_legacy_webkit_hotkeys_once() {
        let config_file = Arc::new(ConfigFile::new_temp());
        let legacy_root = tempdir().unwrap();
        write_legacy_local_storage(
            legacy_root.path(),
            serde_json::json!({
                "state": {
                    "hotkeys": {
                        "translation": {
                            "selection-translate": "⇧⌥D"
                        }
                    }
                }
            }),
        );
        let configuration = HotkeyConfiguration::with_legacy_root(
            config_file.clone(),
            Some(legacy_root.path().to_path_buf()),
        );

        let first_snapshot = configuration.snapshot().unwrap();
        assert_eq!(
            first_snapshot.translation.get(SELECTION_TRANSLATE_ACTION),
            Some(&"⇧⌥D".to_string())
        );
        let persisted: HotkeySettingsSnapshot = config_file.load("hotkeys").unwrap();
        assert_eq!(persisted, first_snapshot);

        write_legacy_local_storage(
            legacy_root.path(),
            serde_json::json!({
                "state": {
                    "hotkeys": {
                        "translation": {
                            "selection-translate": "⌘D"
                        }
                    }
                }
            }),
        );

        let second_snapshot = configuration.snapshot().unwrap();
        assert_eq!(second_snapshot, first_snapshot);
    }

    #[test]
    fn hotkey_configuration_ignores_legacy_settings_without_hotkeys() {
        let config_file = Arc::new(ConfigFile::new_temp());
        let legacy_root = tempdir().unwrap();
        write_legacy_local_storage(
            legacy_root.path(),
            serde_json::json!({
                "state": {
                    "language": "en",
                    "theme": "dark",
                    "startOnBoot": true
                }
            }),
        );
        let configuration = HotkeyConfiguration::with_legacy_root(
            config_file.clone(),
            Some(legacy_root.path().to_path_buf()),
        );

        let snapshot = configuration.snapshot().unwrap();

        assert_eq!(snapshot, default_hotkey_snapshot());
        assert!(config_file
            .load::<HotkeySettingsSnapshot>("hotkeys")
            .is_err());
    }

    fn write_legacy_local_storage(root: &Path, json: serde_json::Value) {
        let storage_dir = root.join("https_snaplingo_0.localstorage");
        std::fs::create_dir_all(&storage_dir).unwrap();
        let storage_path = storage_dir.join("localstorage.sqlite3");

        if storage_path.exists() {
            std::fs::remove_file(&storage_path).unwrap();
        }

        let connection = Connection::open(&storage_path).unwrap();
        connection
            .execute(
                "CREATE TABLE ItemTable (key TEXT PRIMARY KEY, value BLOB NOT NULL)",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO ItemTable (key, value) VALUES (?1, ?2)",
                (
                    "snaplingo-settings",
                    serde_json::to_string(&json).unwrap().into_bytes(),
                ),
            )
            .unwrap();
    }
}

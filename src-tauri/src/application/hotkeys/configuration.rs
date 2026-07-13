use std::sync::{Arc, Mutex};

use crate::application::hotkeys::{display_hotkey_to_accelerator, HotkeyStore};
use crate::domain::hotkey_config::{
    default_hotkey_snapshot, hotkey_category_mut, validate_hotkey_action, HotkeySettingsSnapshot,
};
use crate::{AppError, Result};

pub struct HotkeyConfiguration {
    store: Arc<dyn HotkeyStore>,
    update_lock: Mutex<()>,
}

impl HotkeyConfiguration {
    pub fn new(store: Arc<dyn HotkeyStore>) -> Self {
        Self {
            store,
            update_lock: Mutex::new(()),
        }
    }

    pub fn snapshot(&self) -> Result<HotkeySettingsSnapshot> {
        match self.store.load_hotkeys() {
            Ok(snapshot) => Ok(normalized_snapshot(snapshot)),
            Err(AppError::Config(_)) => Ok(default_hotkey_snapshot()),
            Err(err) => Err(err),
        }
    }

    pub fn update_hotkey(
        &self,
        category: &str,
        action: &str,
        hotkey: &str,
    ) -> Result<HotkeySettingsSnapshot> {
        let _guard = self.update_lock.lock().unwrap();
        validate_hotkey_action(category, action)?;
        display_hotkey_to_accelerator(hotkey)?;

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
    display_hotkey_to_accelerator(hotkey)?;
    Ok(())
}

#[cfg(test)]
mod hotkey_configuration_tests {
    use std::sync::Arc;

    use super::HotkeyConfiguration;
    use crate::application::hotkeys::HotkeyStore;
    use crate::domain::hotkey_config::{
        default_hotkey_snapshot, HotkeySettingsSnapshot, SELECTION_TRANSLATE_ACTION,
        TRANSLATION_CATEGORY,
    };
    use crate::infrastructure::storage::SqliteConfigStore;

    #[test]
    fn snapshot_returns_defaults_when_no_configuration_exists() {
        let store = Arc::new(SqliteConfigStore::new_in_memory());
        let configuration = HotkeyConfiguration::new(store);

        assert_eq!(configuration.snapshot().unwrap(), default_hotkey_snapshot());
    }

    #[test]
    fn update_preserves_other_actions() {
        let store = Arc::new(SqliteConfigStore::new_in_memory());
        let configuration = HotkeyConfiguration::new(store.clone());

        let updated = configuration
            .update_hotkey(TRANSLATION_CATEGORY, SELECTION_TRANSLATE_ACTION, "⇧⌥D")
            .unwrap();

        assert_eq!(
            updated.translation.get(SELECTION_TRANSLATE_ACTION),
            Some(&"⇧⌥D".to_string())
        );
        assert_eq!(updated.screenshot, default_hotkey_snapshot().screenshot);
        assert_eq!(updated.ocr, default_hotkey_snapshot().ocr);
        assert_eq!(store.load_hotkeys().unwrap(), updated);
    }

    #[test]
    fn invalid_saved_shortcuts_fall_back_to_defaults() {
        let store = Arc::new(SqliteConfigStore::new_in_memory());
        store
            .save_hotkeys(&HotkeySettingsSnapshot {
                translation: std::collections::HashMap::from([(
                    SELECTION_TRANSLATE_ACTION.to_string(),
                    "⌘".to_string(),
                )]),
                ..HotkeySettingsSnapshot::default()
            })
            .unwrap();
        let configuration = HotkeyConfiguration::new(store);

        assert_eq!(
            configuration
                .snapshot()
                .unwrap()
                .translation
                .get(SELECTION_TRANSLATE_ACTION),
            default_hotkey_snapshot()
                .translation
                .get(SELECTION_TRANSLATE_ACTION)
        );
    }

    #[test]
    fn unknown_saved_actions_are_ignored() {
        let store = Arc::new(SqliteConfigStore::new_in_memory());
        store
            .save_hotkeys(&HotkeySettingsSnapshot {
                translation: std::collections::HashMap::from([
                    (SELECTION_TRANSLATE_ACTION.to_string(), "⇧⌥D".to_string()),
                    ("surprise".to_string(), "⌘U".to_string()),
                ]),
                ..HotkeySettingsSnapshot::default()
            })
            .unwrap();
        let configuration = HotkeyConfiguration::new(store);

        let snapshot = configuration.snapshot().unwrap();
        assert_eq!(
            snapshot.translation.get(SELECTION_TRANSLATE_ACTION),
            Some(&"⇧⌥D".to_string())
        );
        assert!(!snapshot.translation.contains_key("surprise"));
    }
}

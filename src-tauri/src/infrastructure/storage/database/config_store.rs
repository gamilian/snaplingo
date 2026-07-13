use std::sync::Arc;

use chrono::Utc;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::application::hotkeys::HotkeyStore;
use crate::application::providers::{
    CustomTranslationProviderDef, ProviderConfigStore, TranslationPromptStrategyConfig,
};
use crate::application::settings::SettingsStore;
use crate::domain::hotkey_config::HotkeySettingsSnapshot;
use crate::domain::SettingsSnapshot;
use crate::{AppError, Result};

use super::Database;

const SETTINGS_NAMESPACE: &str = "settings";
const HOTKEYS_NAMESPACE: &str = "hotkeys";
const CUSTOM_TRANSLATION_PROVIDERS_NAMESPACE: &str = "custom_translation_providers";
const ACTIVE_TRANSLATION_PROVIDERS_NAMESPACE: &str = "active_translation_providers";
const ACTIVE_OCR_PROVIDER_NAMESPACE: &str = "active_ocr_provider";
const TRANSLATION_PROMPT_STRATEGIES_NAMESPACE: &str = "translation_prompt_strategies";
const PAYLOAD_VERSION: i32 = 1;

/// SQLite implementation of the existing configuration ports.
///
/// The ports remain section-oriented during the first persistence migration so runtime
/// behaviour stays stable while the backing store moves from JSON to SQLite.
pub struct SqliteConfigStore {
    database: Arc<Database>,
}

impl SqliteConfigStore {
    pub fn new(database: Arc<Database>) -> Self {
        Self { database }
    }

    pub fn load_settings(&self) -> Result<SettingsSnapshot> {
        self.load_namespace(SETTINGS_NAMESPACE)
    }

    pub fn save_settings(&self, snapshot: &SettingsSnapshot) -> Result<()> {
        self.save_namespace(SETTINGS_NAMESPACE, snapshot)
    }

    #[cfg(test)]
    pub fn new_in_memory() -> Self {
        Self::new(Arc::new(
            Database::in_memory().expect("initialize test database"),
        ))
    }

    #[cfg(test)]
    pub fn new_temp() -> Self {
        Self::new_in_memory()
    }

    fn save_namespace<T: Serialize>(&self, namespace: &str, value: &T) -> Result<()> {
        let payload = serde_json::to_string(value)?;
        let updated_at = Utc::now().timestamp_millis();

        self.database.with_transaction(|transaction| {
            transaction.execute(
                "INSERT INTO settings (namespace, payload_version, payload_json, revision, updated_at)
                 VALUES (?1, ?2, ?3, 1, ?4)
                 ON CONFLICT(namespace) DO UPDATE SET
                    payload_version = excluded.payload_version,
                    payload_json = excluded.payload_json,
                    revision = settings.revision + 1,
                    updated_at = excluded.updated_at",
                params![namespace, PAYLOAD_VERSION, payload, updated_at],
            )?;
            Ok(())
        })
    }

    fn load_namespace<T: for<'de> Deserialize<'de>>(&self, namespace: &str) -> Result<T> {
        let payload = self.database.with_connection(|connection| {
            connection
                .query_row(
                    "SELECT payload_json FROM settings WHERE namespace = ?1",
                    [namespace],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(Into::into)
        })?;

        let payload = payload.ok_or_else(|| {
            AppError::Config(format!("Configuration namespace '{}' not found", namespace))
        })?;
        Ok(serde_json::from_str(&payload)?)
    }

    #[cfg(test)]
    pub fn save<T: Serialize>(&self, namespace: &str, value: &T) -> Result<()> {
        self.save_namespace(namespace, value)
    }

    #[cfg(test)]
    pub fn load<T: for<'de> Deserialize<'de>>(&self, namespace: &str) -> Result<T> {
        self.load_namespace(namespace)
    }
}

impl SettingsStore for SqliteConfigStore {
    fn load_settings(&self) -> Result<SettingsSnapshot> {
        SqliteConfigStore::load_settings(self)
    }

    fn save_settings(&self, snapshot: &SettingsSnapshot) -> Result<()> {
        SqliteConfigStore::save_settings(self, snapshot)
    }
}

impl HotkeyStore for SqliteConfigStore {
    fn load_hotkeys(&self) -> Result<HotkeySettingsSnapshot> {
        self.load_namespace(HOTKEYS_NAMESPACE)
    }

    fn save_hotkeys(&self, snapshot: &HotkeySettingsSnapshot) -> Result<()> {
        self.save_namespace(HOTKEYS_NAMESPACE, snapshot)
    }
}

impl ProviderConfigStore for SqliteConfigStore {
    fn load_custom_translation_providers(&self) -> Result<Vec<CustomTranslationProviderDef>> {
        self.load_namespace(CUSTOM_TRANSLATION_PROVIDERS_NAMESPACE)
    }

    fn save_custom_translation_providers(
        &self,
        providers: &[CustomTranslationProviderDef],
    ) -> Result<()> {
        self.save_namespace(CUSTOM_TRANSLATION_PROVIDERS_NAMESPACE, &providers)
    }

    fn load_active_translation_providers(&self) -> Result<Vec<String>> {
        self.load_namespace(ACTIVE_TRANSLATION_PROVIDERS_NAMESPACE)
    }

    fn save_active_translation_providers(&self, provider_ids: &[String]) -> Result<()> {
        self.save_namespace(ACTIVE_TRANSLATION_PROVIDERS_NAMESPACE, &provider_ids)
    }

    fn load_active_ocr_provider(&self) -> Result<String> {
        self.load_namespace(ACTIVE_OCR_PROVIDER_NAMESPACE)
    }

    fn save_active_ocr_provider(&self, provider_id: &str) -> Result<()> {
        self.save_namespace(ACTIVE_OCR_PROVIDER_NAMESPACE, &provider_id)
    }

    fn load_translation_prompt_strategies(&self) -> Result<TranslationPromptStrategyConfig> {
        self.load_namespace(TRANSLATION_PROMPT_STRATEGIES_NAMESPACE)
    }

    fn save_translation_prompt_strategies(
        &self,
        config: &TranslationPromptStrategyConfig,
    ) -> Result<()> {
        self.save_namespace(TRANSLATION_PROMPT_STRATEGIES_NAMESPACE, config)
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::SettingsSnapshot;
    use crate::error::AppError;

    use super::SqliteConfigStore;

    #[test]
    fn stores_settings_as_a_versioned_json_document() {
        let store = SqliteConfigStore::new_in_memory();
        let mut snapshot = SettingsSnapshot::default();
        snapshot.general.theme = "dark".to_string();

        store.save_settings(&snapshot).unwrap();
        assert_eq!(store.load_settings().unwrap(), snapshot);
    }

    #[test]
    fn missing_namespace_uses_the_existing_configuration_error_contract() {
        let store = SqliteConfigStore::new_in_memory();
        let error = store.load_settings().unwrap_err();

        assert!(matches!(error, AppError::Config(_)));
    }
}

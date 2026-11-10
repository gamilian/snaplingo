use std::sync::Arc;

use chrono::Utc;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::{AppError, Result};
use snaplingo_core::hotkeys::model::HotkeySettingsSnapshot;
use snaplingo_core::hotkeys::HotkeyStore;
use snaplingo_core::providers::CustomTranslationProviderDef;
use snaplingo_core::providers::ProviderConfigStore;
use snaplingo_core::providers::TranslationPromptStrategyConfig;
use snaplingo_core::settings::model::SettingsSnapshot;
use snaplingo_core::settings::SettingsStore;

use super::Database;

const SETTINGS_NAMESPACE: &str = "settings";
const HOTKEYS_NAMESPACE: &str = "hotkeys";
const CUSTOM_TRANSLATION_PROVIDERS_NAMESPACE: &str = "custom_translation_providers";
const ACTIVE_TRANSLATION_PROVIDERS_NAMESPACE: &str = "active_translation_providers";
const INACTIVE_TRANSLATION_PROVIDERS_NAMESPACE: &str = "inactive_translation_providers";
const ACTIVE_OCR_PROVIDER_NAMESPACE: &str = "active_ocr_provider";
const TRANSLATION_PROMPT_STRATEGIES_NAMESPACE: &str = "translation_prompt_strategies";
const SERVICES_NAVIGATION_NAMESPACE: &str = "services_navigation";
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

    pub fn load_services_route(&self) -> Result<Option<String>> {
        self.load_namespace_optional(SERVICES_NAVIGATION_NAMESPACE)
    }

    pub fn save_services_route(&self, route: &str) -> Result<()> {
        self.save_namespace(SERVICES_NAVIGATION_NAMESPACE, &route)
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn new_in_memory() -> Self {
        Self::new(Arc::new(
            Database::in_memory().expect("initialize test database"),
        ))
    }

    #[cfg(any(test, feature = "test-support"))]
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
            ).map_err(snaplingo_core::AppError::database)?;
            Ok(())
        })
    }

    fn load_namespace<T: for<'de> Deserialize<'de>>(&self, namespace: &str) -> Result<T> {
        self.load_namespace_optional(namespace)?.ok_or_else(|| {
            AppError::Config(format!("Configuration namespace '{}' not found", namespace))
        })
    }

    fn load_namespace_optional<T: for<'de> Deserialize<'de>>(
        &self,
        namespace: &str,
    ) -> Result<Option<T>> {
        let payload = self.database.with_connection(|connection| {
            connection
                .query_row(
                    "SELECT payload_json FROM settings WHERE namespace = ?1",
                    [namespace],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(snaplingo_core::AppError::database)
        })?;

        payload
            .map(|payload| serde_json::from_str(&payload).map_err(Into::into))
            .transpose()
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn save<T: Serialize>(&self, namespace: &str, value: &T) -> Result<()> {
        self.save_namespace(namespace, value)
    }

    #[cfg(any(test, feature = "test-support"))]
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

    fn load_inactive_translation_providers(&self) -> Result<Vec<String>> {
        self.load_namespace(INACTIVE_TRANSLATION_PROVIDERS_NAMESPACE)
    }

    fn save_inactive_translation_providers(&self, provider_ids: &[String]) -> Result<()> {
        self.save_namespace(INACTIVE_TRANSLATION_PROVIDERS_NAMESPACE, &provider_ids)
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
    use snaplingo_core::error::AppError;
    use snaplingo_core::settings::model::SettingsSnapshot;

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

    #[test]
    fn services_route_survives_a_real_database_reopen_without_changing_settings() {
        let directory = std::env::temp_dir().join(format!(
            "snaplingo-services-route-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("settings.sqlite3");
        let settings = SettingsSnapshot::default();
        {
            let database = std::sync::Arc::new(super::Database::open(&path).unwrap());
            let store = SqliteConfigStore::new(database);
            assert_eq!(store.load_services_route().unwrap(), None);
            store.save_settings(&settings).unwrap();
            store.save_services_route("translation").unwrap();
            store.save_services_route("tts").unwrap();
        }
        {
            let store =
                SqliteConfigStore::new(std::sync::Arc::new(super::Database::open(&path).unwrap()));
            assert_eq!(store.load_services_route().unwrap().as_deref(), Some("tts"));
            assert_eq!(store.load_settings().unwrap(), settings);
        }
        std::fs::remove_dir_all(directory).unwrap();
    }
}

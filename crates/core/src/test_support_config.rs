//! In-memory adapter for portable workflow tests; no persistence implementation.
use crate::hotkeys::model::HotkeySettingsSnapshot;
use crate::hotkeys::HotkeyStore;
use crate::providers::CustomTranslationProviderDef;
use crate::providers::ProviderConfigStore;
use crate::providers::TranslationPromptStrategyConfig;
use crate::settings::model::SettingsSnapshot;
use crate::settings::SettingsStore;
use crate::Result;
use serde::{de::DeserializeOwned, Serialize};
use std::{collections::HashMap, sync::Mutex};
#[derive(Default)]
pub(crate) struct MemoryConfigStore(Mutex<HashMap<&'static str, serde_json::Value>>);
impl MemoryConfigStore {
    pub fn new_in_memory() -> Self {
        Self::default()
    }
    pub fn new_temp() -> Self {
        Self::default()
    }
    fn read<T: DeserializeOwned>(&self, key: &'static str) -> Result<T> {
        let values = self.0.lock().unwrap();
        let value = values.get(key).ok_or_else(|| {
            crate::AppError::Config(format!("Configuration namespace '{key}' not found"))
        })?;
        Ok(serde_json::from_value(value.clone())?)
    }
    fn write<T: Serialize + ?Sized>(&self, key: &'static str, value: &T) -> Result<()> {
        self.0
            .lock()
            .unwrap()
            .insert(key, serde_json::to_value(value)?);
        Ok(())
    }
    pub fn save<T: Serialize + ?Sized>(&self, key: &'static str, value: &T) -> Result<()> {
        self.write(key, value)
    }
    pub fn load<T: DeserializeOwned>(&self, key: &'static str) -> Result<T> {
        self.read(key)
    }
    pub fn load_settings(&self) -> Result<SettingsSnapshot> {
        self.read("settings")
    }
    pub fn save_settings(&self, value: &SettingsSnapshot) -> Result<()> {
        self.write("settings", value)
    }
}
impl SettingsStore for MemoryConfigStore {
    fn load_settings(&self) -> Result<SettingsSnapshot> {
        self.load_settings()
    }
    fn save_settings(&self, value: &SettingsSnapshot) -> Result<()> {
        self.save_settings(value)
    }
}
impl HotkeyStore for MemoryConfigStore {
    fn load_hotkeys(&self) -> Result<HotkeySettingsSnapshot> {
        self.read("hotkeys")
    }
    fn save_hotkeys(&self, value: &HotkeySettingsSnapshot) -> Result<()> {
        self.write("hotkeys", value)
    }
}
impl ProviderConfigStore for MemoryConfigStore {
    fn load_custom_translation_providers(&self) -> Result<Vec<CustomTranslationProviderDef>> {
        self.read("custom_translation_providers")
    }
    fn save_custom_translation_providers(
        &self,
        value: &[CustomTranslationProviderDef],
    ) -> Result<()> {
        self.write("custom_translation_providers", value)
    }
    fn load_active_translation_providers(&self) -> Result<Vec<String>> {
        self.read("active_translation_providers")
    }
    fn save_active_translation_providers(&self, value: &[String]) -> Result<()> {
        self.write("active_translation_providers", value)
    }
    fn load_inactive_translation_providers(&self) -> Result<Vec<String>> {
        self.read("inactive_translation_providers")
    }
    fn save_inactive_translation_providers(&self, value: &[String]) -> Result<()> {
        self.write("inactive_translation_providers", value)
    }
    fn load_active_ocr_provider(&self) -> Result<String> {
        self.read("active_ocr_provider")
    }
    fn save_active_ocr_provider(&self, value: &str) -> Result<()> {
        self.write("active_ocr_provider", value)
    }
    fn load_translation_prompt_strategies(&self) -> Result<TranslationPromptStrategyConfig> {
        self.read("translation_prompt_strategies")
    }
    fn save_translation_prompt_strategies(
        &self,
        value: &TranslationPromptStrategyConfig,
    ) -> Result<()> {
        self.write("translation_prompt_strategies", value)
    }
}

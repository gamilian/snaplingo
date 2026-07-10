use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Deserialize;

use crate::application::capture::configured_capture_save_dir;
use crate::domain::{GeneralSettings, ScreenshotSettings, SettingsSnapshot, TranslationSettings};
use crate::infrastructure::storage::ConfigFile;
use crate::{AppError, Result};

const SETTINGS_CONFIG_KEY: &str = "settings";
const LEGACY_LOCAL_STORAGE_KEY: &str = "snaplingo-settings";

pub struct SettingsConfiguration {
    config_file: Arc<ConfigFile>,
    home_dir: Option<PathBuf>,
    default_screenshot_save_dir: PathBuf,
    legacy_local_storage_root: Option<PathBuf>,
}

impl SettingsConfiguration {
    pub fn new(config_file: Arc<ConfigFile>) -> Self {
        let home_dir = dirs::home_dir();

        Self::with_paths(
            config_file,
            home_dir.clone(),
            default_screenshot_save_dir(),
            home_dir.map(|path| path.join("Library/WebKit/com.snaplingo.app")),
        )
    }

    pub(crate) fn with_paths(
        config_file: Arc<ConfigFile>,
        home_dir: Option<PathBuf>,
        default_screenshot_save_dir: PathBuf,
        legacy_local_storage_root: Option<PathBuf>,
    ) -> Self {
        Self {
            config_file,
            home_dir,
            default_screenshot_save_dir,
            legacy_local_storage_root,
        }
    }

    pub fn snapshot(&self) -> Result<SettingsSnapshot> {
        match self
            .config_file
            .load::<SettingsSnapshot>(SETTINGS_CONFIG_KEY)
        {
            Ok(snapshot) => Ok(self.normalized_snapshot(snapshot)),
            Err(AppError::Config(_)) => {
                if let Some(snapshot) = self.migrate_legacy_durable_settings()? {
                    return Ok(snapshot);
                }

                Ok(self.default_snapshot())
            }
            Err(err) => Err(err),
        }
    }

    pub fn update_general(&self, input: GeneralSettings) -> Result<SettingsSnapshot> {
        let mut snapshot = self.snapshot()?;
        snapshot.general = input;
        self.save_snapshot(snapshot)
    }

    pub fn update_screenshot(&self, input: ScreenshotSettings) -> Result<SettingsSnapshot> {
        let mut snapshot = self.snapshot()?;
        snapshot.screenshot = input;
        self.save_snapshot(snapshot)
    }

    pub fn update_translation(&self, input: TranslationSettings) -> Result<SettingsSnapshot> {
        let mut snapshot = self.snapshot()?;
        snapshot.translation = input;
        self.save_snapshot(snapshot)
    }

    fn save_snapshot(&self, snapshot: SettingsSnapshot) -> Result<SettingsSnapshot> {
        let snapshot = self.normalized_snapshot(snapshot);
        self.config_file.save(SETTINGS_CONFIG_KEY, &snapshot)?;
        Ok(snapshot)
    }

    fn migrate_legacy_durable_settings(&self) -> Result<Option<SettingsSnapshot>> {
        let Some(settings) = self.load_legacy_durable_settings() else {
            return Ok(None);
        };

        let mut snapshot = self.default_snapshot();

        if let Some(language) = settings.language {
            snapshot.general.language = language;
        }
        if let Some(theme) = settings.theme {
            snapshot.general.theme = theme;
        }
        if let Some(start_on_boot) = settings.start_on_boot {
            snapshot.general.start_on_boot = start_on_boot;
        }

        if let Some(save_path) = settings.screenshot_save_path {
            snapshot.screenshot.save_path = save_path;
        }
        if let Some(format) = settings.screenshot_format {
            snapshot.screenshot.format = format;
        }
        if let Some(quality) = settings.screenshot_quality {
            snapshot.screenshot.quality = quality;
        }

        if let Some(default_source_lang) = settings.default_source_lang {
            snapshot.translation.default_source_lang = default_source_lang;
        }
        if let Some(default_target_lang) = settings.default_target_lang {
            snapshot.translation.default_target_lang = default_target_lang;
        }

        self.save_snapshot(snapshot).map(Some)
    }

    fn load_legacy_durable_settings(&self) -> Option<LegacyDurableSettings> {
        let root = self.legacy_local_storage_root.as_deref()?;

        for path in find_legacy_local_storage_paths(root) {
            if let Some(settings) = legacy_durable_settings_from_local_storage_path(&path) {
                return Some(settings);
            }
        }

        None
    }

    fn default_snapshot(&self) -> SettingsSnapshot {
        let mut snapshot = SettingsSnapshot::default();
        snapshot.screenshot.save_path = self
            .default_screenshot_save_dir
            .to_string_lossy()
            .to_string();
        snapshot
    }

    fn normalized_snapshot(&self, mut snapshot: SettingsSnapshot) -> SettingsSnapshot {
        snapshot.screenshot.save_path =
            self.normalize_screenshot_save_path(&snapshot.screenshot.save_path);
        snapshot
    }

    fn normalize_screenshot_save_path(&self, value: &str) -> String {
        let trimmed = value.trim();

        if trimmed.is_empty() {
            return self
                .default_screenshot_save_dir
                .to_string_lossy()
                .to_string();
        }

        if let Some(home_dir) = self.home_dir.as_deref() {
            return configured_capture_save_dir(trimmed, home_dir)
                .to_string_lossy()
                .to_string();
        }

        PathBuf::from(trimmed).to_string_lossy().to_string()
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct LegacyFrontendSettingsDocument {
    state: LegacyDurableSettings,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct LegacyDurableSettings {
    language: Option<String>,
    theme: Option<String>,
    start_on_boot: Option<bool>,
    screenshot_save_path: Option<String>,
    screenshot_format: Option<String>,
    screenshot_quality: Option<u8>,
    default_source_lang: Option<String>,
    default_target_lang: Option<String>,
}

impl LegacyDurableSettings {
    fn has_any_value(&self) -> bool {
        self.language.is_some()
            || self.theme.is_some()
            || self.start_on_boot.is_some()
            || self.screenshot_save_path.is_some()
            || self.screenshot_format.is_some()
            || self.screenshot_quality.is_some()
            || self.default_source_lang.is_some()
            || self.default_target_lang.is_some()
    }
}

fn default_screenshot_save_dir() -> PathBuf {
    dirs::picture_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(std::env::temp_dir)
        .join("SnapLingo")
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

fn legacy_durable_settings_from_local_storage_path(path: &Path) -> Option<LegacyDurableSettings> {
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

    legacy_durable_settings_from_local_storage_value(&value)
}

fn legacy_durable_settings_from_local_storage_value(value: &[u8]) -> Option<LegacyDurableSettings> {
    let json = decode_local_storage_value(value)?;
    let settings = serde_json::from_str::<LegacyFrontendSettingsDocument>(&json)
        .ok()?
        .state;

    settings.has_any_value().then_some(settings)
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
mod settings_configuration_tests {
    use std::path::Path;
    use std::sync::Arc;

    use rusqlite::Connection;
    use tempfile::tempdir;

    use super::SettingsConfiguration;
    use crate::domain::{
        GeneralSettings, ScreenshotSettings, SettingsSnapshot, TranslationSettings,
    };
    use crate::infrastructure::storage::ConfigFile;

    #[test]
    fn loads_merged_snapshot_when_config_file_is_empty() {
        let config_file = Arc::new(ConfigFile::new_temp());
        let home_dir = tempdir().unwrap();
        let default_save_dir = home_dir.path().join("Snapshots");

        let configuration = SettingsConfiguration::with_paths(
            config_file,
            Some(home_dir.path().to_path_buf()),
            default_save_dir.clone(),
            None,
        );

        let snapshot = configuration.snapshot().unwrap();

        assert_eq!(snapshot.general, GeneralSettings::default());
        assert_eq!(
            snapshot.screenshot,
            ScreenshotSettings {
                save_path: default_save_dir.to_string_lossy().to_string(),
                ..ScreenshotSettings::default()
            }
        );
        assert_eq!(snapshot.translation, TranslationSettings::default());
    }

    #[test]
    fn updating_one_section_preserves_other_sections() {
        let config_file = Arc::new(ConfigFile::new_temp());
        let home_dir = tempdir().unwrap();
        let default_save_dir = home_dir.path().join("Snapshots");

        let configuration = SettingsConfiguration::with_paths(
            config_file.clone(),
            Some(home_dir.path().to_path_buf()),
            default_save_dir.clone(),
            None,
        );

        let updated = configuration
            .update_general(GeneralSettings {
                language: "en".to_string(),
                theme: "dark".to_string(),
                start_on_boot: true,
            })
            .unwrap();

        assert_eq!(
            updated.general,
            GeneralSettings {
                language: "en".to_string(),
                theme: "dark".to_string(),
                start_on_boot: true,
            }
        );
        assert_eq!(
            updated.screenshot,
            ScreenshotSettings {
                save_path: default_save_dir.to_string_lossy().to_string(),
                ..ScreenshotSettings::default()
            }
        );
        assert_eq!(updated.translation, TranslationSettings::default());

        let persisted: SettingsSnapshot = config_file.load("settings").unwrap();
        assert_eq!(persisted, updated);
    }

    #[test]
    fn screenshot_save_path_normalization_expands_tilde() {
        let config_file = Arc::new(ConfigFile::new_temp());
        let home_dir = tempdir().unwrap();
        let default_save_dir = home_dir.path().join("Snapshots");

        let configuration = SettingsConfiguration::with_paths(
            config_file,
            Some(home_dir.path().to_path_buf()),
            default_save_dir,
            None,
        );

        let updated = configuration
            .update_screenshot(ScreenshotSettings {
                save_path: "~/captures".to_string(),
                format: "webp".to_string(),
                quality: 77,
            })
            .unwrap();

        assert_eq!(
            updated.screenshot,
            ScreenshotSettings {
                save_path: home_dir
                    .path()
                    .join("captures")
                    .to_string_lossy()
                    .to_string(),
                format: "webp".to_string(),
                quality: 77,
            }
        );
    }

    #[test]
    fn merges_legacy_frontend_durable_settings_once_without_touching_navigation_or_hotkeys() {
        let config_file = Arc::new(ConfigFile::new_temp());
        let home_dir = tempdir().unwrap();
        let default_save_dir = home_dir.path().join("Snapshots");
        let legacy_root = tempdir().unwrap();

        write_legacy_local_storage(
            legacy_root.path(),
            serde_json::json!({
                "state": {
                    "activeMainTab": "translation",
                    "screenshotSubTab": "save-settings",
                    "hotkeys": {
                        "screenshot": {
                            "screenshot": "F12"
                        }
                    },
                    "language": "en",
                    "theme": "dark",
                    "startOnBoot": true,
                    "screenshotSavePath": "~/legacy-captures",
                    "screenshotFormat": "jpg",
                    "screenshotQuality": 81,
                    "capturedScreenshot": "data:image/png;base64,abc",
                    "defaultSourceLang": "ja",
                    "defaultTargetLang": "fr"
                }
            }),
        );

        let configuration = SettingsConfiguration::with_paths(
            config_file.clone(),
            Some(home_dir.path().to_path_buf()),
            default_save_dir,
            Some(legacy_root.path().to_path_buf()),
        );

        let first_snapshot = configuration.snapshot().unwrap();
        assert_eq!(
            first_snapshot.general,
            GeneralSettings {
                language: "en".to_string(),
                theme: "dark".to_string(),
                start_on_boot: true,
            }
        );
        assert_eq!(
            first_snapshot.screenshot,
            ScreenshotSettings {
                save_path: home_dir
                    .path()
                    .join("legacy-captures")
                    .to_string_lossy()
                    .to_string(),
                format: "jpg".to_string(),
                quality: 81,
            }
        );
        assert_eq!(
            first_snapshot.translation,
            TranslationSettings {
                default_source_lang: "ja".to_string(),
                default_target_lang: "fr".to_string(),
            }
        );

        write_legacy_local_storage(
            legacy_root.path(),
            serde_json::json!({
                "state": {
                    "language": "de",
                    "theme": "light",
                    "defaultTargetLang": "es"
                }
            }),
        );

        let second_snapshot = configuration.snapshot().unwrap();
        assert_eq!(second_snapshot, first_snapshot);
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

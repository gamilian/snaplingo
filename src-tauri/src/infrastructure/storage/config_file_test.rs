use super::ConfigFile;
use crate::domain::{GeneralSettings, ScreenshotSettings, SettingsSnapshot, TranslationSettings};
use crate::error::{AppError, Result};
use std::collections::HashSet;
use tempfile::NamedTempFile;

#[test]
fn settings_snapshot_defaults_are_sectioned() {
    let snapshot = SettingsSnapshot::default();

    assert_eq!(
        snapshot.general,
        GeneralSettings {
            language: "zh-CN".to_string(),
            theme: "system".to_string(),
            start_on_boot: false,
        }
    );
    assert_eq!(snapshot.screenshot.format, "png");
    assert_eq!(snapshot.screenshot.quality, 90);
    assert_ne!(snapshot.screenshot.save_path, "~/Pictures/SnapLingo");
    assert_eq!(
        snapshot.translation,
        TranslationSettings {
            default_source_lang: "auto".to_string(),
            default_target_lang: "zh-CN".to_string(),
        }
    );
}

#[test]
fn settings_snapshot_loads_partial_sections_with_defaults() {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path();

    let config_file = ConfigFile::new(path.to_path_buf());

    config_file
        .save(
            "settings",
            &serde_json::json!({
                "general": {
                    "theme": "dark"
                },
                "translation": {
                    "default_target_lang": "en"
                }
            }),
        )
        .unwrap();

    let loaded: SettingsSnapshot = config_file.load("settings").unwrap();

    assert_eq!(
        loaded.general,
        GeneralSettings {
            language: "zh-CN".to_string(),
            theme: "dark".to_string(),
            start_on_boot: false,
        }
    );
    assert_eq!(loaded.screenshot, ScreenshotSettings::default());
    assert_eq!(
        loaded.translation,
        TranslationSettings {
            default_source_lang: "auto".to_string(),
            default_target_lang: "en".to_string(),
        }
    );
}

#[test]
fn settings_snapshot_round_trips_through_config_file() {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path();

    let config_file = ConfigFile::new(path.to_path_buf());

    let snapshot = SettingsSnapshot {
        general: GeneralSettings {
            language: "en".to_string(),
            theme: "dark".to_string(),
            start_on_boot: true,
        },
        screenshot: ScreenshotSettings {
            save_path: "/tmp/snaps".to_string(),
            format: "jpg".to_string(),
            quality: 80,
        },
        translation: TranslationSettings {
            default_source_lang: "ja".to_string(),
            default_target_lang: "en".to_string(),
        },
    };

    config_file.save("settings", &snapshot).unwrap();

    let loaded: SettingsSnapshot = config_file.load("settings").unwrap();

    assert_eq!(loaded, snapshot);
}

#[test]
fn test_load_nonexistent_key() {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path();

    let config_file = ConfigFile::new(path.to_path_buf());

    let result: Result<SettingsSnapshot> = config_file.load("nonexistent");

    assert!(result.is_err());
    match result {
        Err(AppError::Config(msg)) => {
            assert!(msg.contains("not found"));
        }
        _ => panic!("Expected Config error"),
    }
}

#[test]
fn test_save_multiple_keys() {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path();

    let config_file = ConfigFile::new(path.to_path_buf());

    let config1 = SettingsSnapshot {
        general: GeneralSettings {
            language: "en".to_string(),
            theme: "light".to_string(),
            start_on_boot: false,
        },
        screenshot: ScreenshotSettings {
            save_path: "/tmp/config1".to_string(),
            format: "png".to_string(),
            quality: 90,
        },
        translation: TranslationSettings {
            default_source_lang: "auto".to_string(),
            default_target_lang: "es".to_string(),
        },
    };
    config_file.save("config1", &config1).unwrap();

    let config2 = SettingsSnapshot {
        general: GeneralSettings {
            language: "ja".to_string(),
            theme: "system".to_string(),
            start_on_boot: true,
        },
        screenshot: ScreenshotSettings {
            save_path: "/tmp/config2".to_string(),
            format: "webp".to_string(),
            quality: 75,
        },
        translation: TranslationSettings {
            default_source_lang: "ja".to_string(),
            default_target_lang: "en".to_string(),
        },
    };
    config_file.save("config2", &config2).unwrap();

    let loaded1: SettingsSnapshot = config_file.load("config1").unwrap();
    let loaded2: SettingsSnapshot = config_file.load("config2").unwrap();

    assert_eq!(loaded1.general.language, "en");
    assert_eq!(loaded2.general.language, "ja");
    assert_eq!(loaded1.screenshot.format, "png");
    assert_eq!(loaded2.screenshot.format, "webp");
}

#[test]
fn test_unique_temp_write_paths_do_not_collide() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut paths = HashSet::new();

    for _ in 0..1000 {
        let path = super::config_file::unique_temp_write_path(temp_dir.path());
        assert!(paths.insert(path));
    }
}

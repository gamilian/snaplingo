use super::ConfigFile;
use crate::domain::AppConfig;
use crate::error::{AppError, Result};
use tempfile::NamedTempFile;
use std::collections::HashSet;

#[test]
fn test_save_and_load() {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path();

    let config_file = ConfigFile::new(path.to_path_buf());

    // Save a config
    let config = AppConfig {
        translation_provider: "google-translate".to_string(),
        source_language: "en".to_string(),
        target_language: "es".to_string(),
        hotkey: Some("Cmd+Shift+T".to_string()),
        auto_copy: false,
    };

    config_file.save("app_config", &config).unwrap();

    // Load it back
    let loaded: AppConfig = config_file.load("app_config").unwrap();

    assert_eq!(loaded.translation_provider, "google-translate");
    assert_eq!(loaded.source_language, "en");
    assert_eq!(loaded.target_language, "es");
    assert_eq!(loaded.hotkey, Some("Cmd+Shift+T".to_string()));
    assert_eq!(loaded.auto_copy, false);
}

#[test]
fn test_load_nonexistent_key() {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path();

    let config_file = ConfigFile::new(path.to_path_buf());

    let result: Result<AppConfig> = config_file.load("nonexistent");

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

    // Save first config
    let config1 = AppConfig {
        translation_provider: "google-translate".to_string(),
        source_language: "en".to_string(),
        target_language: "es".to_string(),
        hotkey: Some("Cmd+Shift+T".to_string()),
        auto_copy: false,
    };
    config_file.save("config1", &config1).unwrap();

    // Save second config
    let config2 = AppConfig {
        translation_provider: "deepl".to_string(),
        source_language: "ja".to_string(),
        target_language: "en".to_string(),
        hotkey: Some("Cmd+Shift+D".to_string()),
        auto_copy: true,
    };
    config_file.save("config2", &config2).unwrap();

    // Load both back
    let loaded1: AppConfig = config_file.load("config1").unwrap();
    let loaded2: AppConfig = config_file.load("config2").unwrap();

    assert_eq!(loaded1.source_language, "en");
    assert_eq!(loaded2.source_language, "ja");
    assert_eq!(loaded1.translation_provider, "google-translate");
    assert_eq!(loaded2.translation_provider, "deepl");
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

use tauri::State;

use crate::application::SettingsConfiguration;
use crate::domain::{GeneralSettings, ScreenshotSettings, SettingsSnapshot, TranslationSettings};

#[tauri::command]
pub fn get_settings_snapshot(
    state: State<'_, crate::AppState>,
) -> Result<SettingsSnapshot, String> {
    get_settings_snapshot_for_configuration(state.settings.configuration.as_ref())
}

#[tauri::command]
pub fn update_general_settings(
    input: GeneralSettings,
    state: State<'_, crate::AppState>,
) -> Result<SettingsSnapshot, String> {
    update_general_settings_for_configuration(state.settings.configuration.as_ref(), input)
}

#[tauri::command]
pub fn update_screenshot_settings(
    input: ScreenshotSettings,
    state: State<'_, crate::AppState>,
) -> Result<SettingsSnapshot, String> {
    update_screenshot_settings_for_configuration(state.settings.configuration.as_ref(), input)
}

#[tauri::command]
pub fn update_annotation_colors(
    colors: Vec<[u8; 4]>,
    state: State<'_, crate::AppState>,
) -> Result<SettingsSnapshot, String> {
    update_annotation_colors_for_configuration(state.settings.configuration.as_ref(), colors)
}

#[tauri::command]
pub fn update_translation_settings(
    input: TranslationSettings,
    state: State<'_, crate::AppState>,
) -> Result<SettingsSnapshot, String> {
    update_translation_settings_for_configuration(state.settings.configuration.as_ref(), input)
}

fn get_settings_snapshot_for_configuration(
    configuration: &SettingsConfiguration,
) -> Result<SettingsSnapshot, String> {
    configuration.snapshot().map_err(|err| err.to_string())
}

fn update_general_settings_for_configuration(
    configuration: &SettingsConfiguration,
    input: GeneralSettings,
) -> Result<SettingsSnapshot, String> {
    configuration
        .update_general(input)
        .map_err(|err| err.to_string())
}

fn update_screenshot_settings_for_configuration(
    configuration: &SettingsConfiguration,
    input: ScreenshotSettings,
) -> Result<SettingsSnapshot, String> {
    configuration
        .update_screenshot(input)
        .map_err(|err| err.to_string())
}

fn update_annotation_colors_for_configuration(
    configuration: &SettingsConfiguration,
    colors: Vec<[u8; 4]>,
) -> Result<SettingsSnapshot, String> {
    configuration
        .update_annotation_colors(colors)
        .map_err(|err| err.to_string())
}

fn update_translation_settings_for_configuration(
    configuration: &SettingsConfiguration,
    input: TranslationSettings,
) -> Result<SettingsSnapshot, String> {
    configuration
        .update_translation(input)
        .map_err(|err| err.to_string())
}

#[cfg(test)]
mod settings_commands_tests {
    use std::sync::Arc;

    use tempfile::tempdir;

    use super::{
        get_settings_snapshot_for_configuration, update_annotation_colors_for_configuration,
        update_translation_settings_for_configuration,
    };
    use crate::application::SettingsConfiguration;
    use crate::domain::{ScreenshotSettings, SettingsSnapshot, TranslationSettings};
    use crate::infrastructure::storage::ConfigFile;

    #[test]
    fn get_settings_snapshot_reads_from_backend_configuration() {
        let config_file = Arc::new(ConfigFile::new_temp());
        let home_dir = tempdir().unwrap();
        let default_save_dir = home_dir.path().join("Snapshots");
        let configuration = SettingsConfiguration::with_paths(
            config_file,
            Some(home_dir.path().to_path_buf()),
            default_save_dir.clone(),
            None,
        );

        let snapshot = get_settings_snapshot_for_configuration(&configuration).unwrap();

        assert_eq!(snapshot, configuration.snapshot().unwrap());
        assert_eq!(
            snapshot.screenshot.save_path,
            default_save_dir.to_string_lossy().to_string()
        );
    }

    #[test]
    fn update_translation_settings_delegates_and_returns_updated_snapshot() {
        let config_file = Arc::new(ConfigFile::new_temp());
        let home_dir = tempdir().unwrap();
        let configuration = SettingsConfiguration::with_paths(
            config_file,
            Some(home_dir.path().to_path_buf()),
            home_dir.path().join("Snapshots"),
            None,
        );

        let updated = update_translation_settings_for_configuration(
            &configuration,
            TranslationSettings {
                default_source_lang: "ja".to_string(),
                default_target_lang: "en".to_string(),
            },
        )
        .unwrap();

        assert_eq!(updated, configuration.snapshot().unwrap());
        assert_eq!(updated.translation.default_source_lang, "ja");
        assert_eq!(updated.translation.default_target_lang, "en");
        assert_eq!(updated.general, SettingsSnapshot::default().general);
    }

    #[test]
    fn update_annotation_colors_preserves_other_screenshot_settings() {
        let config_file = Arc::new(ConfigFile::new_temp());
        let home_dir = tempdir().unwrap();
        let configuration = SettingsConfiguration::with_paths(
            config_file,
            Some(home_dir.path().to_path_buf()),
            home_dir.path().join("Snapshots"),
            None,
        );
        configuration
            .update_screenshot(ScreenshotSettings {
                save_path: home_dir.path().join("Custom").to_string_lossy().to_string(),
                format: "webp".to_string(),
                quality: 72,
                ..ScreenshotSettings::default()
            })
            .unwrap();
        let colors = vec![[12, 34, 56, 255], [200, 150, 100, 255]];

        let updated =
            update_annotation_colors_for_configuration(&configuration, colors.clone()).unwrap();

        assert_eq!(updated.screenshot.annotation_colors, colors);
        assert_eq!(updated.screenshot.format, "webp");
        assert_eq!(updated.screenshot.quality, 72);
        assert_eq!(updated, configuration.snapshot().unwrap());
    }
}

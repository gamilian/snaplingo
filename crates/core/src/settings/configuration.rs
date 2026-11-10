use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use super::{GeneralField, GeneralValue, OcrField, ScreenshotField, StartOnBoot, TranslationField};
use crate::capture::configured_capture_save_dir;
use crate::library::capacity::FavoriteCapacityPolicyProvider;
use crate::library::history::HistoryCleanupPolicy;
use crate::library::history::HistoryPolicyProvider;
use crate::settings::model::GeneralSettings;
use crate::settings::model::HistorySettings;
use crate::settings::model::OcrSettings;
use crate::settings::model::ScreenshotSettings;
use crate::settings::model::SettingsSnapshot;
use crate::settings::model::TranslationSettings;
use crate::settings::SettingsStore;
use crate::{AppError, Result};

pub struct SettingsConfiguration {
    store: Arc<dyn SettingsStore>,
    change_notifier: Option<Arc<dyn SettingsChangeNotifier>>,
    home_dir: Option<PathBuf>,
    default_screenshot_save_dir: PathBuf,
    update_lock: Mutex<()>,
}

impl HistoryPolicyProvider for SettingsConfiguration {
    fn current_policy(&self) -> Result<HistoryCleanupPolicy> {
        let settings = self.snapshot()?.history;
        Ok(HistoryCleanupPolicy {
            enabled: settings.auto_cleanup_enabled,
            retention_days: settings.retention_days,
            maximum_records: settings.maximum_records,
        })
    }
}

impl FavoriteCapacityPolicyProvider for SettingsConfiguration {
    fn maximum_favorites(&self) -> Result<u32> {
        Ok(self.snapshot()?.history.maximum_favorites)
    }
}

pub trait SettingsChangeNotifier: Send + Sync {
    fn settings_changed(&self);
}

impl SettingsConfiguration {
    /// Deterministic configuration fixture for workflow and adapter tests.
    /// Production composition supplies the platform paths through `with_paths`.
    #[cfg(any(test, feature = "test-support"))]
    pub fn new(store: Arc<dyn SettingsStore>) -> Self {
        Self::with_paths(store, None, PathBuf::from("test-screenshots"))
    }

    pub fn with_change_notifier(mut self, notifier: Arc<dyn SettingsChangeNotifier>) -> Self {
        self.change_notifier = Some(notifier);
        self
    }

    pub fn with_paths(
        store: Arc<dyn SettingsStore>,
        home_dir: Option<PathBuf>,
        default_screenshot_save_dir: PathBuf,
    ) -> Self {
        Self {
            store,
            change_notifier: None,
            home_dir,
            default_screenshot_save_dir,
            update_lock: Mutex::new(()),
        }
    }

    pub fn snapshot(&self) -> Result<SettingsSnapshot> {
        match self.store.load_settings() {
            Ok(snapshot) => Ok(self.normalized_snapshot(snapshot)),
            Err(AppError::Config(_)) => Ok(self.default_snapshot()),
            Err(err) => Err(err),
        }
    }

    pub fn update_general(&self, input: GeneralSettings) -> Result<SettingsSnapshot> {
        let _guard = self.update_lock.lock().unwrap();
        let mut snapshot = self.snapshot()?;
        snapshot.general = input;
        self.save_snapshot(snapshot)
    }

    pub fn edit_general(
        &self,
        field: GeneralField,
        value: GeneralValue,
        start_on_boot: &dyn StartOnBoot,
    ) -> Result<SettingsSnapshot> {
        let _guard = self.update_lock.lock().unwrap();
        let mut snapshot = self.snapshot()?;
        if field.value(&snapshot.general) == value {
            return Ok(snapshot);
        }
        let previous_start_on_boot = snapshot.general.start_on_boot;
        field.apply(&mut snapshot.general, value)?;
        let next_start_on_boot = snapshot.general.start_on_boot;
        let changed_start_on_boot = next_start_on_boot != previous_start_on_boot;
        if changed_start_on_boot {
            start_on_boot.set_enabled(next_start_on_boot)?;
        }
        self.save_snapshot(snapshot).inspect_err(|_| {
            if changed_start_on_boot {
                let _ = start_on_boot.set_enabled(previous_start_on_boot);
            }
        })
    }

    pub fn update_last_result_window_position(&self, x: i32, y: i32) -> Result<()> {
        let _guard = self.update_lock.lock().unwrap();
        let mut snapshot = self.snapshot()?;
        snapshot.general.last_result_window_x = Some(x);
        snapshot.general.last_result_window_y = Some(y);
        self.save_snapshot(snapshot)?;
        Ok(())
    }

    pub fn update_screenshot(&self, mut input: ScreenshotSettings) -> Result<SettingsSnapshot> {
        Self::normalized_annotation_shortcuts(&input.annotation_tool_shortcuts)?;
        Self::normalized_pin_border_shortcut(&input.pin_border_shortcut)?;
        let _guard = self.update_lock.lock().unwrap();
        let mut snapshot = self.snapshot()?;
        input.last_save_directory = snapshot.screenshot.last_save_directory;
        input.last_save_format = snapshot.screenshot.last_save_format;
        snapshot.screenshot = input;
        self.save_snapshot(snapshot)
    }

    pub fn edit_screenshot(
        &self,
        field: ScreenshotField,
        value: GeneralValue,
    ) -> Result<SettingsSnapshot> {
        let _guard = self.update_lock.lock().unwrap();
        let mut snapshot = self.snapshot()?;
        let previous = snapshot.screenshot.clone();
        field.apply(&mut snapshot.screenshot, value)?;
        snapshot.screenshot.annotation_tool_shortcuts =
            Self::normalized_annotation_shortcuts(&snapshot.screenshot.annotation_tool_shortcuts)?;
        snapshot.screenshot.pin_border_shortcut =
            Self::normalized_pin_border_shortcut(&snapshot.screenshot.pin_border_shortcut)?;
        if snapshot.screenshot == previous {
            return Ok(snapshot);
        }
        self.save_snapshot(snapshot)
    }

    pub fn record_screenshot_save_choice(
        &self,
        path: &std::path::Path,
        format: &str,
    ) -> Result<()> {
        let _guard = self.update_lock.lock().unwrap();
        let mut snapshot = self.snapshot()?;
        snapshot.screenshot.last_save_directory = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map(|parent| parent.to_string_lossy().into_owned());
        snapshot.screenshot.last_save_format = Some(format.to_string());
        self.save_snapshot(snapshot)?;
        Ok(())
    }

    pub fn update_annotation_colors(&self, colors: Vec<[u8; 4]>) -> Result<SettingsSnapshot> {
        let _guard = self.update_lock.lock().unwrap();
        let mut snapshot = self.snapshot()?;
        snapshot.screenshot.annotation_colors = colors;
        self.save_snapshot(snapshot)
    }

    pub fn update_translation(&self, input: TranslationSettings) -> Result<SettingsSnapshot> {
        let _guard = self.update_lock.lock().unwrap();
        let mut snapshot = self.snapshot()?;
        snapshot.translation = input;
        self.save_snapshot(snapshot)
    }

    pub fn edit_translation(
        &self,
        field: TranslationField,
        value: GeneralValue,
    ) -> Result<SettingsSnapshot> {
        let _guard = self.update_lock.lock().unwrap();
        let mut snapshot = self.snapshot()?;
        let unchanged = field.value(&snapshot.translation) == value;
        field.apply(&mut snapshot.translation, value)?;
        if unchanged {
            return Ok(snapshot);
        }
        self.save_snapshot(snapshot)
    }

    pub fn update_ocr(&self, input: OcrSettings) -> Result<SettingsSnapshot> {
        let _guard = self.update_lock.lock().unwrap();
        let mut snapshot = self.snapshot()?;
        snapshot.ocr = input;
        self.save_snapshot(snapshot)
    }

    pub fn edit_ocr(&self, field: OcrField, value: GeneralValue) -> Result<SettingsSnapshot> {
        let _guard = self.update_lock.lock().unwrap();
        let mut snapshot = self.snapshot()?;
        let unchanged = field.value(&snapshot.ocr) == value;
        field.apply(&mut snapshot.ocr, value)?;
        if unchanged {
            return Ok(snapshot);
        }
        self.save_snapshot(snapshot)
    }

    pub fn update_history(&self, input: HistorySettings) -> Result<SettingsSnapshot> {
        let _guard = self.update_lock.lock().unwrap();
        let mut snapshot = self.snapshot()?;
        snapshot.history = input;
        self.save_snapshot(snapshot)
    }

    fn save_snapshot(&self, snapshot: SettingsSnapshot) -> Result<SettingsSnapshot> {
        let snapshot = self.normalized_snapshot(snapshot);
        self.store.save_settings(&snapshot)?;
        if let Some(notifier) = &self.change_notifier {
            notifier.settings_changed();
        }
        Ok(snapshot)
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
        if snapshot.general.language != "zh-CN" {
            snapshot.general.language = "zh-CN".to_string();
        }
        if !matches!(snapshot.general.theme.as_str(), "system" | "light" | "dark") {
            snapshot.general.theme = "system".to_string();
        }
        if !matches!(
            snapshot.general.interface_theme.as_str(),
            "native" | "studio" | "paper"
        ) {
            snapshot.general.interface_theme = "native".to_string();
        }
        if !matches!(
            snapshot.general.proxy_mode.as_str(),
            "system" | "none" | "manual"
        ) {
            snapshot.general.proxy_mode = "system".to_string();
        }
        snapshot.general.proxy_url = snapshot.general.proxy_url.trim().to_string();
        snapshot.general.request_timeout_ms =
            snapshot.general.request_timeout_ms.clamp(1_000, 120_000);
        snapshot.general.retry_count = snapshot.general.retry_count.clamp(0, 5);
        if !matches!(
            snapshot.general.log_level.as_str(),
            "debug" | "info" | "warn" | "error"
        ) {
            snapshot.general.log_level = "info".to_string();
        }
        snapshot.general.log_retention_days = snapshot.general.log_retention_days.clamp(1, 365);
        snapshot.general.system_tts_voice = snapshot.general.system_tts_voice.trim().to_string();
        snapshot.general.system_tts_rate = snapshot.general.system_tts_rate.clamp(80, 400);
        snapshot.general.settings_window_width = snapshot
            .general
            .settings_window_width
            .map(|width| width.clamp(700, 4_000));
        snapshot.general.settings_window_height = snapshot
            .general
            .settings_window_height
            .map(|height| height.clamp(500, 3_000));
        snapshot.screenshot.save_path =
            self.normalize_screenshot_save_path(&snapshot.screenshot.save_path);
        if !matches!(snapshot.screenshot.format.as_str(), "png" | "jpg" | "webp") {
            snapshot.screenshot.format = "png".to_string();
        }
        if !matches!(
            snapshot.screenshot.naming_rule.as_str(),
            "timestamp" | "unix-ms" | "date" | "counter" | "custom"
        ) {
            snapshot.screenshot.naming_rule = "timestamp".to_string();
        }
        snapshot.screenshot.quality = snapshot.screenshot.quality.clamp(50, 100);
        snapshot.screenshot.default_stroke_width =
            snapshot.screenshot.default_stroke_width.clamp(1, 32);
        snapshot.screenshot.selection_border_width =
            snapshot.screenshot.selection_border_width.clamp(1, 8);
        snapshot.screenshot.magnifier_zoom = snapshot.screenshot.magnifier_zoom.clamp(4, 20);
        snapshot.screenshot.default_font_size = snapshot.screenshot.default_font_size.clamp(10, 96);
        snapshot.screenshot.pin_opacity = snapshot.screenshot.pin_opacity.clamp(5, 100);
        snapshot.screenshot.longshot_max_height =
            snapshot.screenshot.longshot_max_height.clamp(100, 100_000);
        snapshot.screenshot.pin_border_shortcut =
            Self::normalized_pin_border_shortcut(&snapshot.screenshot.pin_border_shortcut)
                .unwrap_or_else(|_| "b".to_string());
        snapshot.screenshot.last_save_format = snapshot
            .screenshot
            .last_save_format
            .filter(|format| ["png", "jpg", "webp"].contains(&format.as_str()));
        snapshot.screenshot.last_save_directory = snapshot
            .screenshot
            .last_save_directory
            .filter(|directory| !directory.trim().is_empty());
        if !["original", "center"].contains(&snapshot.screenshot.pin_position.as_str()) {
            snapshot.screenshot.pin_position = "original".to_string();
        }
        if !["auto", "100", "125", "150", "175", "200"]
            .contains(&snapshot.screenshot.capture_ui_scale.as_str())
        {
            snapshot.screenshot.capture_ui_scale = "auto".to_string();
        }
        snapshot.screenshot.annotation_tool_shortcuts =
            Self::normalized_annotation_shortcuts(&snapshot.screenshot.annotation_tool_shortcuts)
                .unwrap_or_else(|_| crate::settings::model::default_annotation_tool_shortcuts());
        snapshot.screenshot.custom_file_name =
            snapshot.screenshot.custom_file_name.trim().to_string();
        if snapshot.screenshot.custom_file_name.is_empty() {
            snapshot.screenshot.custom_file_name = "SnapLingo".to_string();
        }
        snapshot.history.retention_days = snapshot.history.retention_days.clamp(1, 3650);
        snapshot.history.maximum_records = snapshot.history.maximum_records.clamp(100, 100_000);
        snapshot.history.maximum_favorites = snapshot.history.maximum_favorites.clamp(100, 100_000);
        snapshot.ocr.recognition_language = snapshot.ocr.recognition_language.trim().to_string();
        if snapshot.ocr.recognition_language.is_empty() {
            snapshot.ocr.recognition_language = "auto".to_string();
        }
        snapshot
    }

    fn normalized_pin_border_shortcut(value: &str) -> Result<String> {
        let value = value.trim().to_ascii_lowercase();
        if value.is_empty() {
            return Ok(value);
        }
        let parts: Vec<_> = value.split('+').collect();
        let (key, modifiers) = parts.split_last().unwrap();
        let single = key.len() == 1 && key.as_bytes()[0].is_ascii_alphanumeric();
        let function = key
            .strip_prefix('f')
            .and_then(|value| value.parse::<u8>().ok())
            .is_some_and(|value| (1..=24).contains(&value) && *key == format!("f{value}"));
        let mut used = std::collections::HashSet::new();
        if !(single || function)
            || modifiers.iter().any(|modifier| {
                !["ctrl", "meta", "alt", "shift"].contains(modifier) || !used.insert(*modifier)
            })
        {
            return Err(AppError::Config("Invalid pin border shortcut".into()));
        }
        let mut normalized: Vec<_> = ["ctrl", "meta", "alt", "shift"]
            .into_iter()
            .filter(|modifier| used.contains(modifier))
            .collect();
        normalized.push(key);
        Ok(normalized.join("+"))
    }

    fn normalized_annotation_shortcuts(
        input: &std::collections::HashMap<String, String>,
    ) -> Result<std::collections::HashMap<String, String>> {
        let mut shortcuts = crate::settings::model::default_annotation_tool_shortcuts();
        for (tool, key) in input {
            if !shortcuts.contains_key(tool) {
                return Err(AppError::Config(format!(
                    "Unknown annotation tool '{}'",
                    tool
                )));
            }
            shortcuts.insert(tool.clone(), key.trim().to_ascii_lowercase());
        }
        let mut used = std::collections::HashSet::new();
        for key in shortcuts.values() {
            if key.is_empty() {
                continue;
            }
            let single = key.len() == 1 && key.as_bytes()[0].is_ascii_alphanumeric();
            let function = key
                .strip_prefix('f')
                .and_then(|value| value.parse::<u8>().ok())
                .is_some_and(|value| (1..=24).contains(&value) && key == &format!("f{value}"));
            if !(single || function) || ["c", "f", "r", "f3"].contains(&key.as_str()) {
                return Err(AppError::Config(format!(
                    "Invalid or reserved annotation shortcut '{}'",
                    key
                )));
            }
            if !used.insert(key.clone()) {
                return Err(AppError::Config(format!(
                    "Duplicate annotation shortcut '{}'",
                    key
                )));
            }
        }
        Ok(shortcuts)
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

#[cfg(test)]
mod settings_configuration_tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use tempfile::tempdir;

    use super::{SettingsChangeNotifier, SettingsConfiguration};
    use crate::settings::model::GeneralSettings;
    use crate::settings::model::HistorySettings;
    use crate::settings::model::OcrSettings;
    use crate::settings::model::ScreenshotSettings;
    use crate::settings::model::TranslationSettings;
    use crate::test_support_config::MemoryConfigStore;

    struct CountingNotifier(AtomicUsize);

    impl SettingsChangeNotifier for CountingNotifier {
        fn settings_changed(&self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn loads_default_snapshot_when_no_settings_are_stored() {
        let store = Arc::new(MemoryConfigStore::new_in_memory());
        let home_dir = tempdir().unwrap();
        let default_save_dir = home_dir.path().join("Snapshots");
        let configuration = SettingsConfiguration::with_paths(
            store,
            Some(home_dir.path().to_path_buf()),
            default_save_dir.clone(),
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
        let store = Arc::new(MemoryConfigStore::new_in_memory());
        let home_dir = tempdir().unwrap();
        let default_save_dir = home_dir.path().join("Snapshots");
        let configuration = SettingsConfiguration::with_paths(
            store.clone(),
            Some(home_dir.path().to_path_buf()),
            default_save_dir.clone(),
        );

        let updated = configuration
            .update_general(GeneralSettings {
                language: "zh-CN".to_string(),
                theme: "dark".to_string(),
                start_on_boot: true,
                ..GeneralSettings::default()
            })
            .unwrap();

        assert_eq!(updated.general.language, "zh-CN");
        assert_eq!(
            updated.screenshot,
            ScreenshotSettings {
                save_path: default_save_dir.to_string_lossy().to_string(),
                ..ScreenshotSettings::default()
            }
        );
        assert_eq!(updated.translation, TranslationSettings::default());
        assert_eq!(store.load_settings().unwrap(), updated);
    }

    #[test]
    fn interface_themes_persist_and_notify_other_windows_without_changing_appearance() {
        let store = Arc::new(MemoryConfigStore::new_in_memory());
        let notifier = Arc::new(CountingNotifier(AtomicUsize::new(0)));
        let configuration =
            SettingsConfiguration::new(store.clone()).with_change_notifier(notifier.clone());

        for (index, theme) in ["native", "studio", "paper"].into_iter().enumerate() {
            configuration
                .update_general(GeneralSettings {
                    interface_theme: theme.to_string(),
                    theme: "dark".to_string(),
                    ..GeneralSettings::default()
                })
                .unwrap();

            let restored = SettingsConfiguration::new(store.clone())
                .snapshot()
                .unwrap();
            assert_eq!(restored.general.interface_theme, theme);
            assert_eq!(restored.general.theme, "dark");
            assert_eq!(notifier.0.load(Ordering::SeqCst), index + 1);
        }
    }

    #[test]
    fn notifier_binding_preserves_injected_default_and_home_expansion() {
        let store = Arc::new(MemoryConfigStore::new_in_memory());
        let notifier = Arc::new(CountingNotifier(AtomicUsize::new(0)));
        let home = tempdir().unwrap();
        let default = home.path().join("Custom Screenshots");
        let configuration = SettingsConfiguration::with_paths(
            store,
            Some(home.path().to_path_buf()),
            default.clone(),
        )
        .with_change_notifier(notifier.clone());

        assert_eq!(
            configuration.snapshot().unwrap().screenshot.save_path,
            default.to_string_lossy()
        );
        let saved = configuration
            .update_screenshot(ScreenshotSettings {
                save_path: "~/Saved Captures".into(),
                ..ScreenshotSettings::default()
            })
            .unwrap();
        assert_eq!(
            saved.screenshot.save_path,
            home.path().join("Saved Captures").to_string_lossy()
        );
        assert_eq!(notifier.0.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn unknown_interface_themes_fall_back_to_native() {
        let configuration =
            SettingsConfiguration::new(Arc::new(MemoryConfigStore::new_in_memory()));
        let updated = configuration
            .update_general(GeneralSettings {
                interface_theme: "unknown".to_string(),
                ..GeneralSettings::default()
            })
            .unwrap();
        assert_eq!(updated.general.interface_theme, "native");
    }

    #[test]
    fn updating_result_window_position_preserves_the_latest_general_settings() {
        let store = Arc::new(MemoryConfigStore::new_in_memory());
        let configuration = SettingsConfiguration::new(store);
        configuration
            .update_general(GeneralSettings {
                theme: "dark".to_string(),
                ..GeneralSettings::default()
            })
            .unwrap();

        configuration
            .update_last_result_window_position(420, 240)
            .unwrap();

        let general = configuration.snapshot().unwrap().general;
        assert_eq!(general.theme, "dark");
        assert_eq!(general.last_result_window_x, Some(420));
        assert_eq!(general.last_result_window_y, Some(240));
    }

    #[test]
    fn unsupported_interface_languages_fall_back_to_simplified_chinese() {
        let store = Arc::new(MemoryConfigStore::new_in_memory());
        let configuration = SettingsConfiguration::new(store);

        let updated = configuration
            .update_general(GeneralSettings {
                language: "en".to_string(),
                ..GeneralSettings::default()
            })
            .unwrap();

        assert_eq!(updated.general.language, "zh-CN");
    }

    #[test]
    fn screenshot_save_path_normalization_expands_tilde() {
        let store = Arc::new(MemoryConfigStore::new_in_memory());
        let home_dir = tempdir().unwrap();
        let configuration = SettingsConfiguration::with_paths(
            store,
            Some(home_dir.path().to_path_buf()),
            home_dir.path().join("Snapshots"),
        );

        let updated = configuration
            .update_screenshot(ScreenshotSettings {
                save_path: "~/captures".to_string(),
                format: "webp".to_string(),
                quality: 77,
                ..ScreenshotSettings::default()
            })
            .unwrap();

        assert_eq!(
            updated.screenshot.save_path,
            home_dir
                .path()
                .join("captures")
                .to_string_lossy()
                .to_string()
        );
        assert_eq!(updated.screenshot.format, "webp");
        assert_eq!(updated.screenshot.quality, 77);
    }

    #[test]
    fn screenshot_editor_settings_are_normalized_before_persisting() {
        let store = Arc::new(MemoryConfigStore::new_in_memory());
        let home_dir = tempdir().unwrap();
        let configuration = SettingsConfiguration::with_paths(
            store,
            Some(home_dir.path().to_path_buf()),
            home_dir.path().join("Snapshots"),
        );

        let updated = configuration
            .update_screenshot(ScreenshotSettings {
                format: "invalid".to_string(),
                quality: 1,
                naming_rule: "invalid".to_string(),
                custom_file_name: "  ".to_string(),
                default_stroke_width: 99,
                selection_border_width: 0,
                magnifier_zoom: 99,
                default_font_size: 1,
                pin_opacity: 1,
                pin_position: "cursor".into(),
                capture_ui_scale: "900".into(),
                ..ScreenshotSettings::default()
            })
            .unwrap();

        assert_eq!(updated.screenshot.format, "png");
        assert_eq!(updated.screenshot.quality, 50);
        assert_eq!(updated.screenshot.naming_rule, "timestamp");
        assert_eq!(updated.screenshot.custom_file_name, "SnapLingo");
        assert_eq!(updated.screenshot.default_stroke_width, 32);
        assert_eq!(updated.screenshot.selection_border_width, 1);
        assert_eq!(updated.screenshot.magnifier_zoom, 20);
        assert_eq!(updated.screenshot.default_font_size, 10);
        assert_eq!(updated.screenshot.pin_opacity, 5);
        assert_eq!(updated.screenshot.pin_position, "original");
        assert_eq!(updated.screenshot.capture_ui_scale, "auto");
    }

    #[test]
    fn legacy_screenshot_settings_default_new_fields_and_merge_partial_shortcuts() {
        let legacy: ScreenshotSettings =
            serde_json::from_str(r#"{"format":"jpg","pin_opacity":75}"#).unwrap();
        assert!(!legacy.pin_border);
        assert_eq!(legacy.pin_position, "original");
        assert_eq!(legacy.capture_ui_scale, "auto");
        assert_eq!(legacy.annotation_tool_shortcuts["scrolling"], "s");
        let configuration =
            SettingsConfiguration::new(Arc::new(MemoryConfigStore::new_in_memory()));
        let updated = configuration
            .update_screenshot(ScreenshotSettings {
                annotation_tool_shortcuts: [
                    ("arrow".into(), " X ".into()),
                    ("text".into(), String::new()),
                ]
                .into(),
                pin_border: true,
                pin_position: "center".into(),
                capture_ui_scale: "150".into(),
                ..legacy
            })
            .unwrap();
        assert_eq!(updated.screenshot.annotation_tool_shortcuts["arrow"], "x");
        assert_eq!(updated.screenshot.annotation_tool_shortcuts["text"], "");
        assert_eq!(
            updated.screenshot.annotation_tool_shortcuts["scrolling"],
            "s"
        );
        assert!(updated.screenshot.pin_border);
        assert_eq!(
            configuration.snapshot().unwrap().screenshot,
            updated.screenshot
        );
    }

    #[test]
    fn invalid_editor_shortcuts_do_not_replace_saved_preferences_and_scrolling_can_be_reassigned() {
        let configuration =
            SettingsConfiguration::new(Arc::new(MemoryConfigStore::new_in_memory()));
        let original = configuration.snapshot().unwrap();
        for (tool, key) in [
            ("arrow", "O"),
            ("arrow", "c"),
            ("arrow", "Ctrl+x"),
            ("unknown", "x"),
        ] {
            let input = ScreenshotSettings {
                annotation_tool_shortcuts: [(tool.into(), key.into())].into(),
                ..ScreenshotSettings::default()
            };
            assert!(configuration.update_screenshot(input).is_err());
            assert_eq!(configuration.snapshot().unwrap(), original);
        }
        let updated = configuration
            .update_screenshot(ScreenshotSettings {
                annotation_tool_shortcuts: [
                    ("arrow".into(), "s".into()),
                    ("scrolling".into(), "x".into()),
                ]
                .into(),
                ..ScreenshotSettings::default()
            })
            .unwrap();
        assert_eq!(
            updated.screenshot.annotation_tool_shortcuts["scrolling"],
            "x"
        );
        assert_eq!(updated.screenshot.annotation_tool_shortcuts["arrow"], "s");
    }

    #[test]
    fn library_capacities_are_normalized_before_persisting() {
        let store = Arc::new(MemoryConfigStore::new_in_memory());
        let configuration = SettingsConfiguration::new(store);

        let updated = configuration
            .update_history(HistorySettings {
                maximum_records: 200_000,
                maximum_favorites: 1,
                ..HistorySettings::default()
            })
            .unwrap();

        assert_eq!(updated.history.maximum_records, 100_000);
        assert_eq!(updated.history.maximum_favorites, 100);
    }

    #[test]
    fn successful_settings_updates_notify_runtime_observers() {
        let store = Arc::new(MemoryConfigStore::new_in_memory());
        let notifier = Arc::new(CountingNotifier(AtomicUsize::new(0)));
        let configuration =
            SettingsConfiguration::new(store).with_change_notifier(notifier.clone());

        configuration
            .update_general(GeneralSettings::default())
            .unwrap();
        configuration
            .update_screenshot(ScreenshotSettings::default())
            .unwrap();
        configuration
            .update_annotation_colors(vec![[1, 2, 3, 255]])
            .unwrap();
        configuration
            .update_translation(TranslationSettings::default())
            .unwrap();
        configuration.update_ocr(OcrSettings::default()).unwrap();
        configuration
            .update_history(HistorySettings::default())
            .unwrap();

        assert_eq!(notifier.0.load(Ordering::SeqCst), 6);
    }
    #[test]
    fn save_choice_survives_a_stale_full_settings_update() {
        let configuration =
            SettingsConfiguration::new(Arc::new(MemoryConfigStore::new_in_memory()));
        let old = configuration.snapshot().unwrap().screenshot;
        configuration
            .record_screenshot_save_choice(std::path::Path::new("/chosen/region.webp"), "webp")
            .unwrap();
        let updated = configuration
            .update_screenshot(ScreenshotSettings {
                naming_rule: "unix-ms".into(),
                longshot_max_height: 12000,
                pin_border_shortcut: "Shift+Ctrl+F4".into(),
                ..old
            })
            .unwrap();
        assert_eq!(
            updated.screenshot.last_save_directory.as_deref(),
            Some("/chosen")
        );
        assert_eq!(updated.screenshot.last_save_format.as_deref(), Some("webp"));
        assert_eq!(updated.screenshot.naming_rule, "unix-ms");
        assert_eq!(updated.screenshot.pin_border_shortcut, "ctrl+shift+f4");
        assert_eq!(updated.screenshot.longshot_max_height, 12000);
        let updated = configuration
            .update_screenshot(ScreenshotSettings {
                annotation_tool_shortcuts: [
                    ("arrow".into(), "2".into()),
                    ("scrolling".into(), "F2".into()),
                ]
                .into(),
                ..updated.screenshot
            })
            .unwrap();
        assert_eq!(updated.screenshot.annotation_tool_shortcuts["arrow"], "2");
        assert_eq!(
            updated.screenshot.annotation_tool_shortcuts["scrolling"],
            "f2"
        );
    }
}

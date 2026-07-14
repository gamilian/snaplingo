use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::application::capture::configured_capture_save_dir;
use crate::application::history::{HistoryCleanupPolicy, HistoryPolicyProvider};
use crate::application::settings::SettingsStore;
use crate::domain::{
    GeneralSettings, HistorySettings, OcrSettings, ScreenshotSettings, SettingsSnapshot,
    TranslationSettings,
};
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

pub trait SettingsChangeNotifier: Send + Sync {
    fn settings_changed(&self);
}

impl SettingsConfiguration {
    pub fn new(store: Arc<dyn SettingsStore>) -> Self {
        Self::with_paths(store, dirs::home_dir(), default_screenshot_save_dir())
    }

    pub fn with_change_notifier(
        store: Arc<dyn SettingsStore>,
        change_notifier: Arc<dyn SettingsChangeNotifier>,
    ) -> Self {
        Self {
            store,
            change_notifier: Some(change_notifier),
            home_dir: dirs::home_dir(),
            default_screenshot_save_dir: default_screenshot_save_dir(),
            update_lock: Mutex::new(()),
        }
    }

    pub(crate) fn with_paths(
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

    pub fn update_screenshot(&self, input: ScreenshotSettings) -> Result<SettingsSnapshot> {
        let _guard = self.update_lock.lock().unwrap();
        let mut snapshot = self.snapshot()?;
        snapshot.screenshot = input;
        self.save_snapshot(snapshot)
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

    pub fn update_ocr(&self, input: OcrSettings) -> Result<SettingsSnapshot> {
        let _guard = self.update_lock.lock().unwrap();
        let mut snapshot = self.snapshot()?;
        snapshot.ocr = input;
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
        snapshot.screenshot.save_path =
            self.normalize_screenshot_save_path(&snapshot.screenshot.save_path);
        if !matches!(snapshot.screenshot.format.as_str(), "png" | "jpg" | "webp") {
            snapshot.screenshot.format = "png".to_string();
        }
        if !matches!(
            snapshot.screenshot.naming_rule.as_str(),
            "timestamp" | "date" | "counter" | "custom"
        ) {
            snapshot.screenshot.naming_rule = "timestamp".to_string();
        }
        snapshot.screenshot.quality = snapshot.screenshot.quality.clamp(50, 100);
        snapshot.screenshot.default_stroke_width =
            snapshot.screenshot.default_stroke_width.clamp(1, 8);
        snapshot.screenshot.default_font_size = snapshot.screenshot.default_font_size.clamp(12, 48);
        snapshot.screenshot.pin_opacity = snapshot.screenshot.pin_opacity.clamp(20, 100);
        snapshot.screenshot.custom_file_name =
            snapshot.screenshot.custom_file_name.trim().to_string();
        if snapshot.screenshot.custom_file_name.is_empty() {
            snapshot.screenshot.custom_file_name = "SnapLingo".to_string();
        }
        snapshot.history.retention_days = snapshot.history.retention_days.clamp(1, 3650);
        snapshot.history.maximum_records = snapshot.history.maximum_records.clamp(100, 100_000);
        snapshot.ocr.recognition_language = snapshot.ocr.recognition_language.trim().to_string();
        if snapshot.ocr.recognition_language.is_empty() {
            snapshot.ocr.recognition_language = "auto".to_string();
        }
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

fn default_screenshot_save_dir() -> PathBuf {
    dirs::picture_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(std::env::temp_dir)
        .join("SnapLingo")
}

#[cfg(test)]
mod settings_configuration_tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use tempfile::tempdir;

    use super::{SettingsChangeNotifier, SettingsConfiguration};
    use crate::domain::{GeneralSettings, OcrSettings, ScreenshotSettings, TranslationSettings};
    use crate::infrastructure::storage::SqliteConfigStore;

    struct CountingNotifier(AtomicUsize);

    impl SettingsChangeNotifier for CountingNotifier {
        fn settings_changed(&self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn loads_default_snapshot_when_no_settings_are_stored() {
        let store = Arc::new(SqliteConfigStore::new_in_memory());
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
        let store = Arc::new(SqliteConfigStore::new_in_memory());
        let home_dir = tempdir().unwrap();
        let default_save_dir = home_dir.path().join("Snapshots");
        let configuration = SettingsConfiguration::with_paths(
            store.clone(),
            Some(home_dir.path().to_path_buf()),
            default_save_dir.clone(),
        );

        let updated = configuration
            .update_general(GeneralSettings {
                language: "en".to_string(),
                theme: "dark".to_string(),
                start_on_boot: true,
            })
            .unwrap();

        assert_eq!(updated.general.language, "en");
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
    fn screenshot_save_path_normalization_expands_tilde() {
        let store = Arc::new(SqliteConfigStore::new_in_memory());
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
        let store = Arc::new(SqliteConfigStore::new_in_memory());
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
                default_font_size: 1,
                pin_opacity: 1,
                ..ScreenshotSettings::default()
            })
            .unwrap();

        assert_eq!(updated.screenshot.format, "png");
        assert_eq!(updated.screenshot.quality, 50);
        assert_eq!(updated.screenshot.naming_rule, "timestamp");
        assert_eq!(updated.screenshot.custom_file_name, "SnapLingo");
        assert_eq!(updated.screenshot.default_stroke_width, 8);
        assert_eq!(updated.screenshot.default_font_size, 12);
        assert_eq!(updated.screenshot.pin_opacity, 20);
    }

    #[test]
    fn successful_settings_updates_notify_runtime_observers() {
        let store = Arc::new(SqliteConfigStore::new_in_memory());
        let notifier = Arc::new(CountingNotifier(AtomicUsize::new(0)));
        let configuration = SettingsConfiguration::with_change_notifier(store, notifier.clone());

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

        assert_eq!(notifier.0.load(Ordering::SeqCst), 5);
    }
}

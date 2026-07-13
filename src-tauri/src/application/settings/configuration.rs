use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::application::capture::configured_capture_save_dir;
use crate::application::settings::SettingsStore;
use crate::domain::{GeneralSettings, ScreenshotSettings, SettingsSnapshot, TranslationSettings};
use crate::{AppError, Result};

pub struct SettingsConfiguration {
    store: Arc<dyn SettingsStore>,
    change_notifier: Option<Arc<dyn SettingsChangeNotifier>>,
    home_dir: Option<PathBuf>,
    default_screenshot_save_dir: PathBuf,
    update_lock: Mutex<()>,
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
    use crate::domain::{GeneralSettings, ScreenshotSettings, TranslationSettings};
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

        assert_eq!(notifier.0.load(Ordering::SeqCst), 4);
    }
}

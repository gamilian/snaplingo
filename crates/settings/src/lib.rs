//! SQLite-backed preferences for an independently hosted shared Settings UI.
//! The caller supplies shortcut registration and confirms group resets before
//! dispatch. Opening preferences never registers startup shortcuts or actions.
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::{Arc, Mutex},
};

use snaplingo_core::{
    application::{
        capture::interface_scale,
        hotkeys::{
            display_hotkey_to_accelerator, HotkeyChangeNotifier, HotkeyConfiguration,
            HotkeyRecording, HotkeyRegistrar, HotkeyRegistration, HotkeyRuntime,
        },
        settings::{
            GeneralField as GeneralSetting, GeneralValue, OcrField as Field,
            ScreenshotField as ScreenshotSetting, SettingsConfiguration, StartOnBoot,
            TranslationField as TranslationSetting,
        },
    },
    domain::{
        capture::LogicalRect,
        hotkey_config::{
            self, hotkey_category, hotkey_category_mut, HotkeySettingsSnapshot, DEFAULT_HOTKEYS,
            HOTKEY_UNSET, OCR_CATEGORY, SCREENSHOT_CATEGORY, TRANSLATION_CATEGORY,
        },
        SettingsSnapshot,
    },
};
use snaplingo_storage::{Database, SqliteConfigStore};
use snaplingo_ui::{
    Action, GeneralField, Hotkey, OcrField, Page, ScreenshotField, Size, Snapshot, Theme,
    TranslationField, Value,
};

pub trait SettingsBackend {
    fn snapshot(&self) -> Result<Snapshot, String>;
    /// Group resets must be confirmed by the host before this call.
    fn dispatch(&mut self, page: Page, action: Action) -> Result<(), String>;
    fn interface_scale(&self, _display: Size) -> f32 {
        1.0
    }
}

const WINDOW: &str = "shared-settings";
const OCR_ACTIONS: [&str; 3] = [
    hotkey_config::SCREENSHOT_OCR_ACTION,
    hotkey_config::SILENT_SCREENSHOT_OCR_ACTION,
    hotkey_config::FILE_OCR_ACTION,
];
const TRANSLATION_ACTIONS: [&str; 3] = [
    hotkey_config::SELECTION_TRANSLATE_ACTION,
    hotkey_config::SCREENSHOT_TRANSLATE_ACTION,
    hotkey_config::SHOW_TRANSLATION_WINDOW_ACTION,
];
const SCREENSHOT_ACTIONS: [&str; 5] = [
    hotkey_config::SCREENSHOT_ACTION,
    hotkey_config::SCREENSHOT_COPY_ACTION,
    hotkey_config::PIN_ACTION,
    hotkey_config::PIN_TOGGLE_ALL_ACTION,
    hotkey_config::PIN_SWITCH_GROUP_ACTION,
];

/// A preview registrar deliberately performs no OS registration. Persistence and
/// conflict validation remain real; this does not provide global shortcuts.
pub struct RecordingOnlyRegistrar;
impl HotkeyRegistrar for RecordingOnlyRegistrar {
    fn register(&self, _: HotkeyRegistration) -> snaplingo_core::Result<()> {
        Ok(())
    }
    fn unregister(&self, _: &str) -> snaplingo_core::Result<()> {
        Ok(())
    }
}

struct SnapshotChangeNotifier;
impl HotkeyChangeNotifier for SnapshotChangeNotifier {
    // The synchronous SettingsBackend contract reads a fresh snapshot after
    // every dispatch. This adapter does not claim to publish desktop events.
    fn hotkeys_changed(&self) {}
}

// Only registrations made through this settings adapter belong to its lifetime.
// This leaves pre-existing application shortcuts under their existing owner.
struct Registrations {
    host: Arc<dyn HotkeyRegistrar>,
    active: Mutex<BTreeSet<String>>,
}
impl HotkeyRegistrar for Registrations {
    fn register(&self, registration: HotkeyRegistration) -> snaplingo_core::Result<()> {
        let accelerator = registration.accelerator.clone();
        self.host.register(registration)?;
        self.active.lock().unwrap().insert(accelerator);
        Ok(())
    }
    fn unregister(&self, accelerator: &str) -> snaplingo_core::Result<()> {
        self.host.unregister(accelerator)?;
        self.active.lock().unwrap().remove(accelerator);
        Ok(())
    }
}
impl Drop for Registrations {
    fn drop(&mut self) {
        for accelerator in self.active.get_mut().unwrap().iter() {
            // A registrar failure during teardown cannot keep the SQLite or
            // Settings UI alive; registration errors during edits are returned.
            let _ = self.host.unregister(accelerator);
        }
    }
}

pub struct PersistentSettings {
    configuration: SettingsConfiguration,
    hotkeys: HotkeyRuntime,
    recording: HotkeyRecording,
    recording_sequence: u64,
    start_on_boot: Arc<dyn StartOnBoot>,
}

struct UnavailableStartOnBoot;
impl StartOnBoot for UnavailableStartOnBoot {
    fn set_enabled(&self, _: bool) -> snaplingo_core::Result<()> {
        Err("开机自启需要窗口宿主提供系统启动项适配器".into())
    }
}

impl PersistentSettings {
    pub fn open(
        path: impl AsRef<Path>,
        registrar: Arc<dyn HotkeyRegistrar>,
    ) -> Result<Self, String> {
        Self::open_with_start_on_boot(path, registrar, Arc::new(UnavailableStartOnBoot))
    }

    pub fn open_with_start_on_boot(
        path: impl AsRef<Path>,
        registrar: Arc<dyn HotkeyRegistrar>,
        start_on_boot: Arc<dyn StartOnBoot>,
    ) -> Result<Self, String> {
        let store = Arc::new(SqliteConfigStore::new(Arc::new(
            Database::open(path.as_ref()).map_err(|error| error.to_string())?,
        )));
        let registrar = Arc::new(Registrations {
            host: registrar,
            active: Mutex::new(BTreeSet::new()),
        });
        Ok(Self {
            configuration: SettingsConfiguration::new(store.clone()),
            hotkeys: HotkeyRuntime::with_change_notifier(
                Arc::new(HotkeyConfiguration::new(store)),
                registrar,
                Arc::new(SnapshotChangeNotifier),
            ),
            recording: HotkeyRecording::default(),
            recording_sequence: 0,
            start_on_boot,
        })
    }

    fn hotkey_target(page: Page, index: usize) -> Result<(&'static str, &'static str), String> {
        let (category, actions): (_, &[&str]) = match page {
            Page::Ocr => (OCR_CATEGORY, &OCR_ACTIONS),
            Page::Translation => (TRANSLATION_CATEGORY, &TRANSLATION_ACTIONS),
            Page::Screenshot => (SCREENSHOT_CATEGORY, &SCREENSHOT_ACTIONS),
            _ => return Err("当前页面没有快捷键配置".into()),
        };
        actions
            .get(index)
            .copied()
            .map(|action| (category, action))
            .ok_or_else(|| "快捷键行不存在".into())
    }

    fn validate_change(&self, category: &str, action: &str, shortcut: &str) -> Result<(), String> {
        let mut snapshot = self.hotkeys.snapshot().map_err(|error| error.to_string())?;
        hotkey_category_mut(&mut snapshot, category)
            .map_err(|error| error.to_string())?
            .insert(action.into(), shortcut.into());
        validate_hotkey_conflicts(&snapshot)
    }
}

impl SettingsBackend for PersistentSettings {
    fn snapshot(&self) -> Result<Snapshot, String> {
        let saved = self
            .configuration
            .snapshot()
            .map_err(|error| error.to_string())?;
        let hotkeys = self.hotkeys.snapshot().map_err(|error| error.to_string())?;
        Ok(project_snapshot(
            &saved,
            &hotkeys,
            &self.hotkeys.default_snapshot(),
        ))
    }

    fn dispatch(&mut self, page: Page, action: Action) -> Result<(), String> {
        match action {
            Action::EditGeneral { field, value } => {
                if !field.accepts(&value) {
                    return Err("设置值不符合此通用控件".into());
                }
                self.configuration
                    .edit_general(
                        general_field(field),
                        general_value(value),
                        self.start_on_boot.as_ref(),
                    )
                    .map_err(|error| error.to_string())?;
            }
            Action::EditScreenshot { field, value } => {
                self.configuration
                    .edit_screenshot(screenshot_field(field), general_value(value))
                    .map_err(|error| error.to_string())?;
                self.recording.cancel_for_window(WINDOW);
            }
            Action::EditOcr { field, value } => {
                self.configuration
                    .edit_ocr(ocr_field(field), general_value(value))
                    .map_err(|error| error.to_string())?;
            }
            Action::EditTranslation { field, value } => {
                self.configuration
                    .edit_translation(translation_field(field), general_value(value))
                    .map_err(|error| error.to_string())?;
            }
            Action::BeginHotkeyRecording { index } => {
                let (category, action) = Self::hotkey_target(page, index)?;
                self.recording_sequence += 1;
                self.recording.begin(
                    WINDOW,
                    format!("{category}:{action}:{}", self.recording_sequence),
                );
            }
            Action::BeginLocalShortcutRecording { field } => {
                if !matches!(
                    field,
                    ScreenshotField::AnnotationShortcut(_) | ScreenshotField::PinBorderShortcut
                ) {
                    return Err("此截图设置不是快捷键".into());
                }
                self.recording_sequence += 1;
                self.recording.begin(
                    WINDOW,
                    format!("local:{field:?}:{}", self.recording_sequence),
                );
            }
            Action::EndHotkeyRecording => self.recording.cancel_for_window(WINDOW),
            Action::SaveHotkey { index, shortcut } => {
                let (category, action) = Self::hotkey_target(page, index)?;
                self.validate_change(category, action, &shortcut)?;
                self.hotkeys
                    .update_hotkey(category.into(), action.into(), shortcut)
                    .map_err(|error| error.to_string())?;
                self.recording.cancel_for_window(WINDOW);
            }
            Action::ResetHotkey { index } => {
                let (category, action) = Self::hotkey_target(page, index)?;
                let defaults = self.hotkeys.default_snapshot();
                let shortcut = &hotkey_category(&defaults, category).unwrap()[action];
                self.validate_change(category, action, shortcut)?;
                self.hotkeys
                    .reset_hotkey(category.into(), action.into())
                    .map_err(|error| error.to_string())?;
                self.recording.cancel_for_window(WINDOW);
            }
            Action::ResetHotkeys => {
                // Dispatch this only after the host obtains user confirmation.
                let (category, _) = Self::hotkey_target(page, 0)?;
                let mut next = self.hotkeys.snapshot().map_err(|error| error.to_string())?;
                *hotkey_category_mut(&mut next, category).map_err(|error| error.to_string())? =
                    hotkey_category(&self.hotkeys.default_snapshot(), category)
                        .unwrap()
                        .clone();
                validate_hotkey_conflicts(&next)?;
                self.hotkeys
                    .reset_category(category.into())
                    .map_err(|error| error.to_string())?;
                self.recording.cancel_for_window(WINDOW);
            }
            Action::CheckHotkeyConflicts => validate_hotkey_conflicts(
                &self.hotkeys.snapshot().map_err(|error| error.to_string())?,
            )?,
            Action::Navigate(Page::General | Page::Ocr | Page::Translation | Page::Screenshot) => {}
            Action::GeneralCommand(_) => {
                return Err("此操作需要完整应用提供系统权限、维护或更新适配器".into())
            }
            Action::SelectScreenshotDirectory
            | Action::SelectScreenshotColor { .. }
            | Action::CopyToClipboard(_)
            | Action::ReadClipboard => {
                return Err("此操作需要窗口宿主提供本机对话框或剪贴板适配器".into())
            }
            Action::Navigate(_) | Action::ConfigureCapacity => {
                return Err(
                    "此独立设置窗口已连接 OCR、翻译和快捷键配置；此操作仍由完整应用提供。".into(),
                )
            }
        }
        Ok(())
    }

    fn interface_scale(&self, display: Size) -> f32 {
        let mode = self
            .configuration
            .snapshot()
            .map(|snapshot| snapshot.screenshot.capture_ui_scale)
            .unwrap_or("auto".into());
        interface_scale(
            &mode,
            Some(&LogicalRect {
                x: 0.0,
                y: 0.0,
                width: display.width.into(),
                height: display.height.into(),
            }),
        ) as f32
    }
}

impl Drop for PersistentSettings {
    fn drop(&mut self) {
        self.recording.cancel_for_window(WINDOW);
    }
}

/// Project real durable values; callers such as the retained app can reuse this
/// conversion without opening a second database or constructing this adapter.
pub fn project_snapshot(
    saved: &SettingsSnapshot,
    hotkeys: &HotkeySettingsSnapshot,
    defaults: &HotkeySettingsSnapshot,
) -> Snapshot {
    let mut snapshot = Snapshot {
        theme: match saved.general.interface_theme.as_str() {
            "studio" => Theme::Studio,
            "paper" => Theme::Paper,
            _ => Theme::Native,
        },
        dark: saved.general.theme == "dark",
        ..Snapshot::default()
    };
    let source = &saved.ocr;
    let source_general = &saved.general;
    snapshot.general = snaplingo_ui::GeneralPreferences {
        interface_theme: source_general.interface_theme.clone(),
        language: source_general.language.clone(),
        appearance: source_general.theme.clone(),
        start_on_boot: source_general.start_on_boot,
        proxy_mode: source_general.proxy_mode.clone(),
        proxy_url: source_general.proxy_url.clone(),
        request_timeout_ms: source_general.request_timeout_ms,
        retry_count: source_general.retry_count.into(),
        log_level: source_general.log_level.clone(),
        log_retention_days: source_general.log_retention_days.into(),
        gpu_composition: source_general.experimental_gpu_acceleration,
        performance_monitoring: source_general.performance_monitoring,
    };
    let screenshot = &saved.screenshot;
    snapshot.screenshot = snaplingo_ui::ScreenshotPreferences {
        save_path: screenshot.save_path.clone(),
        format: screenshot.format.clone(),
        quality: screenshot.quality,
        naming_rule: screenshot.naming_rule.clone(),
        custom_file_name: screenshot.custom_file_name.clone(),
        last_save_directory: screenshot.last_save_directory.clone(),
        last_save_format: screenshot.last_save_format.clone(),
        auto_copy: screenshot.auto_copy,
        default_stroke_width: screenshot.default_stroke_width,
        default_font_size: screenshot.default_font_size,
        remember_last_tool: screenshot.remember_last_tool,
        show_selection_size: screenshot.show_selection_size,
        show_magnifier: screenshot.show_magnifier,
        magnifier_zoom: screenshot.magnifier_zoom,
        pin_opacity: screenshot.pin_opacity,
        pin_shadow: screenshot.pin_shadow,
        pin_border: screenshot.pin_border,
        pin_position: screenshot.pin_position.clone(),
        pin_border_shortcut: screenshot.pin_border_shortcut.clone(),
        longshot_max_height: screenshot.longshot_max_height,
        capture_ui_scale: screenshot.capture_ui_scale.clone(),
        annotation_tool_shortcuts: screenshot.annotation_tool_shortcuts.clone(),
        annotation_colors: screenshot.annotation_colors.clone(),
        selection_border_width: screenshot.selection_border_width,
        selection_border_color: screenshot.selection_border_color,
        selection_mask_color: screenshot.selection_mask_color,
    };
    snapshot.ocr = snaplingo_ui::OcrPreferences {
        gpu_acceleration: source.gpu_acceleration,
        recognition_language: source.recognition_language.clone(),
        preserve_formatting: source.preserve_formatting,
        remove_chinese_spaces: source.remove_chinese_spaces,
        show_confidence: source.show_confidence,
        window_position: source.window_position.clone(),
        hide_silent_status: source.hide_silent_status,
    };
    let source = &saved.translation;
    snapshot.translation = snaplingo_ui::TranslationPreferences {
        default_source_lang: source.default_source_lang.clone(),
        default_target_lang: source.default_target_lang.clone(),
        auto_translate: source.auto_translate,
        auto_copy: source.auto_copy,
        preserve_line_breaks: source.preserve_line_breaks,
        incremental_translation: source.incremental_translation,
        window_always_on_top: source.window_always_on_top,
        hide_on_blur: source.hide_on_blur,
        selection_window_position: source.selection_window_position.clone(),
        input_window_position: source.input_window_position.clone(),
        selection_input_state: source.selection_input_state.clone(),
        screenshot_input_state: source.screenshot_input_state.clone(),
        max_window_height_ratio: source.max_window_height_ratio,
        window_width: source.window_width,
        selection_text_mode: source.selection_text_mode.clone(),
    };
    let rows = |category, actions: [&str; 3]| {
        actions.map(|action| Hotkey {
            value: hotkey_category(hotkeys, category)
                .and_then(|keys| keys.get(action))
                .cloned()
                .unwrap_or(HOTKEY_UNSET.into()),
            default_value: hotkey_category(defaults, category)
                .and_then(|keys| keys.get(action))
                .cloned()
                .unwrap_or(HOTKEY_UNSET.into()),
        })
    };
    snapshot.hotkeys = rows(OCR_CATEGORY, OCR_ACTIONS);
    snapshot.translation_hotkeys = rows(TRANSLATION_CATEGORY, TRANSLATION_ACTIONS);
    snapshot.screenshot_hotkeys = SCREENSHOT_ACTIONS.map(|action| Hotkey {
        value: hotkey_category(hotkeys, SCREENSHOT_CATEGORY)
            .and_then(|keys| keys.get(action))
            .cloned()
            .unwrap_or(HOTKEY_UNSET.into()),
        default_value: hotkey_category(defaults, SCREENSHOT_CATEGORY)
            .and_then(|keys| keys.get(action))
            .cloned()
            .unwrap_or(HOTKEY_UNSET.into()),
    });
    snapshot.capacity.retention_days = saved.history.retention_days.into();
    snapshot.capacity.history_limit = saved.history.maximum_records;
    snapshot.capacity.favorites_limit = saved.history.maximum_favorites;
    snapshot
}

pub fn validate_hotkey_conflicts(snapshot: &HotkeySettingsSnapshot) -> Result<(), String> {
    let mut owners = BTreeMap::new();
    for binding in DEFAULT_HOTKEYS {
        let Some(shortcut) =
            hotkey_category(snapshot, binding.category).and_then(|keys| keys.get(binding.action))
        else {
            continue;
        };
        if let Some(accelerator) =
            display_hotkey_to_accelerator(shortcut).map_err(|error| error.to_string())?
        {
            let owner = format!("{}:{}", binding.category, binding.action);
            let mut keys = accelerator
                .split('+')
                .map(|part| match part {
                    "CmdOrCtrl" if cfg!(target_os = "macos") => "Meta",
                    "CmdOrCtrl" => "Ctrl",
                    other => other,
                })
                .collect::<Vec<_>>();
            keys.sort_unstable();
            keys.dedup();
            if let Some(previous) = owners.insert(keys.join("+"), owner.clone()) {
                return Err(format!("快捷键冲突：{previous} 与 {owner} 使用相同快捷键"));
            }
        }
    }
    Ok(())
}

pub fn general_value(value: Value) -> GeneralValue {
    match value {
        Value::Toggle(value) => GeneralValue::Toggle(value),
        Value::Text(value) => GeneralValue::Text(value),
        Value::Number(value) => GeneralValue::Number(value),
    }
}

pub fn general_field(field: GeneralField) -> GeneralSetting {
    match field {
        GeneralField::InterfaceTheme => GeneralSetting::InterfaceTheme,
        GeneralField::Language => GeneralSetting::Language,
        GeneralField::Appearance => GeneralSetting::Appearance,
        GeneralField::StartOnBoot => GeneralSetting::StartOnBoot,
        GeneralField::ProxyMode => GeneralSetting::ProxyMode,
        GeneralField::ProxyUrl => GeneralSetting::ProxyUrl,
        GeneralField::RequestTimeout => GeneralSetting::RequestTimeout,
        GeneralField::RetryCount => GeneralSetting::RetryCount,
        GeneralField::LogLevel => GeneralSetting::LogLevel,
        GeneralField::LogRetention => GeneralSetting::LogRetention,
        GeneralField::GpuComposition => GeneralSetting::GpuComposition,
        GeneralField::PerformanceMonitoring => GeneralSetting::PerformanceMonitoring,
    }
}

pub fn ocr_field(field: OcrField) -> Field {
    match field {
        OcrField::GpuAcceleration => Field::GpuAcceleration,
        OcrField::RecognitionLanguage => Field::RecognitionLanguage,
        OcrField::PreserveFormatting => Field::PreserveFormatting,
        OcrField::RemoveChineseSpaces => Field::RemoveChineseSpaces,
        OcrField::ShowConfidence => Field::ShowConfidence,
        OcrField::WindowPosition => Field::WindowPosition,
        OcrField::HideSilentStatus => Field::HideSilentStatus,
    }
}

pub fn screenshot_field(field: ScreenshotField) -> ScreenshotSetting {
    match field {
        ScreenshotField::SavePath => ScreenshotSetting::SavePath,
        ScreenshotField::Format => ScreenshotSetting::Format,
        ScreenshotField::Quality => ScreenshotSetting::Quality,
        ScreenshotField::NamingRule => ScreenshotSetting::NamingRule,
        ScreenshotField::CustomFileName => ScreenshotSetting::CustomFileName,
        ScreenshotField::AutoCopy => ScreenshotSetting::AutoCopy,
        ScreenshotField::LongshotMaxHeight => ScreenshotSetting::LongshotMaxHeight,
        ScreenshotField::SelectionBorderWidth => ScreenshotSetting::SelectionBorderWidth,
        ScreenshotField::SelectionBorderColor => ScreenshotSetting::SelectionBorderColor,
        ScreenshotField::SelectionMaskColor => ScreenshotSetting::SelectionMaskColor,
        ScreenshotField::MaskOpacity => ScreenshotSetting::MaskOpacity,
        ScreenshotField::RememberLastTool => ScreenshotSetting::RememberLastTool,
        ScreenshotField::ShowSelectionSize => ScreenshotSetting::ShowSelectionSize,
        ScreenshotField::ShowMagnifier => ScreenshotSetting::ShowMagnifier,
        ScreenshotField::MagnifierZoom => ScreenshotSetting::MagnifierZoom,
        ScreenshotField::PinOpacity => ScreenshotSetting::PinOpacity,
        ScreenshotField::PinShadow => ScreenshotSetting::PinShadow,
        ScreenshotField::PinPosition => ScreenshotSetting::PinPosition,
        ScreenshotField::PinBorder => ScreenshotSetting::PinBorder,
        ScreenshotField::CaptureUiScale => ScreenshotSetting::CaptureUiScale,
        ScreenshotField::AnnotationShortcut(tool) => ScreenshotSetting::AnnotationShortcut(tool),
        ScreenshotField::ResetAnnotationShortcuts => ScreenshotSetting::ResetAnnotationShortcuts,
        ScreenshotField::PinBorderShortcut => ScreenshotSetting::PinBorderShortcut,
    }
}

pub fn translation_field(field: TranslationField) -> TranslationSetting {
    match field {
        TranslationField::DefaultSourceLang => TranslationSetting::DefaultSourceLang,
        TranslationField::DefaultTargetLang => TranslationSetting::DefaultTargetLang,
        TranslationField::SelectionTextMode => TranslationSetting::SelectionTextMode,
        TranslationField::SelectionWindowPosition => TranslationSetting::SelectionWindowPosition,
        TranslationField::InputWindowPosition => TranslationSetting::InputWindowPosition,
        TranslationField::MaxWindowHeightRatio => TranslationSetting::MaxWindowHeightRatio,
        TranslationField::WindowWidth => TranslationSetting::WindowWidth,
        TranslationField::SelectionInputState => TranslationSetting::SelectionInputState,
        TranslationField::ScreenshotInputState => TranslationSetting::ScreenshotInputState,
        TranslationField::AutoTranslate => TranslationSetting::AutoTranslate,
        TranslationField::AutoCopy => TranslationSetting::AutoCopy,
        TranslationField::PreserveLineBreaks => TranslationSetting::PreserveLineBreaks,
        TranslationField::IncrementalTranslation => TranslationSetting::IncrementalTranslation,
        TranslationField::WindowAlwaysOnTop => TranslationSetting::WindowAlwaysOnTop,
        TranslationField::HideOnBlur => TranslationSetting::HideOnBlur,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn general_edits_preserve_other_preferences_and_survive_reopen() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("general.db");
        let mut backend =
            PersistentSettings::open(&path, Arc::new(RecordingOnlyRegistrar)).unwrap();
        let before = backend.snapshot().unwrap();
        for (field, value) in [
            (GeneralField::InterfaceTheme, Value::Text("studio".into())),
            (GeneralField::Appearance, Value::Text("dark".into())),
            (GeneralField::ProxyMode, Value::Text("manual".into())),
            (
                GeneralField::ProxyUrl,
                Value::Text("socks5://127.0.0.1:1080".into()),
            ),
            (GeneralField::RequestTimeout, Value::Number(24_000)),
            (GeneralField::RetryCount, Value::Number(3)),
            (GeneralField::LogRetention, Value::Number(14)),
            (GeneralField::PerformanceMonitoring, Value::Toggle(true)),
        ] {
            backend
                .dispatch(Page::General, Action::EditGeneral { field, value })
                .unwrap();
        }
        let saved = backend.snapshot().unwrap();
        assert_eq!(saved.theme, Theme::Studio);
        assert!(saved.dark);
        assert_eq!(saved.general.request_timeout_ms, 24_000);
        assert_eq!(saved.ocr, before.ocr);
        assert_eq!(saved.screenshot, before.screenshot);
        assert_eq!(saved.translation, before.translation);
        for (field, value) in [
            (GeneralField::Appearance, Value::Text("unknown".into())),
            (GeneralField::RetryCount, Value::Number(6)),
            (GeneralField::LogLevel, Value::Toggle(true)),
        ] {
            assert!(backend
                .dispatch(Page::General, Action::EditGeneral { field, value })
                .is_err());
        }
        assert_eq!(backend.snapshot().unwrap(), saved);
        drop(backend);
        assert_eq!(
            PersistentSettings::open(&path, Arc::new(RecordingOnlyRegistrar))
                .unwrap()
                .snapshot()
                .unwrap(),
            saved
        );
    }
    #[test]
    fn startup_edits_use_real_host_adapter_and_failure_does_not_persist() {
        struct Startup(std::sync::atomic::AtomicBool);
        impl StartOnBoot for Startup {
            fn set_enabled(&self, enabled: bool) -> snaplingo_core::Result<()> {
                self.0.store(enabled, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            }
        }
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("startup.db");
        let mut preview =
            PersistentSettings::open(&path, Arc::new(RecordingOnlyRegistrar)).unwrap();
        assert!(preview
            .dispatch(
                Page::General,
                Action::EditGeneral {
                    field: GeneralField::StartOnBoot,
                    value: Value::Toggle(true)
                }
            )
            .unwrap_err()
            .contains("适配器"));
        assert!(!preview.snapshot().unwrap().general.start_on_boot);
        drop(preview);
        let startup = Arc::new(Startup(std::sync::atomic::AtomicBool::new(false)));
        let mut backend = PersistentSettings::open_with_start_on_boot(
            &path,
            Arc::new(RecordingOnlyRegistrar),
            startup.clone(),
        )
        .unwrap();
        backend
            .dispatch(
                Page::General,
                Action::EditGeneral {
                    field: GeneralField::StartOnBoot,
                    value: Value::Toggle(true),
                },
            )
            .unwrap();
        assert!(startup.0.load(std::sync::atomic::Ordering::SeqCst));
        assert!(backend.snapshot().unwrap().general.start_on_boot);
    }
    use super::*;
    use snaplingo_ui::{Event, Point, Section, SettingsUi, Target};

    #[derive(Default)]
    struct Registrar {
        calls: std::sync::Mutex<Vec<String>>,
        fail: std::sync::atomic::AtomicBool,
    }
    impl HotkeyRegistrar for Registrar {
        fn register(&self, registration: HotkeyRegistration) -> snaplingo_core::Result<()> {
            self.calls.lock().unwrap().push(format!(
                "register:{}:{}:{}",
                registration.category, registration.action, registration.accelerator
            ));
            if self.fail.load(std::sync::atomic::Ordering::SeqCst) {
                Err("fixture registration denied".into())
            } else {
                Ok(())
            }
        }
        fn unregister(&self, accelerator: &str) -> snaplingo_core::Result<()> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("unregister:{accelerator}"));
            Ok(())
        }
    }

    #[test]
    fn hotkeys_use_real_runtime_category_mapping_and_survive_reopen() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("preferences.db");
        let registrar = Arc::new(Registrar::default());
        let mut backend = PersistentSettings::open(&path, registrar.clone()).unwrap();
        assert!(
            registrar.calls.lock().unwrap().is_empty(),
            "Opening preferences must not install startup shortcuts"
        );
        backend
            .dispatch(Page::Ocr, Action::BeginHotkeyRecording { index: 0 })
            .unwrap();
        assert!(backend
            .recording
            .session()
            .unwrap()
            .id
            .starts_with("ocr:screenshot-ocr:"));
        backend
            .dispatch(
                Page::Ocr,
                Action::SaveHotkey {
                    index: 0,
                    shortcut: "F18".into(),
                },
            )
            .unwrap();
        assert!(backend.recording.session().is_none());
        backend
            .dispatch(
                Page::Translation,
                Action::SaveHotkey {
                    index: 0,
                    shortcut: "F19".into(),
                },
            )
            .unwrap();
        backend
            .dispatch(Page::Translation, Action::BeginHotkeyRecording { index: 1 })
            .unwrap();
        backend
            .dispatch(Page::Translation, Action::EndHotkeyRecording)
            .unwrap();
        assert!(backend.recording.session().is_none());
        let snapshot = backend.snapshot().unwrap();
        assert_eq!(snapshot.hotkeys[0].value, "F18");
        assert_eq!(snapshot.translation_hotkeys[0].value, "F19");
        assert_eq!(snapshot.hotkeys[0].default_value, "⇧⌥S");
        let calls = registrar.calls.lock().unwrap();
        assert!(calls
            .iter()
            .any(|call| call == "register:ocr:screenshot-ocr:F18"));
        assert!(calls
            .iter()
            .any(|call| call == "register:translation:selection-translate:F19"));
        drop(calls);
        drop(backend);
        let calls = registrar.calls.lock().unwrap();
        assert!(calls.iter().any(|call| call == "unregister:F18"));
        assert!(calls.iter().any(|call| call == "unregister:F19"));
        assert_eq!(
            calls
                .iter()
                .filter(|call| call.starts_with("unregister:"))
                .count(),
            2,
            "Teardown must release only shortcuts created through this adapter"
        );
        drop(calls);
        let reopened = PersistentSettings::open(&path, Arc::new(RecordingOnlyRegistrar))
            .unwrap()
            .snapshot()
            .unwrap();
        assert_eq!(reopened.hotkeys[0].value, "F18");
        assert_eq!(reopened.translation_hotkeys[0].value, "F19");
    }

    #[test]
    fn conflicts_registration_failure_clear_and_resets_preserve_durable_state() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("preferences.db");
        let registrar = Arc::new(Registrar::default());
        let mut backend = PersistentSettings::open(&path, registrar.clone()).unwrap();
        let initial = backend.snapshot().unwrap();
        // These collide with another feature category, including equivalent modifier orders.
        for shortcut in ["⌥D", "⌥⇧S", "F1"] {
            assert!(backend
                .dispatch(
                    Page::Ocr,
                    Action::SaveHotkey {
                        index: 1,
                        shortcut: shortcut.into()
                    }
                )
                .unwrap_err()
                .contains("冲突"));
        }
        assert_eq!(backend.snapshot().unwrap(), initial);
        assert!(registrar.calls.lock().unwrap().is_empty());
        registrar
            .fail
            .store(true, std::sync::atomic::Ordering::SeqCst);
        assert!(backend
            .dispatch(
                Page::Ocr,
                Action::SaveHotkey {
                    index: 0,
                    shortcut: "F18".into()
                }
            )
            .unwrap_err()
            .contains("registration denied"));
        assert_eq!(backend.snapshot().unwrap(), initial);
        registrar
            .fail
            .store(false, std::sync::atomic::Ordering::SeqCst);
        backend
            .dispatch(
                Page::Ocr,
                Action::SaveHotkey {
                    index: 0,
                    shortcut: "F18".into(),
                },
            )
            .unwrap();
        backend
            .dispatch(
                Page::Ocr,
                Action::SaveHotkey {
                    index: 0,
                    shortcut: HOTKEY_UNSET.into(),
                },
            )
            .unwrap();
        assert_eq!(backend.snapshot().unwrap().hotkeys[0].value, HOTKEY_UNSET);
        assert!(registrar
            .calls
            .lock()
            .unwrap()
            .iter()
            .any(|call| call == "unregister:F18"));
        backend
            .dispatch(Page::Ocr, Action::ResetHotkey { index: 0 })
            .unwrap();
        assert_eq!(backend.snapshot().unwrap().hotkeys, initial.hotkeys);
        backend
            .dispatch(
                Page::Translation,
                Action::SaveHotkey {
                    index: 0,
                    shortcut: "F19".into(),
                },
            )
            .unwrap();
        backend
            .dispatch(
                Page::Translation,
                Action::SaveHotkey {
                    index: 1,
                    shortcut: "F20".into(),
                },
            )
            .unwrap();
        // The host has confirmed this group reset before delivering the action.
        backend
            .dispatch(Page::Translation, Action::ResetHotkeys)
            .unwrap();
        assert_eq!(
            backend.snapshot().unwrap().translation_hotkeys,
            initial.translation_hotkeys
        );
        backend
            .dispatch(Page::Translation, Action::CheckHotkeyConflicts)
            .unwrap();
    }

    #[test]
    fn translation_toggles_choices_and_ranges_use_the_real_field_edits() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("preferences.db");
        let mut backend =
            PersistentSettings::open(&path, Arc::new(RecordingOnlyRegistrar)).unwrap();
        for (field, value) in [
            (
                TranslationField::DefaultTargetLang,
                Value::Text("ja".into()),
            ),
            (TranslationField::AutoCopy, Value::Toggle(true)),
            (TranslationField::WindowWidth, Value::Number(780)),
        ] {
            backend
                .dispatch(Page::Translation, Action::EditTranslation { field, value })
                .unwrap();
        }
        let source = backend.snapshot().unwrap();
        assert_eq!(source.translation.default_target_lang, "ja");
        assert!(source.translation.auto_copy);
        assert_eq!(source.translation.window_width, 780);
        assert!(backend
            .dispatch(
                Page::Translation,
                Action::EditTranslation {
                    field: TranslationField::DefaultTargetLang,
                    value: Value::Text("invalid-language".into())
                }
            )
            .is_err());
        assert_eq!(backend.snapshot().unwrap(), source);
    }

    #[test]
    fn imported_duplicate_shortcuts_are_reported_without_mutating_preferences() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("preferences.db");
        let store = SqliteConfigStore::new(Arc::new(Database::open(&path).unwrap()));
        let mut saved = hotkey_config::default_hotkey_snapshot();
        saved
            .ocr
            .insert(hotkey_config::FILE_OCR_ACTION.into(), "⌥D".into());
        snaplingo_core::application::hotkeys::HotkeyStore::save_hotkeys(&store, &saved).unwrap();
        let mut backend =
            PersistentSettings::open(&path, Arc::new(RecordingOnlyRegistrar)).unwrap();
        let before = backend.snapshot().unwrap();
        assert!(backend
            .dispatch(Page::Ocr, Action::CheckHotkeyConflicts)
            .unwrap_err()
            .contains("translation:selection-translate"));
        assert_eq!(backend.snapshot().unwrap(), before);
    }

    #[test]
    fn screenshot_text_colors_local_shortcuts_and_dialog_choices_persist() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("preferences.db");
        let mut backend =
            PersistentSettings::open(&path, Arc::new(RecordingOnlyRegistrar)).unwrap();
        backend
            .configuration
            .record_screenshot_save_choice(Path::new("/tmp/chosen.webp"), "webp")
            .unwrap();
        for (field, value) in [
            (
                ScreenshotField::SavePath,
                Value::Text(directory.path().join("截图").to_string_lossy().into()),
            ),
            (ScreenshotField::NamingRule, Value::Text("custom".into())),
            (
                ScreenshotField::CustomFileName,
                Value::Text("截图_🙂".into()),
            ),
            (ScreenshotField::LongshotMaxHeight, Value::Number(24_000)),
            (
                ScreenshotField::SelectionBorderColor,
                Value::Number(u32::from_be_bytes([91, 127, 255, 120])),
            ),
            (
                ScreenshotField::SelectionMaskColor,
                Value::Number(u32::from_be_bytes([25, 82, 72, 64])),
            ),
            (ScreenshotField::MaskOpacity, Value::Number(67)),
            (
                ScreenshotField::PinBorderShortcut,
                Value::Text("SHIFT+CTRL+F4".into()),
            ),
        ] {
            backend
                .dispatch(Page::Screenshot, Action::EditScreenshot { field, value })
                .unwrap();
        }
        backend
            .dispatch(
                Page::Screenshot,
                Action::BeginLocalShortcutRecording {
                    field: ScreenshotField::AnnotationShortcut("arrow"),
                },
            )
            .unwrap();
        assert!(backend
            .recording
            .session()
            .unwrap()
            .id
            .starts_with("local:"));
        backend
            .dispatch(
                Page::Screenshot,
                Action::EditScreenshot {
                    field: ScreenshotField::AnnotationShortcut("arrow"),
                    value: Value::Text(" F24 ".into()),
                },
            )
            .unwrap();
        assert!(backend.recording.session().is_none());
        let saved = backend.snapshot().unwrap().screenshot;
        assert_eq!(saved.custom_file_name, "截图_🙂");
        assert_eq!(saved.longshot_max_height, 24_000);
        assert_eq!(saved.selection_border_color, [91, 127, 255, 120]);
        assert_eq!(saved.selection_mask_color, [25, 82, 72, 171]);
        assert_eq!(saved.annotation_tool_shortcuts["arrow"], "f24");
        assert_eq!(saved.pin_border_shortcut, "ctrl+shift+f4");
        assert_eq!(saved.last_save_directory.as_deref(), Some("/tmp"));
        assert_eq!(saved.last_save_format.as_deref(), Some("webp"));
        drop(backend);
        let mut backend =
            PersistentSettings::open(&path, Arc::new(RecordingOnlyRegistrar)).unwrap();
        assert_eq!(backend.snapshot().unwrap().screenshot, saved);
        backend
            .dispatch(
                Page::Screenshot,
                Action::EditScreenshot {
                    field: ScreenshotField::ResetAnnotationShortcuts,
                    value: Value::Toggle(true),
                },
            )
            .unwrap();
        assert_eq!(
            backend
                .snapshot()
                .unwrap()
                .screenshot
                .annotation_tool_shortcuts,
            snaplingo_core::domain::config::default_annotation_tool_shortcuts()
        );
        assert_eq!(
            backend.snapshot().unwrap().screenshot.pin_border_shortcut,
            "ctrl+shift+f4"
        );
    }

    #[test]
    fn invalid_screenshot_controls_do_not_write_and_native_actions_need_a_host() {
        let directory = tempfile::tempdir().unwrap();
        let mut backend = PersistentSettings::open(
            directory.path().join("preferences.db"),
            Arc::new(RecordingOnlyRegistrar),
        )
        .unwrap();
        let before = backend.snapshot().unwrap();
        for (field, value) in [
            (ScreenshotField::SavePath, Value::Number(10)),
            (ScreenshotField::LongshotMaxHeight, Value::Number(99)),
            (ScreenshotField::Format, Value::Text("gif".into())),
            (
                ScreenshotField::AnnotationShortcut("arrow"),
                Value::Text("t".into()),
            ),
            (
                ScreenshotField::PinBorderShortcut,
                Value::Text("ctrl+ctrl+b".into()),
            ),
        ] {
            assert!(backend
                .dispatch(Page::Screenshot, Action::EditScreenshot { field, value })
                .is_err());
        }
        assert_eq!(backend.snapshot().unwrap(), before);
        for action in [
            Action::SelectScreenshotDirectory,
            Action::ReadClipboard,
            Action::CopyToClipboard("text".into()),
            Action::SelectScreenshotColor {
                field: ScreenshotField::SelectionMaskColor,
                rgba: [0, 0, 0, 46],
            },
        ] {
            assert!(backend
                .dispatch(Page::Screenshot, action)
                .unwrap_err()
                .contains("宿主"));
        }
        assert_eq!(backend.snapshot().unwrap(), before);
        // Keep the existing workflow's normalized default for a blank path.
        backend
            .dispatch(
                Page::Screenshot,
                Action::EditScreenshot {
                    field: ScreenshotField::SavePath,
                    value: Value::Text(" ".into()),
                },
            )
            .unwrap();
        assert!(!backend
            .snapshot()
            .unwrap()
            .screenshot
            .save_path
            .trim()
            .is_empty());
    }

    #[test]
    fn screenshot_five_global_hotkeys_share_runtime_conflicts_clear_and_reset() {
        let directory = tempfile::tempdir().unwrap();
        let mut backend = PersistentSettings::open(
            directory.path().join("preferences.db"),
            Arc::new(RecordingOnlyRegistrar),
        )
        .unwrap();
        let defaults = backend.snapshot().unwrap().screenshot_hotkeys;
        for index in 0..5 {
            backend
                .dispatch(
                    Page::Screenshot,
                    Action::SaveHotkey {
                        index,
                        shortcut: format!("F{}", 18 + index),
                    },
                )
                .unwrap();
        }
        assert_eq!(
            backend.snapshot().unwrap().screenshot_hotkeys[4].value,
            "F22"
        );
        assert!(backend
            .dispatch(
                Page::Translation,
                Action::SaveHotkey {
                    index: 0,
                    shortcut: "F22".into()
                }
            )
            .unwrap_err()
            .contains("冲突"));
        backend
            .dispatch(
                Page::Screenshot,
                Action::SaveHotkey {
                    index: 4,
                    shortcut: HOTKEY_UNSET.into(),
                },
            )
            .unwrap();
        assert_eq!(
            backend.snapshot().unwrap().screenshot_hotkeys[4].value,
            HOTKEY_UNSET
        );
        backend
            .dispatch(Page::Screenshot, Action::ResetHotkey { index: 4 })
            .unwrap();
        assert_eq!(
            backend.snapshot().unwrap().screenshot_hotkeys[4],
            defaults[4]
        );
        backend
            .dispatch(Page::Screenshot, Action::ResetHotkeys)
            .unwrap();
        assert_eq!(backend.snapshot().unwrap().screenshot_hotkeys, defaults);
    }

    #[test]
    fn pointer_edit_reaches_the_real_workflow_and_survives_database_reopen() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.db");
        let store = Arc::new(SqliteConfigStore::new(Arc::new(
            Database::open(&path).unwrap(),
        )));
        let mut original = SettingsSnapshot::default();
        original.general.proxy_url = "http://127.0.0.1:7890".into();
        original.translation.default_target_lang = "ja".into();
        store.save_settings(&original).unwrap();
        let mut backend =
            PersistentSettings::open(&path, Arc::new(RecordingOnlyRegistrar)).unwrap();
        let mut ui = SettingsUi::new(backend.snapshot().unwrap());
        let size = Size {
            width: 900.0,
            height: 650.0,
        };
        let scene = ui.frame(size, 1.0);
        let tab = scene
            .hits
            .iter()
            .find(|hit| hit.target == Target::Section(Section::Recognition))
            .unwrap();
        ui.update(Event::PointerDown(Point {
            x: tab.rect.x + 1.0,
            y: tab.rect.y + 1.0,
        }));
        let scene = ui.frame(size, 1.0);
        let control = scene
            .hits
            .iter()
            .find(|hit| hit.target == Target::Ocr(OcrField::GpuAcceleration))
            .unwrap();
        let actions = ui.update(Event::PointerDown(Point {
            x: control.rect.x + 1.0,
            y: control.rect.y + 1.0,
        }));
        assert_eq!(actions.len(), 1);
        backend.dispatch(Page::Ocr, actions[0].clone()).unwrap();
        assert!(!backend.snapshot().unwrap().ocr.gpu_acceleration);
        drop(backend);
        drop(store);
        let reopened = SqliteConfigStore::new(Arc::new(Database::open(&path).unwrap()))
            .load_settings()
            .unwrap();
        assert!(!reopened.ocr.gpu_acceleration);
        assert_eq!(reopened.general.proxy_url, original.general.proxy_url);
        assert_eq!(reopened.translation, original.translation);
    }
}

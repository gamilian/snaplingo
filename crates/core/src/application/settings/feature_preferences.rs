use crate::domain::{OcrSettings, TranslationSettings};

use super::GeneralValue;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeaturePreferences {
    pub ocr: OcrSettings,
    pub translation: TranslationSettings,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OcrField {
    GpuAcceleration,
    RecognitionLanguage,
    PreserveFormatting,
    RemoveChineseSpaces,
    ShowConfidence,
    WindowPosition,
    HideSilentStatus,
}

const OCR_LANGUAGES: &[(&str, &str)] = &[
    ("auto", "自动检测"),
    ("zh-CN", "中文简体"),
    ("zh-TW", "中文繁體"),
    ("en", "English"),
    ("ja", "日本語"),
    ("ko", "한국어"),
];
const POSITIONS: &[(&str, &str)] = &[
    ("center", "居中"),
    ("below-cursor", "鼠标下方"),
    ("cursor", "鼠标位置"),
    ("last-position", "上次停留位置"),
];
const SOURCE_LANGUAGES: &[(&str, &str)] = &[
    ("auto", "自动检测"),
    ("en", "英语"),
    ("zh-CN", "中文简体"),
    ("zh-TW", "中文繁体"),
    ("ja", "日语"),
    ("ko", "韩语"),
    ("es", "西班牙语"),
    ("fr", "法语"),
    ("de", "德语"),
    ("ru", "俄语"),
    ("ar", "阿拉伯语"),
];
const TARGET_LANGUAGES: &[(&str, &str)] = &[
    ("auto", "自动"),
    ("en", "英语"),
    ("zh-CN", "中文简体"),
    ("zh-TW", "中文繁体"),
    ("ja", "日语"),
    ("ko", "韩语"),
    ("es", "西班牙语"),
    ("fr", "法语"),
    ("de", "德语"),
    ("ru", "俄语"),
    ("ar", "阿拉伯语"),
];
const INPUT_STATES: &[(&str, &str)] = &[
    ("last", "上次状态"),
    ("collapsed", "总是折叠"),
    ("expanded", "总是展开"),
];
const SELECTION_MODES: &[(&str, &str)] = &[
    ("smart", "智能模式"),
    ("quality", "效果优先"),
    ("speed", "速度优先"),
];

impl OcrField {
    pub const ALL: [Self; 7] = [
        Self::GpuAcceleration,
        Self::RecognitionLanguage,
        Self::PreserveFormatting,
        Self::RemoveChineseSpaces,
        Self::ShowConfidence,
        Self::WindowPosition,
        Self::HideSilentStatus,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::GpuAcceleration => "本地模型 GPU 加速",
            Self::RecognitionLanguage => "识别语言",
            Self::PreserveFormatting => "保留文本格式",
            Self::RemoveChineseSpaces => "中文自动去除空格",
            Self::ShowConfidence => "显示识别置信度",
            Self::WindowPosition => "OCR 窗口位置",
            Self::HideSilentStatus => "隐藏识别状态提示",
        }
    }

    pub fn choices(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Self::RecognitionLanguage => OCR_LANGUAGES,
            Self::WindowPosition => POSITIONS,
            _ => &[],
        }
    }

    pub fn value(self, settings: &OcrSettings) -> GeneralValue {
        use GeneralValue::*;
        match self {
            Self::GpuAcceleration => Toggle(settings.gpu_acceleration),
            Self::RecognitionLanguage => Text(settings.recognition_language.clone()),
            Self::PreserveFormatting => Toggle(settings.preserve_formatting),
            Self::RemoveChineseSpaces => Toggle(settings.remove_chinese_spaces),
            Self::ShowConfidence => Toggle(settings.show_confidence),
            Self::WindowPosition => Text(settings.window_position.clone()),
            Self::HideSilentStatus => Toggle(settings.hide_silent_status),
        }
    }

    pub(super) fn apply(
        self,
        settings: &mut OcrSettings,
        value: GeneralValue,
    ) -> crate::Result<()> {
        use GeneralValue::*;
        match (self, value) {
            (Self::GpuAcceleration, Toggle(value)) => settings.gpu_acceleration = value,
            (Self::RecognitionLanguage, Text(value)) => {
                validate_choice(&value, self.choices())?;
                settings.recognition_language = value;
            }
            (Self::PreserveFormatting, Toggle(value)) => settings.preserve_formatting = value,
            (Self::RemoveChineseSpaces, Toggle(value)) => settings.remove_chinese_spaces = value,
            (Self::ShowConfidence, Toggle(value)) => settings.show_confidence = value,
            (Self::WindowPosition, Text(value)) => {
                validate_choice(&value, self.choices())?;
                settings.window_position = value;
            }
            (Self::HideSilentStatus, Toggle(value)) => settings.hide_silent_status = value,
            _ => return Err("Setting value does not match its control".into()),
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TranslationField {
    DefaultSourceLang,
    DefaultTargetLang,
    SelectionTextMode,
    SelectionWindowPosition,
    InputWindowPosition,
    MaxWindowHeightRatio,
    WindowWidth,
    SelectionInputState,
    ScreenshotInputState,
    AutoTranslate,
    IncrementalTranslation,
    AutoCopy,
    PreserveLineBreaks,
    WindowAlwaysOnTop,
    HideOnBlur,
}

impl TranslationField {
    pub const ALL: [Self; 15] = [
        Self::DefaultSourceLang,
        Self::DefaultTargetLang,
        Self::SelectionTextMode,
        Self::SelectionWindowPosition,
        Self::InputWindowPosition,
        Self::MaxWindowHeightRatio,
        Self::WindowWidth,
        Self::SelectionInputState,
        Self::ScreenshotInputState,
        Self::AutoTranslate,
        Self::IncrementalTranslation,
        Self::AutoCopy,
        Self::PreserveLineBreaks,
        Self::WindowAlwaysOnTop,
        Self::HideOnBlur,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::DefaultSourceLang => "源语言",
            Self::DefaultTargetLang => "目标语言",
            Self::SelectionTextMode => "取词模式",
            Self::SelectionWindowPosition => "划词翻译与截图翻译",
            Self::InputWindowPosition => "手动翻译窗口",
            Self::MaxWindowHeightRatio => "最高屏幕占比",
            Self::WindowWidth => "窗口宽度",
            Self::SelectionInputState => "划词翻译",
            Self::ScreenshotInputState => "截图翻译",
            Self::AutoTranslate => "自动翻译",
            Self::IncrementalTranslation => "增量翻译",
            Self::AutoCopy => "翻译后自动复制",
            Self::PreserveLineBreaks => "保留原文换行",
            Self::WindowAlwaysOnTop => "结果窗口置顶",
            Self::HideOnBlur => "失去焦点时隐藏",
        }
    }

    pub fn choices(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Self::DefaultSourceLang => SOURCE_LANGUAGES,
            Self::DefaultTargetLang => TARGET_LANGUAGES,
            Self::SelectionTextMode => SELECTION_MODES,
            Self::SelectionWindowPosition | Self::InputWindowPosition => POSITIONS,
            Self::SelectionInputState | Self::ScreenshotInputState => INPUT_STATES,
            _ => &[],
        }
    }

    pub fn value(self, settings: &TranslationSettings) -> GeneralValue {
        use GeneralValue::*;
        match self {
            Self::DefaultSourceLang => Text(settings.default_source_lang.clone()),
            Self::DefaultTargetLang => Text(settings.default_target_lang.clone()),
            Self::SelectionTextMode => Text(settings.selection_text_mode.clone()),
            Self::SelectionWindowPosition => Text(settings.selection_window_position.clone()),
            Self::InputWindowPosition => Text(settings.input_window_position.clone()),
            Self::MaxWindowHeightRatio => Number(settings.max_window_height_ratio.into()),
            Self::WindowWidth => Number(settings.window_width.into()),
            Self::SelectionInputState => Text(settings.selection_input_state.clone()),
            Self::ScreenshotInputState => Text(settings.screenshot_input_state.clone()),
            Self::AutoTranslate => Toggle(settings.auto_translate),
            Self::IncrementalTranslation => Toggle(settings.incremental_translation),
            Self::AutoCopy => Toggle(settings.auto_copy),
            Self::PreserveLineBreaks => Toggle(settings.preserve_line_breaks),
            Self::WindowAlwaysOnTop => Toggle(settings.window_always_on_top),
            Self::HideOnBlur => Toggle(settings.hide_on_blur),
        }
    }

    pub(super) fn apply(
        self,
        settings: &mut TranslationSettings,
        value: GeneralValue,
    ) -> crate::Result<()> {
        use GeneralValue::*;
        match (self, value) {
            (Self::DefaultSourceLang, Text(value)) => {
                validate_choice(&value, self.choices())?;
                settings.default_source_lang = value;
            }
            (Self::DefaultTargetLang, Text(value)) => {
                validate_choice(&value, self.choices())?;
                settings.default_target_lang = value;
            }
            (Self::SelectionTextMode, Text(value)) => {
                validate_choice(&value, self.choices())?;
                settings.selection_text_mode = value;
            }
            (Self::SelectionWindowPosition, Text(value)) => {
                validate_choice(&value, self.choices())?;
                settings.selection_window_position = value;
            }
            (Self::InputWindowPosition, Text(value)) => {
                validate_choice(&value, self.choices())?;
                settings.input_window_position = value;
            }
            (Self::MaxWindowHeightRatio, Number(value)) if (30..=90).contains(&value) => {
                settings.max_window_height_ratio = value as u8
            }
            (Self::WindowWidth, Number(value)) if (300..=1000).contains(&value) => {
                settings.window_width = value as u16
            }
            (Self::SelectionInputState, Text(value)) => {
                validate_choice(&value, self.choices())?;
                settings.selection_input_state = value;
            }
            (Self::ScreenshotInputState, Text(value)) => {
                validate_choice(&value, self.choices())?;
                settings.screenshot_input_state = value;
            }
            (Self::AutoTranslate, Toggle(value)) => settings.auto_translate = value,
            (Self::IncrementalTranslation, Toggle(value)) => {
                settings.incremental_translation = value
            }
            (Self::AutoCopy, Toggle(value)) => settings.auto_copy = value,
            (Self::PreserveLineBreaks, Toggle(value)) => settings.preserve_line_breaks = value,
            (Self::WindowAlwaysOnTop, Toggle(value)) => settings.window_always_on_top = value,
            (Self::HideOnBlur, Toggle(value)) => settings.hide_on_blur = value,
            (Self::MaxWindowHeightRatio | Self::WindowWidth, Number(_)) => {
                return Err("Setting value is outside its supported range".into())
            }
            _ => return Err("Setting value does not match its control".into()),
        }
        Ok(())
    }
}

fn validate_choice(value: &str, choices: &[(&str, &str)]) -> crate::Result<()> {
    if choices.iter().any(|(id, _)| *id == value) {
        Ok(())
    } else {
        Err("Setting value is not one of its supported choices".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::settings::{
        GeneralField, SettingsApplication, SettingsChangeNotifier, SettingsConfiguration,
        SettingsStore, StartOnBoot,
    };
    use crate::application::History;
    use crate::domain::SettingsSnapshot;
    use crate::infrastructure::storage::{
        Database, SqliteAppLogRepository, SqliteHistoryRepository,
    };
    use std::sync::{Arc, Barrier, Mutex};

    #[derive(Default)]
    struct Store(Mutex<SettingsSnapshot>);
    impl SettingsStore for Store {
        fn load_settings(&self) -> crate::Result<SettingsSnapshot> {
            Ok(self.0.lock().unwrap().clone())
        }
        fn save_settings(&self, snapshot: &SettingsSnapshot) -> crate::Result<()> {
            *self.0.lock().unwrap() = snapshot.clone();
            Ok(())
        }
    }
    struct Committed {
        store: Arc<Store>,
        observed: Mutex<Vec<SettingsSnapshot>>,
    }
    impl SettingsChangeNotifier for Committed {
        fn settings_changed(&self) {
            self.observed
                .lock()
                .unwrap()
                .push(self.store.load_settings().unwrap());
        }
    }
    struct Boot;
    impl StartOnBoot for Boot {
        fn set_enabled(&self, _: bool) -> crate::Result<()> {
            Ok(())
        }
    }
    fn fixture() -> (
        Arc<SettingsApplication>,
        Arc<SettingsConfiguration>,
        Arc<Committed>,
    ) {
        let store = Arc::new(Store::default());
        let committed = Arc::new(Committed {
            store: store.clone(),
            observed: Mutex::new(Vec::new()),
        });
        let configuration = Arc::new(SettingsConfiguration::with_change_notifier(
            store,
            committed.clone(),
        ));
        let database = Arc::new(Database::in_memory().unwrap());
        let settings = Arc::new(SettingsApplication::new(
            configuration.clone(),
            Arc::new(Boot),
            Arc::new(History::new(Arc::new(SqliteHistoryRepository::new(
                database.clone(),
            )))),
            Arc::new(SqliteAppLogRepository::new(database)),
        ));
        (settings, configuration, committed)
    }

    #[test]
    fn native_feature_edits_persist_every_existing_ocr_and_translation_preference() {
        use GeneralValue::*;
        let (application, configuration, _) = fixture();
        for (field, value) in [
            (OcrField::GpuAcceleration, Toggle(false)),
            (OcrField::RecognitionLanguage, Text("ja".into())),
            (OcrField::PreserveFormatting, Toggle(false)),
            (OcrField::RemoveChineseSpaces, Toggle(false)),
            (OcrField::ShowConfidence, Toggle(true)),
            (OcrField::WindowPosition, Text("last-position".into())),
            (OcrField::HideSilentStatus, Toggle(true)),
        ] {
            application.edit_ocr(field, value).unwrap();
        }
        for (field, value) in [
            (TranslationField::DefaultSourceLang, Text("fr".into())),
            (TranslationField::DefaultTargetLang, Text("auto".into())),
            (TranslationField::SelectionTextMode, Text("quality".into())),
            (
                TranslationField::SelectionWindowPosition,
                Text("cursor".into()),
            ),
            (
                TranslationField::InputWindowPosition,
                Text("last-position".into()),
            ),
            (TranslationField::MaxWindowHeightRatio, Number(90)),
            (TranslationField::WindowWidth, Number(1000)),
            (
                TranslationField::SelectionInputState,
                Text("collapsed".into()),
            ),
            (
                TranslationField::ScreenshotInputState,
                Text("expanded".into()),
            ),
            (TranslationField::AutoTranslate, Toggle(false)),
            (TranslationField::IncrementalTranslation, Toggle(true)),
            (TranslationField::AutoCopy, Toggle(true)),
            (TranslationField::PreserveLineBreaks, Toggle(false)),
            (TranslationField::WindowAlwaysOnTop, Toggle(false)),
            (TranslationField::HideOnBlur, Toggle(false)),
        ] {
            application.edit_translation(field, value).unwrap();
        }
        let preferences = application.feature_preferences().unwrap();
        assert_eq!(
            preferences.ocr,
            OcrSettings {
                gpu_acceleration: false,
                recognition_language: "ja".into(),
                preserve_formatting: false,
                remove_chinese_spaces: false,
                show_confidence: true,
                window_position: "last-position".into(),
                hide_silent_status: true
            }
        );
        assert_eq!(
            preferences.translation,
            TranslationSettings {
                default_source_lang: "fr".into(),
                default_target_lang: "auto".into(),
                selection_text_mode: "quality".into(),
                selection_window_position: "cursor".into(),
                input_window_position: "last-position".into(),
                max_window_height_ratio: 90,
                window_width: 1000,
                selection_input_state: "collapsed".into(),
                screenshot_input_state: "expanded".into(),
                auto_translate: false,
                incremental_translation: true,
                auto_copy: true,
                preserve_line_breaks: false,
                window_always_on_top: false,
                hide_on_blur: false
            }
        );
        assert_eq!(
            configuration.snapshot().unwrap().general,
            SettingsSnapshot::default().general
        );
    }

    #[test]
    fn rejects_invalid_choices_wrong_controls_and_ranges_without_saving_or_notifying() {
        use GeneralValue::*;
        let (application, configuration, committed) = fixture();
        let before = configuration.snapshot().unwrap();
        for (field, value) in [
            (OcrField::RecognitionLanguage, Text("zh".into())),
            (OcrField::WindowPosition, Text("below_cursor".into())),
            (OcrField::ShowConfidence, Number(1)),
        ] {
            assert!(application.edit_ocr(field, value).is_err());
        }
        for (field, value) in [
            (TranslationField::DefaultTargetLang, Text("xx".into())),
            (TranslationField::SelectionInputState, Text("open".into())),
            (
                TranslationField::SelectionTextMode,
                Text("automatic".into()),
            ),
            (TranslationField::MaxWindowHeightRatio, Number(29)),
            (TranslationField::MaxWindowHeightRatio, Number(91)),
            (TranslationField::WindowWidth, Number(299)),
            (TranslationField::WindowWidth, Number(u32::MAX)),
            (TranslationField::AutoCopy, Text("true".into())),
        ] {
            assert!(application.edit_translation(field, value).is_err());
        }
        assert_eq!(configuration.snapshot().unwrap(), before);
        assert!(committed.observed.lock().unwrap().is_empty());
        application
            .edit_ocr(
                OcrField::GpuAcceleration,
                Toggle(before.ocr.gpu_acceleration),
            )
            .unwrap();
        application
            .edit_translation(
                TranslationField::AutoCopy,
                Toggle(before.translation.auto_copy),
            )
            .unwrap();
        assert!(
            committed.observed.lock().unwrap().is_empty(),
            "Unchanged controls must not write or emit settings changes"
        );
        application
            .edit_translation(TranslationField::WindowWidth, Number(300))
            .unwrap();
        application
            .edit_translation(TranslationField::MaxWindowHeightRatio, Number(30))
            .unwrap();
        assert_eq!(
            committed
                .observed
                .lock()
                .unwrap()
                .last()
                .unwrap()
                .translation
                .max_window_height_ratio,
            30
        );
    }

    #[test]
    fn choices_keep_the_original_codes_order_and_labels() {
        let ids = |choices: &[(&str, &str)]| {
            choices
                .iter()
                .map(|(id, _)| (*id).to_owned())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            ids(OcrField::RecognitionLanguage.choices()),
            ["auto", "zh-CN", "zh-TW", "en", "ja", "ko"]
        );
        assert_eq!(
            ids(TranslationField::DefaultSourceLang.choices()),
            ["auto", "en", "zh-CN", "zh-TW", "ja", "ko", "es", "fr", "de", "ru", "ar"]
        );
        assert_eq!(
            TranslationField::DefaultTargetLang.choices()[0],
            ("auto", "自动")
        );
        for field in [
            TranslationField::SelectionWindowPosition,
            TranslationField::InputWindowPosition,
        ] {
            assert_eq!(
                ids(field.choices()),
                ["center", "below-cursor", "cursor", "last-position"]
            );
        }
        assert_eq!(
            ids(TranslationField::SelectionTextMode.choices()),
            ["smart", "quality", "speed"]
        );
        assert_eq!(
            ids(TranslationField::SelectionInputState.choices()),
            ["last", "collapsed", "expanded"]
        );
        assert_eq!(OcrField::GpuAcceleration.label(), "本地模型 GPU 加速");
        assert_eq!(TranslationField::WindowWidth.label(), "窗口宽度");
    }

    #[test]
    fn concurrent_native_edits_preserve_other_controls_and_background_window_position() {
        let (application, configuration, committed) = fixture();
        let start = Arc::new(Barrier::new(4));
        let rounds = Arc::new(Mutex::new(Vec::new()));
        let mut threads = Vec::new();
        for task in 0..4 {
            let application = application.clone();
            let configuration = configuration.clone();
            let start = start.clone();
            let rounds = rounds.clone();
            threads.push(std::thread::spawn(move || {
                for turn in 0..40 {
                    start.wait();
                    match task {
                        0 => {
                            application
                                .edit_translation(
                                    TranslationField::WindowWidth,
                                    GeneralValue::Number(300 + turn * 10),
                                )
                                .unwrap();
                        }
                        1 => {
                            application
                                .edit_translation(
                                    TranslationField::MaxWindowHeightRatio,
                                    GeneralValue::Number(30 + turn),
                                )
                                .unwrap();
                        }
                        2 => {
                            application
                                .edit_general(
                                    GeneralField::PerformanceMonitoring,
                                    GeneralValue::Toggle(turn % 2 == 1),
                                )
                                .unwrap();
                        }
                        _ => {
                            configuration
                                .update_last_result_window_position(-(turn as i32), turn as i32)
                                .unwrap();
                        }
                    }
                    start.wait();
                    if task == 0 {
                        rounds
                            .lock()
                            .unwrap()
                            .push(configuration.snapshot().unwrap());
                    }
                    start.wait();
                }
            }));
        }
        for thread in threads {
            thread.join().unwrap();
        }
        for (turn, snapshot) in rounds.lock().unwrap().iter().enumerate() {
            assert_eq!(snapshot.translation.window_width, 300 + turn as u16 * 10);
            assert_eq!(
                snapshot.translation.max_window_height_ratio,
                30 + turn as u8
            );
            assert_eq!(snapshot.general.performance_monitoring, turn % 2 == 1);
            assert_eq!(snapshot.general.last_result_window_x, Some(-(turn as i32)));
        }
        let notifications = committed.observed.lock().unwrap();
        assert!(!notifications.is_empty());
        assert_eq!(
            notifications.last().unwrap(),
            &configuration.snapshot().unwrap(),
            "Subscribers must see committed settings"
        );
    }

    #[test]
    fn failed_general_field_commit_rolls_back_login_registration() {
        struct FailSave(Store);
        impl SettingsStore for FailSave {
            fn load_settings(&self) -> crate::Result<SettingsSnapshot> {
                self.0.load_settings()
            }
            fn save_settings(&self, _: &SettingsSnapshot) -> crate::Result<()> {
                Err("fixture settings storage failed".into())
            }
        }
        struct LoginRegistration(Mutex<Vec<bool>>);
        impl StartOnBoot for LoginRegistration {
            fn set_enabled(&self, enabled: bool) -> crate::Result<()> {
                self.0.lock().unwrap().push(enabled);
                Ok(())
            }
        }
        let configuration = SettingsConfiguration::new(Arc::new(FailSave(Store::default())));
        let login = LoginRegistration(Mutex::new(Vec::new()));
        assert!(configuration
            .edit_general(
                GeneralField::StartOnBoot,
                GeneralValue::Toggle(true),
                &login
            )
            .is_err());
        assert_eq!(*login.0.lock().unwrap(), [true, false]);
        assert!(!configuration.snapshot().unwrap().general.start_on_boot);
    }
}

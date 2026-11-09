use super::*;
use snaplingo_core::application::settings::GeneralValue;
use snaplingo_core::application::settings::*;
use snaplingo_core::application::{providers::common::CredentialField, providers::*, settings::*};
use snaplingo_core::domain::{OcrSettings, TranslationSettings};
mod cases {

    use super::*;
    use crate::infrastructure::storage::{
        Database, SqliteAppLogRepository, SqliteHistoryRepository,
    };
    use snaplingo_core::application::settings::{
        GeneralField, SettingsApplication, SettingsChangeNotifier, SettingsConfiguration,
        SettingsStore, StartOnBoot,
    };
    use snaplingo_core::application::History;
    use snaplingo_core::domain::SettingsSnapshot;
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

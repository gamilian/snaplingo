use super::*;
use crate::Result;
use serde::Serialize;
use snaplingo_core::hotkeys::configuration::HotkeyConfiguration;
use snaplingo_core::hotkeys::runtime::*;
use snaplingo_core::providers::common::CredentialField;
use snaplingo_core::providers::*;
use snaplingo_core::settings::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
mod cases {
    use crate::{history_repository, hotkey_runtime};

    use std::collections::HashSet;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use super::{
        HotkeyChangeNotifier, HotkeyRegistrar, HotkeyRegistration, HotkeyRuntime,
        HotkeyTriggerTiming,
    };
    use crate::Result;
    use snaplingo_core::hotkeys::configuration::HotkeyConfiguration;
    use snaplingo_core::hotkeys::model::default_hotkey_snapshot;
    use snaplingo_core::hotkeys::model::SCREENSHOT_ACTION;
    use snaplingo_core::hotkeys::model::SCREENSHOT_CATEGORY;
    use snaplingo_core::hotkeys::model::SCREENSHOT_TRANSLATE_ACTION;
    use snaplingo_core::hotkeys::model::SELECTION_TRANSLATE_ACTION;
    use snaplingo_core::hotkeys::model::SHOW_TRANSLATION_WINDOW_ACTION;
    use snaplingo_core::hotkeys::model::TRANSLATION_CATEGORY;
    use snaplingo_storage::Database;
    use snaplingo_storage::SqliteConfigStore;

    struct CountingNotifier(AtomicUsize);

    impl HotkeyChangeNotifier for CountingNotifier {
        fn hotkeys_changed(&self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum Operation {
        Register {
            category: String,
            action: String,
            accelerator: String,
            timing: HotkeyTriggerTiming,
        },
        Unregister(String),
    }

    #[derive(Default)]
    struct FakeHotkeyRegistrar {
        operations: Mutex<Vec<Operation>>,
        fail_register_for: Mutex<Option<String>>,
        fail_unregister_for: Mutex<Option<String>>,
        registered: Mutex<HashSet<String>>,
    }

    impl FakeHotkeyRegistrar {
        fn operations(&self) -> Vec<Operation> {
            self.operations.lock().unwrap().clone()
        }

        fn clear(&self) {
            self.operations.lock().unwrap().clear();
        }

        fn fail_register_for(&self, accelerator: &str) {
            *self.fail_register_for.lock().unwrap() = Some(accelerator.to_string());
        }

        fn fail_unregister_for(&self, accelerator: &str) {
            *self.fail_unregister_for.lock().unwrap() = Some(accelerator.to_string());
        }

        fn is_registered(&self, accelerator: &str) -> bool {
            self.registered.lock().unwrap().contains(accelerator)
        }
    }

    impl HotkeyRegistrar for FakeHotkeyRegistrar {
        fn register(&self, registration: HotkeyRegistration) -> Result<()> {
            self.operations.lock().unwrap().push(Operation::Register {
                category: registration.category.clone(),
                action: registration.action.clone(),
                accelerator: registration.accelerator.clone(),
                timing: registration.timing,
            });

            if self.fail_register_for.lock().unwrap().as_deref()
                == Some(registration.accelerator.as_str())
            {
                return Err(crate::AppError::Other("registration failed".to_string()));
            }

            self.registered
                .lock()
                .unwrap()
                .insert(registration.accelerator);
            Ok(())
        }

        fn unregister(&self, accelerator: &str) -> Result<()> {
            self.operations
                .lock()
                .unwrap()
                .push(Operation::Unregister(accelerator.to_string()));
            if self.fail_unregister_for.lock().unwrap().as_deref() == Some(accelerator) {
                return Err(crate::AppError::Other("unregistration failed".to_string()));
            }
            self.registered.lock().unwrap().remove(accelerator);
            Ok(())
        }
    }

    #[test]
    fn hotkey_runtime_startup_registers_from_configuration_snapshot() {
        let (runtime, configuration, registrar) = runtime_with_configuration();
        configuration
            .update_hotkey(TRANSLATION_CATEGORY, SELECTION_TRANSLATE_ACTION, "⇧⌥D")
            .unwrap();
        runtime.register_startup_hotkeys().unwrap();

        assert!(registrar.operations().contains(&Operation::Register {
            category: TRANSLATION_CATEGORY.to_string(),
            action: SELECTION_TRANSLATE_ACTION.to_string(),
            accelerator: "Shift+Alt+KeyD".to_string(),
            timing: HotkeyTriggerTiming::Released,
        }));
        assert!(!registrar.operations().contains(&Operation::Register {
            category: TRANSLATION_CATEGORY.to_string(),
            action: SELECTION_TRANSLATE_ACTION.to_string(),
            accelerator: "Alt+KeyD".to_string(),
            timing: HotkeyTriggerTiming::Released,
        }));
    }

    #[test]
    fn hotkey_runtime_skips_a_persisted_system_copy_shortcut() {
        let store = Arc::new(SqliteConfigStore::new_temp());
        let mut snapshot = default_hotkey_snapshot();
        snapshot
            .translation
            .insert(SELECTION_TRANSLATE_ACTION.to_string(), "⌘C".to_string());
        store.save("hotkeys", &snapshot).unwrap();
        let configuration = Arc::new(HotkeyConfiguration::new(store));
        let registrar = Arc::new(FakeHotkeyRegistrar::default());
        let runtime = hotkey_runtime(configuration, registrar.clone());

        runtime.register_startup_hotkeys().unwrap();

        assert!(!registrar.operations().iter().any(|operation| {
            matches!(
                operation,
                Operation::Register { accelerator, .. }
                    if accelerator == "CmdOrCtrl+KeyC"
            )
        }));
        assert!(registrar.operations().iter().any(|operation| {
            matches!(
                operation,
                Operation::Register { accelerator, .. }
                    if accelerator == "F1"
            )
        }));
    }

    #[test]
    fn hotkey_runtime_update_to_same_accelerator_is_noop() {
        let (runtime, _configuration, registrar) = runtime_with_configuration();
        runtime.register_startup_hotkeys().unwrap();
        registrar.clear();

        let outcome = runtime
            .update_hotkey(
                TRANSLATION_CATEGORY.to_string(),
                SELECTION_TRANSLATE_ACTION.to_string(),
                "⌥D".to_string(),
            )
            .unwrap();

        assert_eq!(outcome.accelerator, Some("Alt+KeyD".to_string()));
        assert!(registrar.operations().is_empty());
    }

    #[test]
    fn hotkey_runtime_update_registers_new_accelerator_before_unregistering_old_one() {
        let (runtime, _configuration, registrar) = runtime_with_configuration();
        runtime.register_startup_hotkeys().unwrap();
        registrar.clear();

        runtime
            .update_hotkey(
                TRANSLATION_CATEGORY.to_string(),
                SELECTION_TRANSLATE_ACTION.to_string(),
                "⇧⌥D".to_string(),
            )
            .unwrap();

        assert_eq!(
            registrar.operations(),
            vec![
                Operation::Register {
                    category: TRANSLATION_CATEGORY.to_string(),
                    action: SELECTION_TRANSLATE_ACTION.to_string(),
                    accelerator: "Shift+Alt+KeyD".to_string(),
                    timing: HotkeyTriggerTiming::Released,
                },
                Operation::Unregister("Alt+KeyD".to_string()),
            ]
        );
    }

    #[test]
    fn hotkey_runtime_updates_a_combination_to_f1_and_persists_it() {
        let (runtime, configuration, registrar) = runtime_with_configuration();
        runtime.register_startup_hotkeys().unwrap();
        runtime
            .update_hotkey(
                SCREENSHOT_CATEGORY.to_string(),
                SCREENSHOT_ACTION.to_string(),
                "⇧⌘R".to_string(),
            )
            .unwrap();
        registrar.clear();

        let outcome = runtime
            .update_hotkey(
                SCREENSHOT_CATEGORY.to_string(),
                SCREENSHOT_ACTION.to_string(),
                "F1".to_string(),
            )
            .unwrap();

        assert_eq!(outcome.accelerator.as_deref(), Some("F1"));
        assert_eq!(
            configuration.snapshot().unwrap().screenshot[SCREENSHOT_ACTION],
            "F1"
        );
        assert_eq!(
            registrar.operations(),
            vec![
                Operation::Register {
                    category: SCREENSHOT_CATEGORY.to_string(),
                    action: SCREENSHOT_ACTION.to_string(),
                    accelerator: "F1".to_string(),
                    timing: HotkeyTriggerTiming::Pressed,
                },
                Operation::Unregister("Shift+CmdOrCtrl+KeyR".to_string()),
            ]
        );
    }

    #[test]
    fn hotkey_runtime_update_to_unset_unregisters_previous_and_persists_unset() {
        let (runtime, _configuration, registrar) = runtime_with_configuration();
        runtime.register_startup_hotkeys().unwrap();
        registrar.clear();

        let outcome = runtime
            .update_hotkey(
                TRANSLATION_CATEGORY.to_string(),
                SELECTION_TRANSLATE_ACTION.to_string(),
                "未设置".to_string(),
            )
            .unwrap();

        assert_eq!(outcome.accelerator, None);
        assert_eq!(
            registrar.operations(),
            vec![Operation::Unregister("Alt+KeyD".to_string())]
        );
        assert_eq!(
            runtime
                .snapshot()
                .unwrap()
                .translation
                .get(SELECTION_TRANSLATE_ACTION),
            Some(&"未设置".to_string())
        );
    }

    #[test]
    fn hotkey_runtime_reset_uses_domain_default_instead_of_saved_value() {
        let (runtime, _configuration, registrar) = runtime_with_configuration();
        runtime.register_startup_hotkeys().unwrap();
        runtime
            .update_hotkey(
                TRANSLATION_CATEGORY.to_string(),
                SELECTION_TRANSLATE_ACTION.to_string(),
                "⇧⌥D".to_string(),
            )
            .unwrap();
        registrar.clear();

        let outcome = runtime
            .reset_hotkey(
                TRANSLATION_CATEGORY.to_string(),
                SELECTION_TRANSLATE_ACTION.to_string(),
            )
            .unwrap();

        assert_eq!(
            outcome.snapshot.translation.get(SELECTION_TRANSLATE_ACTION),
            Some(&"⌥D".to_string())
        );
        assert_eq!(outcome.accelerator, Some("Alt+KeyD".to_string()));
    }

    #[test]
    fn hotkey_runtime_registration_failure_does_not_persist_new_hotkey() {
        let (runtime, _configuration, registrar) = runtime_with_configuration();
        runtime.register_startup_hotkeys().unwrap();
        registrar.clear();
        registrar.fail_register_for("Shift+Alt+KeyD");

        let result = runtime.update_hotkey(
            TRANSLATION_CATEGORY.to_string(),
            SELECTION_TRANSLATE_ACTION.to_string(),
            "⇧⌥D".to_string(),
        );

        assert!(result.is_err());
        assert!(!registrar
            .operations()
            .contains(&Operation::Unregister("Alt+KeyD".to_string())));
        assert_eq!(
            runtime
                .snapshot()
                .unwrap()
                .translation
                .get(SELECTION_TRANSLATE_ACTION),
            Some(&"⌥D".to_string())
        );
    }

    #[test]
    fn hotkey_runtime_unregistration_failure_keeps_the_previous_hotkey_active() {
        let (runtime, _configuration, registrar) = runtime_with_configuration();
        runtime.register_startup_hotkeys().unwrap();
        registrar.clear();
        registrar.fail_unregister_for("Alt+KeyD");

        let result = runtime.update_hotkey(
            TRANSLATION_CATEGORY.to_string(),
            SELECTION_TRANSLATE_ACTION.to_string(),
            "⇧⌥D".to_string(),
        );

        assert!(result.is_err());
        assert!(registrar.is_registered("Alt+KeyD"));
        assert!(!registrar.is_registered("Shift+Alt+KeyD"));
        assert_eq!(
            runtime
                .snapshot()
                .unwrap()
                .translation
                .get(SELECTION_TRANSLATE_ACTION),
            Some(&"⌥D".to_string())
        );
    }

    #[test]
    fn hotkey_runtime_category_reset_rolls_back_when_a_registration_fails() {
        let (runtime, configuration, registrar) = runtime_with_configuration();
        configuration
            .update_hotkey(TRANSLATION_CATEGORY, SHOW_TRANSLATION_WINDOW_ACTION, "⇧⌥A")
            .unwrap();
        configuration
            .update_hotkey(TRANSLATION_CATEGORY, SCREENSHOT_TRANSLATE_ACTION, "⇧⌥S")
            .unwrap();
        configuration
            .update_hotkey(TRANSLATION_CATEGORY, SELECTION_TRANSLATE_ACTION, "⇧⌥D")
            .unwrap();
        let before = runtime.snapshot().unwrap();
        runtime.register_startup_hotkeys().unwrap();
        registrar.clear();
        registrar.fail_register_for("Alt+KeyS");

        let result = runtime.reset_category(TRANSLATION_CATEGORY.to_string());

        assert!(result.is_err());
        assert_eq!(runtime.snapshot().unwrap(), before);
        assert!(registrar.operations().contains(&Operation::Register {
            category: TRANSLATION_CATEGORY.to_string(),
            action: SHOW_TRANSLATION_WINDOW_ACTION.to_string(),
            accelerator: "Shift+Alt+KeyA".to_string(),
            timing: HotkeyTriggerTiming::Pressed,
        }));
        assert!(registrar.operations().contains(&Operation::Register {
            category: TRANSLATION_CATEGORY.to_string(),
            action: SCREENSHOT_TRANSLATE_ACTION.to_string(),
            accelerator: "Shift+Alt+KeyS".to_string(),
            timing: HotkeyTriggerTiming::Pressed,
        }));
    }

    #[test]
    fn startup_registration_continues_after_one_shortcut_fails() {
        let (runtime, _configuration, registrar) = runtime_with_configuration();
        registrar.fail_register_for("F1");

        runtime.register_startup_hotkeys().unwrap();

        assert!(!registrar.is_registered("F1"));
        assert!(registrar.is_registered("F3"));
        assert!(registrar.operations().contains(&Operation::Register {
            category: SCREENSHOT_CATEGORY.to_string(),
            action: snaplingo_core::hotkeys::model::PIN_ACTION.to_string(),
            accelerator: "F3".to_string(),
            timing: HotkeyTriggerTiming::Pressed,
        }));
    }

    #[test]
    fn hotkey_runtime_category_reset_restores_registrations_when_persistence_fails() {
        let database = Arc::new(Database::in_memory().unwrap());
        let store = Arc::new(SqliteConfigStore::new(database.clone()));
        let configuration = Arc::new(HotkeyConfiguration::new(store));
        configuration
            .update_hotkey(TRANSLATION_CATEGORY, SELECTION_TRANSLATE_ACTION, "⇧⌥D")
            .unwrap();
        let registrar = Arc::new(FakeHotkeyRegistrar::default());
        let runtime = hotkey_runtime(configuration, registrar.clone());
        let before = runtime.snapshot().unwrap();
        runtime.register_startup_hotkeys().unwrap();
        database
            .with_connection(|connection| {
                connection
                    .execute_batch(
                        "CREATE TRIGGER reject_hotkey_updates
                     BEFORE UPDATE ON settings
                     WHEN OLD.namespace = 'hotkeys'
                     BEGIN
                       SELECT RAISE(FAIL, 'forced hotkey persistence failure');
                     END;",
                    )
                    .map_err(snaplingo_core::AppError::database)?;
                Ok(())
            })
            .unwrap();
        registrar.clear();

        let result = runtime.reset_category(TRANSLATION_CATEGORY.to_string());

        assert!(result.is_err());
        assert_eq!(runtime.snapshot().unwrap(), before);
        assert!(registrar
            .operations()
            .contains(&Operation::Unregister("Alt+KeyD".to_string())));
        assert!(registrar.operations().contains(&Operation::Register {
            category: TRANSLATION_CATEGORY.to_string(),
            action: SELECTION_TRANSLATE_ACTION.to_string(),
            accelerator: "Shift+Alt+KeyD".to_string(),
            timing: HotkeyTriggerTiming::Released,
        }));
    }

    #[test]
    fn hotkey_runtime_registers_screenshot_actions_on_press() {
        let (runtime, _configuration, registrar) = runtime_with_configuration();

        runtime.register_startup_hotkeys().unwrap();

        assert!(registrar.operations().contains(&Operation::Register {
            category: SCREENSHOT_CATEGORY.to_string(),
            action: SCREENSHOT_ACTION.to_string(),
            accelerator: "F1".to_string(),
            timing: HotkeyTriggerTiming::Pressed,
        }));
    }

    #[test]
    fn successful_hotkey_update_notifies_runtime_observers() {
        let config_file = Arc::new(SqliteConfigStore::new_temp());
        let configuration = Arc::new(HotkeyConfiguration::new(config_file));
        let notifier = Arc::new(CountingNotifier(AtomicUsize::new(0)));
        let registrar = Arc::new(FakeHotkeyRegistrar::default());
        let runtime =
            HotkeyRuntime::with_change_notifier(configuration, registrar, notifier.clone());
        runtime.register_startup_hotkeys().unwrap();

        runtime
            .update_hotkey(
                TRANSLATION_CATEGORY.to_string(),
                SELECTION_TRANSLATE_ACTION.to_string(),
                "⇧⌥D".to_string(),
            )
            .unwrap();

        assert_eq!(notifier.0.load(Ordering::SeqCst), 1);
    }

    fn runtime_with_configuration() -> (
        HotkeyRuntime,
        Arc<HotkeyConfiguration>,
        Arc<FakeHotkeyRegistrar>,
    ) {
        let config_file = Arc::new(SqliteConfigStore::new_temp());
        let configuration = Arc::new(HotkeyConfiguration::new(config_file));
        let registrar = Arc::new(FakeHotkeyRegistrar::default());
        let runtime = hotkey_runtime(configuration.clone(), registrar.clone());
        (runtime, configuration, registrar)
    }
}

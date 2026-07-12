use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::Serialize;

use crate::application::hotkeys::configuration::HotkeyConfiguration;
use crate::domain::hotkey_config::{
    hotkey_category, validate_hotkey_action, HotkeySettingsSnapshot, DEFAULT_HOTKEYS,
};
use crate::{startup_shortcuts, Result};

pub struct HotkeyRuntime {
    configuration: Arc<HotkeyConfiguration>,
    registrations: Mutex<HashMap<String, String>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HotkeyUpdateOutcome {
    pub snapshot: HotkeySettingsSnapshot,
    pub accelerator: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HotkeyTriggerTiming {
    Pressed,
    Released,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HotkeyRegistration {
    pub category: String,
    pub action: String,
    pub accelerator: String,
    pub timing: HotkeyTriggerTiming,
}

pub(crate) trait HotkeyRegistrar {
    fn register(&self, registration: HotkeyRegistration) -> Result<()>;
    fn unregister(&self, accelerator: &str) -> Result<()>;
}

impl HotkeyRuntime {
    pub fn new(configuration: Arc<HotkeyConfiguration>) -> Self {
        Self {
            configuration,
            registrations: Mutex::new(HashMap::new()),
        }
    }

    pub fn snapshot(&self) -> Result<HotkeySettingsSnapshot> {
        self.configuration.snapshot()
    }

    pub(crate) fn register_startup_hotkeys_with(
        &self,
        registrar: &impl HotkeyRegistrar,
    ) -> Result<()> {
        let snapshot = self.configuration.snapshot()?;

        for default_hotkey in DEFAULT_HOTKEYS {
            let Some(hotkey) = hotkey_category(&snapshot, default_hotkey.category)
                .and_then(|category| category.get(default_hotkey.action))
            else {
                continue;
            };
            let Some(accelerator) =
                resolve_hotkey_accelerator(default_hotkey.category, default_hotkey.action, hotkey)?
            else {
                continue;
            };

            let registration_key =
                hotkey_registration_key(default_hotkey.category, default_hotkey.action);
            let previous_accelerator = self.previous_accelerator(&registration_key)?;
            if previous_accelerator.as_deref() == Some(accelerator.as_str()) {
                continue;
            }

            register_hotkey_action(
                registrar,
                default_hotkey.category,
                default_hotkey.action,
                &accelerator,
            )?;
            self.set_registration(registration_key, Some(accelerator))?;
        }

        Ok(())
    }

    pub(crate) fn update_hotkey_with(
        &self,
        registrar: &impl HotkeyRegistrar,
        category: String,
        action: String,
        hotkey: String,
    ) -> Result<HotkeyUpdateOutcome> {
        let next_accelerator = resolve_hotkey_accelerator(&category, &action, &hotkey)?;
        let registration_key = hotkey_registration_key(&category, &action);
        let previous_accelerator = self.previous_accelerator(&registration_key)?;

        if next_accelerator == previous_accelerator {
            let snapshot = self
                .configuration
                .update_hotkey(&category, &action, &hotkey)?;
            return Ok(HotkeyUpdateOutcome {
                snapshot,
                accelerator: next_accelerator,
            });
        }

        if let Some(accelerator) = &next_accelerator {
            register_hotkey_action(registrar, &category, &action, accelerator)?;
        }

        if let Some(accelerator) = &previous_accelerator {
            if let Err(err) = registrar.unregister(accelerator) {
                log::warn!(
                    "Failed to unregister previous hotkey {} for {}:{}: {}",
                    accelerator,
                    category,
                    action,
                    err
                );
            }
        }

        let snapshot = match self
            .configuration
            .update_hotkey(&category, &action, &hotkey)
        {
            Ok(snapshot) => snapshot,
            Err(err) => {
                rollback_registration_change(
                    registrar,
                    &category,
                    &action,
                    next_accelerator.as_deref(),
                    previous_accelerator.as_deref(),
                );
                return Err(err);
            }
        };

        self.set_registration(registration_key, next_accelerator.clone())?;

        Ok(HotkeyUpdateOutcome {
            snapshot,
            accelerator: next_accelerator,
        })
    }

    fn previous_accelerator(&self, registration_key: &str) -> Result<Option<String>> {
        let registrations = self.registrations.lock().map_err(|err| {
            crate::AppError::Other(format!("Shortcut registry lock poisoned: {err}"))
        })?;
        Ok(registrations.get(registration_key).cloned())
    }

    fn set_registration(
        &self,
        registration_key: String,
        accelerator: Option<String>,
    ) -> Result<()> {
        let mut registrations = self.registrations.lock().map_err(|err| {
            crate::AppError::Other(format!("Shortcut registry lock poisoned: {err}"))
        })?;

        match accelerator {
            Some(accelerator) => {
                registrations.insert(registration_key, accelerator);
            }
            None => {
                registrations.remove(&registration_key);
            }
        }

        Ok(())
    }
}

fn resolve_hotkey_accelerator(
    category: &str,
    action: &str,
    hotkey: &str,
) -> Result<Option<String>> {
    validate_hotkey_action(category, action)?;
    startup_shortcuts::display_hotkey_to_accelerator(hotkey)
}

fn register_hotkey_action(
    registrar: &impl HotkeyRegistrar,
    category: &str,
    action: &str,
    accelerator: &str,
) -> Result<()> {
    registrar.register(HotkeyRegistration {
        category: category.to_string(),
        action: action.to_string(),
        accelerator: accelerator.to_string(),
        timing: hotkey_trigger_timing(category, action),
    })
}

fn rollback_registration_change(
    registrar: &impl HotkeyRegistrar,
    category: &str,
    action: &str,
    next_accelerator: Option<&str>,
    previous_accelerator: Option<&str>,
) {
    if let Some(accelerator) = next_accelerator {
        if let Err(err) = registrar.unregister(accelerator) {
            log::warn!(
                "Failed to roll back hotkey registration {} for {}:{}: {}",
                accelerator,
                category,
                action,
                err
            );
        }
    }

    if let Some(accelerator) = previous_accelerator {
        if let Err(err) = register_hotkey_action(registrar, category, action, accelerator) {
            log::warn!(
                "Failed to restore previous hotkey {} for {}:{}: {}",
                accelerator,
                category,
                action,
                err
            );
        }
    }
}

fn hotkey_trigger_timing(category: &str, action: &str) -> HotkeyTriggerTiming {
    if startup_shortcuts::should_register_hotkey_on_release(category, action) {
        HotkeyTriggerTiming::Released
    } else {
        HotkeyTriggerTiming::Pressed
    }
}

fn hotkey_registration_key(category: &str, action: &str) -> String {
    format!("{category}:{action}")
}

#[cfg(test)]
mod hotkey_runtime_tests {
    use std::sync::{Arc, Mutex};

    use super::{HotkeyRegistrar, HotkeyRegistration, HotkeyRuntime, HotkeyTriggerTiming};
    use crate::application::hotkeys::configuration::HotkeyConfiguration;
    use crate::domain::hotkey_config::{
        SCREENSHOT_ACTION, SCREENSHOT_CATEGORY, SELECTION_TRANSLATE_ACTION, TRANSLATION_CATEGORY,
    };
    use crate::infrastructure::storage::ConfigFile;
    use crate::Result;

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

            Ok(())
        }

        fn unregister(&self, accelerator: &str) -> Result<()> {
            self.operations
                .lock()
                .unwrap()
                .push(Operation::Unregister(accelerator.to_string()));
            Ok(())
        }
    }

    #[test]
    fn hotkey_runtime_startup_registers_from_configuration_snapshot() {
        let (runtime, configuration) = runtime_with_configuration();
        configuration
            .update_hotkey(TRANSLATION_CATEGORY, SELECTION_TRANSLATE_ACTION, "⇧⌥D")
            .unwrap();
        let registrar = FakeHotkeyRegistrar::default();

        runtime.register_startup_hotkeys_with(&registrar).unwrap();

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
    fn hotkey_runtime_update_to_same_accelerator_is_noop() {
        let (runtime, _configuration) = runtime_with_configuration();
        let registrar = FakeHotkeyRegistrar::default();
        runtime.register_startup_hotkeys_with(&registrar).unwrap();
        registrar.clear();

        let outcome = runtime
            .update_hotkey_with(
                &registrar,
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
        let (runtime, _configuration) = runtime_with_configuration();
        let registrar = FakeHotkeyRegistrar::default();
        runtime.register_startup_hotkeys_with(&registrar).unwrap();
        registrar.clear();

        runtime
            .update_hotkey_with(
                &registrar,
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
    fn hotkey_runtime_update_to_unset_unregisters_previous_and_persists_unset() {
        let (runtime, _configuration) = runtime_with_configuration();
        let registrar = FakeHotkeyRegistrar::default();
        runtime.register_startup_hotkeys_with(&registrar).unwrap();
        registrar.clear();

        let outcome = runtime
            .update_hotkey_with(
                &registrar,
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
    fn hotkey_runtime_registration_failure_does_not_persist_new_hotkey() {
        let (runtime, _configuration) = runtime_with_configuration();
        let registrar = FakeHotkeyRegistrar::default();
        runtime.register_startup_hotkeys_with(&registrar).unwrap();
        registrar.clear();
        registrar.fail_register_for("Shift+Alt+KeyD");

        let result = runtime.update_hotkey_with(
            &registrar,
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
    fn hotkey_runtime_registers_release_timing_actions_via_release_path() {
        let (runtime, _configuration) = runtime_with_configuration();
        let registrar = FakeHotkeyRegistrar::default();

        runtime.register_startup_hotkeys_with(&registrar).unwrap();

        assert!(registrar.operations().contains(&Operation::Register {
            category: SCREENSHOT_CATEGORY.to_string(),
            action: SCREENSHOT_ACTION.to_string(),
            accelerator: "Shift+CmdOrCtrl+KeyR".to_string(),
            timing: HotkeyTriggerTiming::Released,
        }));
    }

    fn runtime_with_configuration() -> (HotkeyRuntime, Arc<HotkeyConfiguration>) {
        let config_file = Arc::new(ConfigFile::new_temp());
        let configuration = Arc::new(HotkeyConfiguration::with_legacy_root(config_file, None));
        let runtime = HotkeyRuntime::new(configuration.clone());
        (runtime, configuration)
    }
}

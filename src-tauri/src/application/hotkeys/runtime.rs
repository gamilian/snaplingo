use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::Serialize;

use crate::application::hotkeys::configuration::HotkeyConfiguration;
use crate::application::hotkeys::{
    display_hotkey_to_accelerator, should_register_hotkey_on_release,
};
use crate::domain::hotkey_config::{
    hotkey_category, validate_hotkey_action, HotkeySettingsSnapshot, DEFAULT_HOTKEYS,
};
use crate::Result;

pub struct HotkeyRuntime {
    configuration: Arc<HotkeyConfiguration>,
    change_notifier: Option<Arc<dyn HotkeyChangeNotifier>>,
    operation_lock: Mutex<()>,
    registrations: Mutex<HashMap<String, String>>,
}

pub trait HotkeyChangeNotifier: Send + Sync {
    fn hotkeys_changed(&self);
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

struct HotkeyRegistrationChange {
    category: String,
    action: String,
    registration_key: String,
    previous_accelerator: Option<String>,
    next_accelerator: Option<String>,
}

pub(crate) trait HotkeyRegistrar {
    fn register(&self, registration: HotkeyRegistration) -> Result<()>;
    fn unregister(&self, accelerator: &str) -> Result<()>;
}

impl HotkeyRuntime {
    pub fn new(configuration: Arc<HotkeyConfiguration>) -> Self {
        Self {
            configuration,
            change_notifier: None,
            operation_lock: Mutex::new(()),
            registrations: Mutex::new(HashMap::new()),
        }
    }

    pub fn with_change_notifier(
        configuration: Arc<HotkeyConfiguration>,
        change_notifier: Arc<dyn HotkeyChangeNotifier>,
    ) -> Self {
        Self {
            configuration,
            change_notifier: Some(change_notifier),
            operation_lock: Mutex::new(()),
            registrations: Mutex::new(HashMap::new()),
        }
    }

    fn notify_changed(&self) {
        if let Some(notifier) = &self.change_notifier {
            notifier.hotkeys_changed();
        }
    }

    pub fn snapshot(&self) -> Result<HotkeySettingsSnapshot> {
        self.configuration.snapshot()
    }

    pub fn default_snapshot(&self) -> HotkeySettingsSnapshot {
        crate::domain::hotkey_config::default_hotkey_snapshot()
    }

    pub(crate) fn reset_hotkey_with(
        &self,
        registrar: &impl HotkeyRegistrar,
        category: String,
        action: String,
    ) -> Result<HotkeyUpdateOutcome> {
        validate_hotkey_action(&category, &action)?;
        let hotkey = hotkey_category(&self.default_snapshot(), &category)
            .and_then(|hotkeys| hotkeys.get(&action))
            .cloned()
            .ok_or_else(|| {
                crate::AppError::Other(format!(
                    "Missing default hotkey for '{}:{}'",
                    category, action
                ))
            })?;

        self.update_hotkey_with(registrar, category, action, hotkey)
    }

    pub(crate) fn reset_category_with(
        &self,
        registrar: &impl HotkeyRegistrar,
        category: String,
    ) -> Result<HotkeySettingsSnapshot> {
        let _operation_guard = self.operation_lock.lock().map_err(|err| {
            crate::AppError::Other(format!("Shortcut operation lock poisoned: {err}"))
        })?;
        let defaults = self.default_snapshot();
        let mut entries: Vec<_> = hotkey_category(&defaults, &category)
            .ok_or_else(|| {
                crate::AppError::Other(format!("Unknown hotkey category '{}'", category))
            })?
            .iter()
            .map(|(action, hotkey)| (action.clone(), hotkey.clone()))
            .collect();
        entries.sort_by(|left, right| left.0.cmp(&right.0));

        let mut registrations = self.registrations.lock().map_err(|err| {
            crate::AppError::Other(format!("Shortcut registry lock poisoned: {err}"))
        })?;
        let mut changes = Vec::new();
        for (action, hotkey) in entries {
            let registration_key = hotkey_registration_key(&category, &action);
            let previous_accelerator = registrations.get(&registration_key).cloned();
            let next_accelerator = resolve_hotkey_accelerator(&category, &action, &hotkey)?;
            if previous_accelerator != next_accelerator {
                changes.push(HotkeyRegistrationChange {
                    category: category.clone(),
                    action,
                    registration_key,
                    previous_accelerator,
                    next_accelerator,
                });
            }
        }

        let mut unregistered_previous = Vec::new();
        for (index, change) in changes.iter().enumerate() {
            let Some(accelerator) = change.previous_accelerator.as_deref() else {
                continue;
            };
            if let Err(err) = registrar.unregister(accelerator) {
                restore_previous_registrations(registrar, &changes, &unregistered_previous);
                return Err(err);
            }
            unregistered_previous.push(index);
        }

        let mut registered_next = Vec::new();
        for (index, change) in changes.iter().enumerate() {
            let Some(accelerator) = change.next_accelerator.as_deref() else {
                continue;
            };
            if let Err(err) =
                register_hotkey_action(registrar, &change.category, &change.action, accelerator)
            {
                rollback_category_registration_changes(
                    registrar,
                    &changes,
                    &registered_next,
                    &unregistered_previous,
                );
                return Err(err);
            }
            registered_next.push(index);
        }

        let snapshot = match self.configuration.reset_category(&category) {
            Ok(snapshot) => snapshot,
            Err(err) => {
                rollback_category_registration_changes(
                    registrar,
                    &changes,
                    &registered_next,
                    &unregistered_previous,
                );
                return Err(err);
            }
        };

        for change in changes {
            match change.next_accelerator {
                Some(accelerator) => {
                    registrations.insert(change.registration_key, accelerator);
                }
                None => {
                    registrations.remove(&change.registration_key);
                }
            }
        }
        drop(registrations);
        self.notify_changed();
        Ok(snapshot)
    }

    pub(crate) fn register_startup_hotkeys_with(
        &self,
        registrar: &impl HotkeyRegistrar,
    ) -> Result<()> {
        let _operation_guard = self.operation_lock.lock().map_err(|err| {
            crate::AppError::Other(format!("Shortcut operation lock poisoned: {err}"))
        })?;
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
        let _operation_guard = self.operation_lock.lock().map_err(|err| {
            crate::AppError::Other(format!("Shortcut operation lock poisoned: {err}"))
        })?;
        let next_accelerator = resolve_hotkey_accelerator(&category, &action, &hotkey)?;
        let registration_key = hotkey_registration_key(&category, &action);
        let previous_accelerator = self.previous_accelerator(&registration_key)?;

        if next_accelerator == previous_accelerator {
            let snapshot = self
                .configuration
                .update_hotkey(&category, &action, &hotkey)?;
            self.notify_changed();
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
                if let Some(next_accelerator) = next_accelerator.as_deref() {
                    if let Err(rollback_err) = registrar.unregister(next_accelerator) {
                        log::warn!(
                            "Failed to roll back hotkey registration {} for {}:{} after unregistering {} failed: {}",
                            next_accelerator,
                            category,
                            action,
                            accelerator,
                            rollback_err
                        );
                    }
                }
                return Err(err);
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
        self.notify_changed();

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
    display_hotkey_to_accelerator(hotkey)
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

fn rollback_category_registration_changes(
    registrar: &impl HotkeyRegistrar,
    changes: &[HotkeyRegistrationChange],
    registered_next: &[usize],
    unregistered_previous: &[usize],
) {
    for index in registered_next.iter().rev() {
        let change = &changes[*index];
        let Some(accelerator) = change.next_accelerator.as_deref() else {
            continue;
        };
        if let Err(err) = registrar.unregister(accelerator) {
            log::warn!(
                "Failed to roll back hotkey registration {} for {}:{}: {}",
                accelerator,
                change.category,
                change.action,
                err
            );
        }
    }
    restore_previous_registrations(registrar, changes, unregistered_previous);
}

fn restore_previous_registrations(
    registrar: &impl HotkeyRegistrar,
    changes: &[HotkeyRegistrationChange],
    unregistered_previous: &[usize],
) {
    for index in unregistered_previous.iter().rev() {
        let change = &changes[*index];
        let Some(accelerator) = change.previous_accelerator.as_deref() else {
            continue;
        };
        if let Err(err) =
            register_hotkey_action(registrar, &change.category, &change.action, accelerator)
        {
            log::warn!(
                "Failed to restore previous hotkey {} for {}:{}: {}",
                accelerator,
                change.category,
                change.action,
                err
            );
        }
    }
}

fn hotkey_trigger_timing(category: &str, action: &str) -> HotkeyTriggerTiming {
    if should_register_hotkey_on_release(category, action) {
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
    use std::collections::HashSet;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use super::{
        HotkeyChangeNotifier, HotkeyRegistrar, HotkeyRegistration, HotkeyRuntime,
        HotkeyTriggerTiming,
    };
    use crate::application::hotkeys::configuration::HotkeyConfiguration;
    use crate::domain::hotkey_config::{
        INPUT_TRANSLATE_ACTION, SCREENSHOT_ACTION, SCREENSHOT_CATEGORY,
        SCREENSHOT_TRANSLATE_ACTION, SELECTION_TRANSLATE_ACTION, TRANSLATION_CATEGORY,
    };
    use crate::infrastructure::storage::{Database, SqliteConfigStore};
    use crate::Result;

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
    fn hotkey_runtime_reset_uses_domain_default_instead_of_saved_value() {
        let (runtime, _configuration) = runtime_with_configuration();
        let registrar = FakeHotkeyRegistrar::default();
        runtime.register_startup_hotkeys_with(&registrar).unwrap();
        runtime
            .update_hotkey_with(
                &registrar,
                TRANSLATION_CATEGORY.to_string(),
                SELECTION_TRANSLATE_ACTION.to_string(),
                "⇧⌥D".to_string(),
            )
            .unwrap();
        registrar.clear();

        let outcome = runtime
            .reset_hotkey_with(
                &registrar,
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
    fn hotkey_runtime_unregistration_failure_keeps_the_previous_hotkey_active() {
        let (runtime, _configuration) = runtime_with_configuration();
        let registrar = FakeHotkeyRegistrar::default();
        runtime.register_startup_hotkeys_with(&registrar).unwrap();
        registrar.clear();
        registrar.fail_unregister_for("Alt+KeyD");

        let result = runtime.update_hotkey_with(
            &registrar,
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
        let (runtime, configuration) = runtime_with_configuration();
        configuration
            .update_hotkey(TRANSLATION_CATEGORY, INPUT_TRANSLATE_ACTION, "⇧⌥A")
            .unwrap();
        configuration
            .update_hotkey(TRANSLATION_CATEGORY, SCREENSHOT_TRANSLATE_ACTION, "⇧⌥S")
            .unwrap();
        configuration
            .update_hotkey(TRANSLATION_CATEGORY, SELECTION_TRANSLATE_ACTION, "⇧⌥D")
            .unwrap();
        let before = runtime.snapshot().unwrap();
        let registrar = FakeHotkeyRegistrar::default();
        runtime.register_startup_hotkeys_with(&registrar).unwrap();
        registrar.clear();
        registrar.fail_register_for("Alt+KeyS");

        let result = runtime.reset_category_with(&registrar, TRANSLATION_CATEGORY.to_string());

        assert!(result.is_err());
        assert_eq!(runtime.snapshot().unwrap(), before);
        assert!(registrar.operations().contains(&Operation::Register {
            category: TRANSLATION_CATEGORY.to_string(),
            action: INPUT_TRANSLATE_ACTION.to_string(),
            accelerator: "Shift+Alt+KeyA".to_string(),
            timing: HotkeyTriggerTiming::Pressed,
        }));
        assert!(registrar.operations().contains(&Operation::Register {
            category: TRANSLATION_CATEGORY.to_string(),
            action: SCREENSHOT_TRANSLATE_ACTION.to_string(),
            accelerator: "Shift+Alt+KeyS".to_string(),
            timing: HotkeyTriggerTiming::Released,
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
        let runtime = HotkeyRuntime::new(configuration);
        let before = runtime.snapshot().unwrap();
        let registrar = FakeHotkeyRegistrar::default();
        runtime.register_startup_hotkeys_with(&registrar).unwrap();
        database
            .with_connection(|connection| {
                connection.execute_batch(
                    "CREATE TRIGGER reject_hotkey_updates
                     BEFORE UPDATE ON settings
                     WHEN OLD.namespace = 'hotkeys'
                     BEGIN
                       SELECT RAISE(FAIL, 'forced hotkey persistence failure');
                     END;",
                )?;
                Ok(())
            })
            .unwrap();
        registrar.clear();

        let result = runtime.reset_category_with(&registrar, TRANSLATION_CATEGORY.to_string());

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

    #[test]
    fn successful_hotkey_update_notifies_runtime_observers() {
        let config_file = Arc::new(SqliteConfigStore::new_temp());
        let configuration = Arc::new(HotkeyConfiguration::new(config_file));
        let notifier = Arc::new(CountingNotifier(AtomicUsize::new(0)));
        let runtime = HotkeyRuntime::with_change_notifier(configuration, notifier.clone());
        let registrar = FakeHotkeyRegistrar::default();
        runtime.register_startup_hotkeys_with(&registrar).unwrap();

        runtime
            .update_hotkey_with(
                &registrar,
                TRANSLATION_CATEGORY.to_string(),
                SELECTION_TRANSLATE_ACTION.to_string(),
                "⇧⌥D".to_string(),
            )
            .unwrap();

        assert_eq!(notifier.0.load(Ordering::SeqCst), 1);
    }

    fn runtime_with_configuration() -> (HotkeyRuntime, Arc<HotkeyConfiguration>) {
        let config_file = Arc::new(SqliteConfigStore::new_temp());
        let configuration = Arc::new(HotkeyConfiguration::new(config_file));
        let runtime = HotkeyRuntime::new(configuration.clone());
        (runtime, configuration)
    }
}

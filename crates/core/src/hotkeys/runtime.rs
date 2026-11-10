use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::Serialize;

use crate::hotkeys::configuration::HotkeyConfiguration;
use crate::hotkeys::display_hotkey_to_accelerator;
use crate::hotkeys::model::hotkey_category;
use crate::hotkeys::model::validate_hotkey_action;
use crate::hotkeys::model::HotkeySettingsSnapshot;
use crate::hotkeys::model::DEFAULT_HOTKEYS;
use crate::hotkeys::should_register_hotkey_on_release;
use crate::Result;

pub struct HotkeyRuntime {
    configuration: Arc<HotkeyConfiguration>,
    registrar: Arc<dyn HotkeyRegistrar>,
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
pub enum HotkeyTriggerTiming {
    Pressed,
    Released,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HotkeyRegistration {
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

pub trait HotkeyRegistrar: Send + Sync {
    fn register(&self, registration: HotkeyRegistration) -> Result<()>;
    fn unregister(&self, accelerator: &str) -> Result<()>;
}

impl HotkeyRuntime {
    #[cfg(test)]
    pub fn new(
        configuration: Arc<HotkeyConfiguration>,
        registrar: Arc<dyn HotkeyRegistrar>,
    ) -> Self {
        Self {
            configuration,
            registrar,
            change_notifier: None,
            operation_lock: Mutex::new(()),
            registrations: Mutex::new(HashMap::new()),
        }
    }

    pub fn with_change_notifier(
        configuration: Arc<HotkeyConfiguration>,
        registrar: Arc<dyn HotkeyRegistrar>,
        change_notifier: Arc<dyn HotkeyChangeNotifier>,
    ) -> Self {
        Self {
            configuration,
            registrar,
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
        crate::hotkeys::model::default_hotkey_snapshot()
    }

    pub fn reset_hotkey(&self, category: String, action: String) -> Result<HotkeyUpdateOutcome> {
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

        self.update_hotkey(category, action, hotkey)
    }

    pub fn reset_category(&self, category: String) -> Result<HotkeySettingsSnapshot> {
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
            if let Err(err) = self.registrar.unregister(accelerator) {
                restore_previous_registrations(
                    self.registrar.as_ref(),
                    &changes,
                    &unregistered_previous,
                );
                return Err(err);
            }
            unregistered_previous.push(index);
        }

        let mut registered_next = Vec::new();
        for (index, change) in changes.iter().enumerate() {
            let Some(accelerator) = change.next_accelerator.as_deref() else {
                continue;
            };
            if let Err(err) = register_hotkey_action(
                self.registrar.as_ref(),
                &change.category,
                &change.action,
                accelerator,
            ) {
                rollback_category_registration_changes(
                    self.registrar.as_ref(),
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
                    self.registrar.as_ref(),
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

    pub fn register_startup_hotkeys(&self) -> Result<()> {
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
            let accelerator = match resolve_hotkey_accelerator(
                default_hotkey.category,
                default_hotkey.action,
                hotkey,
            ) {
                Ok(Some(accelerator)) => accelerator,
                Ok(None) => continue,
                Err(error) => {
                    log::warn!(
                        "Skipping invalid global shortcut for {}:{}: {}",
                        default_hotkey.category,
                        default_hotkey.action,
                        error
                    );
                    continue;
                }
            };

            let registration_key =
                hotkey_registration_key(default_hotkey.category, default_hotkey.action);
            let previous_accelerator = self.previous_accelerator(&registration_key)?;
            if previous_accelerator.as_deref() == Some(accelerator.as_str()) {
                continue;
            }

            if let Err(error) = register_hotkey_action(
                self.registrar.as_ref(),
                default_hotkey.category,
                default_hotkey.action,
                &accelerator,
            ) {
                log::warn!(
                    "Skipping unavailable global shortcut for {}:{} ({}): {}",
                    default_hotkey.category,
                    default_hotkey.action,
                    accelerator,
                    error
                );
                continue;
            }
            self.set_registration(registration_key, Some(accelerator))?;
        }

        Ok(())
    }

    pub fn update_hotkey(
        &self,
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
            register_hotkey_action(self.registrar.as_ref(), &category, &action, accelerator)?;
        }

        if let Some(accelerator) = &previous_accelerator {
            if let Err(err) = self.registrar.unregister(accelerator) {
                if let Some(next_accelerator) = next_accelerator.as_deref() {
                    if let Err(rollback_err) = self.registrar.unregister(next_accelerator) {
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
                    self.registrar.as_ref(),
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
    registrar: &dyn HotkeyRegistrar,
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
    registrar: &dyn HotkeyRegistrar,
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
    registrar: &dyn HotkeyRegistrar,
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
    registrar: &dyn HotkeyRegistrar,
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

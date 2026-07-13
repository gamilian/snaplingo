use tauri::State;

#[cfg(test)]
use crate::application::hotkeys::runtime::HotkeyRegistrar;
use crate::application::HotkeyRuntime;
use crate::domain::HotkeySettingsSnapshot;
use crate::infrastructure::system::TauriHotkeyRegistrar;
use crate::HotkeyUpdateOutcome;

#[tauri::command]
pub fn get_hotkey_snapshot(
    state: State<'_, crate::AppState>,
) -> Result<HotkeySettingsSnapshot, String> {
    get_hotkey_snapshot_for_runtime(state.settings.hotkeys.as_ref())
}

#[tauri::command]
pub fn get_default_hotkey_snapshot(state: State<'_, crate::AppState>) -> HotkeySettingsSnapshot {
    state.settings.hotkeys.default_snapshot()
}

#[tauri::command]
pub fn update_hotkey(
    category: String,
    action: String,
    hotkey: String,
    app: tauri::AppHandle,
    state: State<'_, crate::AppState>,
) -> Result<HotkeyUpdateOutcome, String> {
    let outcome = state
        .settings
        .hotkeys
        .update_hotkey_with(
            &TauriHotkeyRegistrar::new(
                app.clone(),
                crate::startup_shortcuts::trigger_hotkey_action,
            ),
            category,
            action,
            hotkey,
        )
        .map_err(|err| err.to_string())?;
    super::state_events::emit_state_changed(&app, super::state_events::HOTKEYS_CHANGED_EVENT);
    Ok(outcome)
}

#[tauri::command]
pub fn reset_hotkey(
    category: String,
    action: String,
    app: tauri::AppHandle,
    state: State<'_, crate::AppState>,
) -> Result<HotkeyUpdateOutcome, String> {
    let outcome = state
        .settings
        .hotkeys
        .reset_hotkey_with(
            &TauriHotkeyRegistrar::new(
                app.clone(),
                crate::startup_shortcuts::trigger_hotkey_action,
            ),
            category,
            action,
        )
        .map_err(|err| err.to_string())?;
    super::state_events::emit_state_changed(&app, super::state_events::HOTKEYS_CHANGED_EVENT);
    Ok(outcome)
}

#[tauri::command]
pub fn reset_hotkey_category(
    category: String,
    app: tauri::AppHandle,
    state: State<'_, crate::AppState>,
) -> Result<HotkeySettingsSnapshot, String> {
    let snapshot = state
        .settings
        .hotkeys
        .reset_category_with(
            &TauriHotkeyRegistrar::new(
                app.clone(),
                crate::startup_shortcuts::trigger_hotkey_action,
            ),
            category,
        )
        .map_err(|err| err.to_string())?;
    super::state_events::emit_state_changed(&app, super::state_events::HOTKEYS_CHANGED_EVENT);
    Ok(snapshot)
}

fn get_hotkey_snapshot_for_runtime(
    runtime: &HotkeyRuntime,
) -> Result<HotkeySettingsSnapshot, String> {
    runtime.snapshot().map_err(|err| err.to_string())
}

#[cfg(test)]
fn update_hotkey_with_registrar_for_runtime(
    runtime: &HotkeyRuntime,
    registrar: &impl HotkeyRegistrar,
    category: String,
    action: String,
    hotkey: String,
) -> Result<HotkeyUpdateOutcome, String> {
    runtime
        .update_hotkey_with(registrar, category, action, hotkey)
        .map_err(|err| err.to_string())
}

#[cfg(test)]
mod hotkey_commands_tests {
    use std::sync::Arc;

    use super::{get_hotkey_snapshot_for_runtime, update_hotkey_with_registrar_for_runtime};
    use crate::application::hotkeys::configuration::HotkeyConfiguration;
    use crate::application::hotkeys::runtime::{
        HotkeyRegistrar, HotkeyRegistration, HotkeyRuntime,
    };
    use crate::domain::hotkey_config::{SELECTION_TRANSLATE_ACTION, TRANSLATION_CATEGORY};
    use crate::infrastructure::storage::SqliteConfigStore;
    use crate::Result;

    #[derive(Default)]
    struct FakeHotkeyRegistrar {
        fail_register: bool,
    }

    impl HotkeyRegistrar for FakeHotkeyRegistrar {
        fn register(&self, _registration: HotkeyRegistration) -> Result<()> {
            if self.fail_register {
                return Err(crate::AppError::Other("registration failed".to_string()));
            }

            Ok(())
        }

        fn unregister(&self, _accelerator: &str) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn get_hotkey_snapshot_delegates_to_runtime() {
        let runtime = test_runtime();

        let snapshot = get_hotkey_snapshot_for_runtime(&runtime).unwrap();

        assert_eq!(
            snapshot.translation.get(SELECTION_TRANSLATE_ACTION),
            Some(&"⌥D".to_string())
        );
    }

    #[test]
    fn update_hotkey_returns_snapshot_and_accelerator() {
        let runtime = test_runtime();
        let registrar = FakeHotkeyRegistrar::default();

        let outcome = update_hotkey_with_registrar_for_runtime(
            &runtime,
            &registrar,
            TRANSLATION_CATEGORY.to_string(),
            SELECTION_TRANSLATE_ACTION.to_string(),
            "⇧⌥D".to_string(),
        )
        .unwrap();

        assert_eq!(outcome.accelerator, Some("Shift+Alt+KeyD".to_string()));
        assert_eq!(
            outcome.snapshot.translation.get(SELECTION_TRANSLATE_ACTION),
            Some(&"⇧⌥D".to_string())
        );
    }

    #[test]
    fn update_hotkey_errors_are_returned_as_strings() {
        let runtime = test_runtime();
        let registrar = FakeHotkeyRegistrar {
            fail_register: true,
        };

        let err = update_hotkey_with_registrar_for_runtime(
            &runtime,
            &registrar,
            TRANSLATION_CATEGORY.to_string(),
            SELECTION_TRANSLATE_ACTION.to_string(),
            "⇧⌥D".to_string(),
        )
        .unwrap_err();

        assert!(err.contains("registration failed"));
    }

    fn test_runtime() -> HotkeyRuntime {
        let config_file = Arc::new(SqliteConfigStore::new_temp());
        let configuration = Arc::new(HotkeyConfiguration::new(config_file));
        HotkeyRuntime::new(configuration)
    }
}

use crate::application::hotkeys::{HotkeyRegistrar, HotkeyRegistration, HotkeyTriggerTiming};
use crate::error::{AppError, Result};
use std::str::FromStr;
use std::sync::Arc;
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShortcutTriggerTiming {
    Pressed,
    Released,
}

pub(crate) struct TauriHotkeyRegistrar {
    app: AppHandle,
    trigger: Arc<dyn Fn(AppHandle, String, String) + Send + Sync>,
}

impl TauriHotkeyRegistrar {
    pub(crate) fn new(
        app: AppHandle,
        trigger: impl Fn(AppHandle, String, String) + Send + Sync + 'static,
    ) -> Self {
        Self {
            app,
            trigger: Arc::new(trigger),
        }
    }
}

impl HotkeyRegistrar for TauriHotkeyRegistrar {
    fn register(&self, registration: HotkeyRegistration) -> Result<()> {
        let category = registration.category.clone();
        let action = registration.action.clone();
        let app = self.app.clone();
        let trigger = self.trigger.clone();
        let handler = move || trigger(app.clone(), category.clone(), action.clone());

        match registration.timing {
            HotkeyTriggerTiming::Pressed => {
                register_shortcut(&self.app, &registration.accelerator, handler)
            }
            HotkeyTriggerTiming::Released => {
                register_shortcut_on_release(&self.app, &registration.accelerator, handler)
            }
        }
    }

    fn unregister(&self, accelerator: &str) -> Result<()> {
        unregister_shortcut(&self.app, accelerator)
    }
}

/// Register a global shortcut
pub fn register_shortcut<F>(app: &AppHandle, accelerator: &str, handler: F) -> Result<()>
where
    F: Fn() + Send + Sync + 'static,
{
    register_shortcut_with_timing(app, accelerator, ShortcutTriggerTiming::Pressed, handler)
}

/// Register a global shortcut that runs after the user releases the key combo.
pub fn register_shortcut_on_release<F>(app: &AppHandle, accelerator: &str, handler: F) -> Result<()>
where
    F: Fn() + Send + Sync + 'static,
{
    register_shortcut_with_timing(app, accelerator, ShortcutTriggerTiming::Released, handler)
}

fn register_shortcut_with_timing<F>(
    app: &AppHandle,
    accelerator: &str,
    timing: ShortcutTriggerTiming,
    handler: F,
) -> Result<()>
where
    F: Fn() + Send + Sync + 'static,
{
    let shortcut = Shortcut::from_str(accelerator)
        .map_err(|e| AppError::Other(format!("Invalid shortcut: {}", e)))?;
    let normalized_accelerator = shortcut.to_string();
    let shortcut_label = normalized_accelerator.clone();

    app.global_shortcut()
        .on_shortcut(shortcut, move |_app, _shortcut, event| {
            log::info!(
                "Global shortcut event: {} ({:?})",
                shortcut_label,
                event.state
            );
            if should_trigger_shortcut(timing, event.state) {
                handler();
            }
        })
        .map_err(|e| AppError::Other(format!("Failed to register shortcut: {}", e)))?;

    log::info!(
        "Registered global shortcut: {} ({})",
        accelerator,
        normalized_accelerator
    );
    Ok(())
}

fn should_trigger_shortcut(timing: ShortcutTriggerTiming, state: ShortcutState) -> bool {
    matches!(
        (timing, state),
        (ShortcutTriggerTiming::Pressed, ShortcutState::Pressed)
            | (ShortcutTriggerTiming::Released, ShortcutState::Released)
    )
}

/// Unregister a global shortcut
pub fn unregister_shortcut(app: &AppHandle, accelerator: &str) -> Result<()> {
    let shortcut = Shortcut::from_str(accelerator)
        .map_err(|e| AppError::Other(format!("Invalid shortcut: {}", e)))?;

    app.global_shortcut()
        .unregister(shortcut)
        .map_err(|e| AppError::Other(format!("Failed to unregister shortcut: {}", e)))?;

    log::info!("Unregistered global shortcut: {}", accelerator);
    Ok(())
}

/// Check if a shortcut is registered
pub fn is_shortcut_registered(app: &AppHandle, accelerator: &str) -> Result<bool> {
    let shortcut = Shortcut::from_str(accelerator)
        .map_err(|e| AppError::Other(format!("Invalid shortcut: {}", e)))?;

    Ok(app.global_shortcut().is_registered(shortcut))
}

#[cfg(test)]
mod tests {
    use super::{should_trigger_shortcut, ShortcutTriggerTiming};
    use tauri_plugin_global_shortcut::ShortcutState;

    #[test]
    fn pressed_timing_only_triggers_on_pressed_events() {
        assert!(should_trigger_shortcut(
            ShortcutTriggerTiming::Pressed,
            ShortcutState::Pressed
        ));
        assert!(!should_trigger_shortcut(
            ShortcutTriggerTiming::Pressed,
            ShortcutState::Released
        ));
    }

    #[test]
    fn released_timing_only_triggers_on_released_events() {
        assert!(should_trigger_shortcut(
            ShortcutTriggerTiming::Released,
            ShortcutState::Released
        ));
        assert!(!should_trigger_shortcut(
            ShortcutTriggerTiming::Released,
            ShortcutState::Pressed
        ));
    }
}

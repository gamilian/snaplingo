use crate::application::hotkeys::{HotkeyRegistrar, HotkeyRegistration, HotkeyTriggerTiming};
use crate::error::{AppError, Result};
use std::str::FromStr;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

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
        .on_shortcut(shortcut, move |app, shortcut, event| {
            log::info!(
                "Global shortcut event: {} ({:?})",
                shortcut_label,
                event.state
            );
            if forward_recorded_hotkey(app, shortcut, event.state) {
                return;
            }
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

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct RecordedHotkey {
    recording_id: String,
    hotkey: String,
}

fn forward_recorded_hotkey(app: &AppHandle, shortcut: &Shortcut, state: ShortcutState) -> bool {
    let Some(recording) = app.try_state::<crate::application::hotkeys::HotkeyRecording>() else {
        return false;
    };
    let Some(session) = recording.session() else {
        return false;
    };
    let window = app.get_webview_window(&session.window_label);
    let Some(window) = window.filter(|window| window.is_focused().unwrap_or(false)) else {
        recording.end(&session.window_label, &session.id);
        return false;
    };

    // Registered keys never reach DOM keydown. During recording, consume both
    // edges and deliver the released chord to the focused settings window.
    if state == ShortcutState::Released {
        let payload = RecordedHotkey {
            recording_id: session.id,
            hotkey: shortcut_display_hotkey(shortcut),
        };
        if let Err(error) = window.emit_to(window.label(), "hotkey-recorded", payload) {
            log::warn!("Failed to deliver recorded hotkey: {error}");
        }
    }
    true
}

fn shortcut_display_hotkey(shortcut: &Shortcut) -> String {
    let mut display = String::new();
    if shortcut.mods.contains(Modifiers::SHIFT) {
        display.push('⇧');
    }
    if shortcut.mods.contains(Modifiers::ALT) {
        display.push('⌥');
    }
    if shortcut.mods.intersects(Modifiers::SUPER | Modifiers::META) {
        display.push('⌘');
    }
    if shortcut.mods.contains(Modifiers::CONTROL) {
        display.push(if cfg!(target_os = "windows") {
            '⌘'
        } else {
            '⌃'
        });
    }
    let key = shortcut.key.to_string();
    display.push_str(
        key.strip_prefix("Key")
            .or_else(|| key.strip_prefix("Digit"))
            .unwrap_or(&key),
    );
    display
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
    use super::{shortcut_display_hotkey, should_trigger_shortcut, ShortcutTriggerTiming};
    use tauri_plugin_global_shortcut::{Shortcut, ShortcutState};

    #[test]
    fn recorded_native_shortcuts_use_the_same_display_format_as_the_recorder() {
        for display in ["F1", "F2", "F3", "F20", "⌘F1", "⇧⌘R", "⇧⌥S", "⌘2"] {
            let accelerator = crate::application::hotkeys::display_hotkey_to_accelerator(display)
                .unwrap()
                .unwrap();
            let shortcut = accelerator.parse::<Shortcut>().unwrap();
            assert_eq!(shortcut_display_hotkey(&shortcut), display);
        }
    }

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

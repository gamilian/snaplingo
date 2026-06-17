use std::collections::HashMap;
use std::sync::{Arc, Mutex, Once, OnceLock};

use global_hotkey::{hotkey::HotKey, GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use tauri::AppHandle;

use crate::error::{AppError, Result};

type ShortcutCallback = Arc<dyn Fn() + Send + Sync + 'static>;

static MANAGER: OnceLock<GlobalHotKeyManager> = OnceLock::new();
static REGISTRATIONS: OnceLock<Mutex<HashMap<String, HotKey>>> = OnceLock::new();
static CALLBACKS: OnceLock<Mutex<HashMap<u32, ShortcutCallback>>> = OnceLock::new();
static EVENT_HANDLER: Once = Once::new();

pub fn register_shortcut<F>(_app: &AppHandle, accelerator: &str, _handler: F) -> Result<()>
where
    F: Fn() + Send + Sync + 'static,
{
    ensure_event_handler();

    let hotkey = parse_shortcut(accelerator)?;
    let id = hotkey.id();
    let callback: ShortcutCallback = Arc::new(_handler);

    {
        let registrations = registrations();
        let mut registrations = registrations
            .lock()
            .map_err(|_| AppError::System("Shortcut registry lock poisoned".to_string()))?;
        if registrations.contains_key(accelerator) {
            callbacks()
                .lock()
                .map_err(|_| AppError::System("Shortcut callback lock poisoned".to_string()))?
                .insert(id, callback);
            return Ok(());
        }

        manager()?
            .register(hotkey)
            .map_err(|e| AppError::Other(format!("Failed to register shortcut: {}", e)))?;
        registrations.insert(accelerator.to_string(), hotkey);
    }

    callbacks()
        .lock()
        .map_err(|_| AppError::System("Shortcut callback lock poisoned".to_string()))?
        .insert(id, callback);

    log::info!("Shortcut registered: {} -> {}", accelerator, id);
    Ok(())
}

pub fn unregister_shortcut(_app: &AppHandle, accelerator: &str) -> Result<()> {
    let hotkey = registrations()
        .lock()
        .map_err(|_| AppError::System("Shortcut registry lock poisoned".to_string()))?
        .remove(accelerator);

    if let Some(hotkey) = hotkey {
        callbacks()
            .lock()
            .map_err(|_| AppError::System("Shortcut callback lock poisoned".to_string()))?
            .remove(&hotkey.id());
        manager()?
            .unregister(hotkey)
            .map_err(|e| AppError::Other(format!("Failed to unregister shortcut: {}", e)))?;
    }

    Ok(())
}

pub fn is_shortcut_registered(_app: &AppHandle, accelerator: &str) -> Result<bool> {
    Ok(registrations()
        .lock()
        .map_err(|_| AppError::System("Shortcut registry lock poisoned".to_string()))?
        .contains_key(accelerator))
}

fn manager() -> Result<&'static GlobalHotKeyManager> {
    if let Some(manager) = MANAGER.get() {
        return Ok(manager);
    }

    let manager = GlobalHotKeyManager::new()
        .map_err(|e| AppError::Other(format!("Failed to create shortcut manager: {}", e)))?;
    Ok(MANAGER.get_or_init(|| manager))
}

fn registrations() -> &'static Mutex<HashMap<String, HotKey>> {
    REGISTRATIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn callbacks() -> &'static Mutex<HashMap<u32, ShortcutCallback>> {
    CALLBACKS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn ensure_event_handler() {
    EVENT_HANDLER.call_once(|| {
        GlobalHotKeyEvent::set_event_handler(Some(|event: GlobalHotKeyEvent| {
            if event.state() != HotKeyState::Pressed {
                return;
            }

            let callback = callbacks()
                .lock()
                .ok()
                .and_then(|callbacks| callbacks.get(&event.id()).cloned());
            if let Some(callback) = callback {
                callback();
            }
        }));
    });
}

fn parse_shortcut(accelerator: &str) -> Result<HotKey> {
    accelerator
        .parse::<HotKey>()
        .map_err(|e| AppError::Other(format!("Invalid shortcut '{}': {}", accelerator, e)))
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses_existing_screenshot_shortcut_accelerator() {
        let hotkey = super::parse_shortcut("Cmd+Shift+R").unwrap();

        assert!(hotkey.id() > 0);
    }

    #[test]
    fn parses_pin_toggle_shortcut_accelerator() {
        let hotkey = super::parse_shortcut("Shift+F3").unwrap();

        assert!(hotkey.id() > 0);
    }

    #[test]
    fn parses_pin_group_switch_shortcut_accelerator() {
        let hotkey = super::parse_shortcut("Cmd+F3").unwrap();

        assert!(hotkey.id() > 0);
    }

    #[test]
    fn rejects_unknown_shortcut_key() {
        assert!(super::parse_shortcut("Cmd+Shift+DefinitelyNotAKey").is_err());
    }
}

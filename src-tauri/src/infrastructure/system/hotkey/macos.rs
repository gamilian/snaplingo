use crate::error::AppError;
use super::backend::{HotkeyBackend, HotkeyId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use global_hotkey::{GlobalHotKeyManager, hotkey::{HotKey, Code, Modifiers}};

/// macOS hotkey backend using Carbon Event Manager via global-hotkey crate
pub struct MacOSHotkeyBackend {
    manager: Arc<GlobalHotKeyManager>,
    // Map from HotKey.id() to (accelerator, HotKey)
    registrations: Mutex<HashMap<u32, (String, HotKey)>>,
}

impl MacOSHotkeyBackend {
    pub fn new() -> Result<Self, AppError> {
        let manager = GlobalHotKeyManager::new()
            .map_err(|e| AppError::Other(format!("Failed to create hotkey manager: {}", e)))?;

        Ok(Self {
            manager: Arc::new(manager),
            registrations: Mutex::new(HashMap::new()),
        })
    }

    /// Parse accelerator string like "Cmd+Shift+S" into modifiers and key code
    fn parse_accelerator(accelerator: &str) -> Result<(Modifiers, Code), AppError> {
        let parts: Vec<&str> = accelerator.split('+').map(|s| s.trim()).collect();

        if parts.is_empty() {
            return Err(AppError::Other("Empty accelerator string".to_string()));
        }

        let mut modifiers = Modifiers::empty();
        let key_part = parts.last().unwrap();

        // Parse modifiers
        for part in &parts[..parts.len() - 1] {
            match part.to_lowercase().as_str() {
                "cmd" | "command" | "super" => modifiers |= Modifiers::SUPER,
                "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
                "alt" | "option" => modifiers |= Modifiers::ALT,
                "shift" => modifiers |= Modifiers::SHIFT,
                _ => return Err(AppError::Other(format!("Unknown modifier: {}", part))),
            }
        }

        // Parse key code
        let code = Self::parse_key_code(key_part)?;

        Ok((modifiers, code))
    }

    /// Parse key string to Code enum
    fn parse_key_code(key: &str) -> Result<Code, AppError> {
        let code = match key.to_lowercase().as_str() {
            // Letters
            "a" => Code::KeyA,
            "b" => Code::KeyB,
            "c" => Code::KeyC,
            "d" => Code::KeyD,
            "e" => Code::KeyE,
            "f" => Code::KeyF,
            "g" => Code::KeyG,
            "h" => Code::KeyH,
            "i" => Code::KeyI,
            "j" => Code::KeyJ,
            "k" => Code::KeyK,
            "l" => Code::KeyL,
            "m" => Code::KeyM,
            "n" => Code::KeyN,
            "o" => Code::KeyO,
            "p" => Code::KeyP,
            "q" => Code::KeyQ,
            "r" => Code::KeyR,
            "s" => Code::KeyS,
            "t" => Code::KeyT,
            "u" => Code::KeyU,
            "v" => Code::KeyV,
            "w" => Code::KeyW,
            "x" => Code::KeyX,
            "y" => Code::KeyY,
            "z" => Code::KeyZ,

            // Numbers
            "0" => Code::Digit0,
            "1" => Code::Digit1,
            "2" => Code::Digit2,
            "3" => Code::Digit3,
            "4" => Code::Digit4,
            "5" => Code::Digit5,
            "6" => Code::Digit6,
            "7" => Code::Digit7,
            "8" => Code::Digit8,
            "9" => Code::Digit9,

            // Function keys
            "f1" => Code::F1,
            "f2" => Code::F2,
            "f3" => Code::F3,
            "f4" => Code::F4,
            "f5" => Code::F5,
            "f6" => Code::F6,
            "f7" => Code::F7,
            "f8" => Code::F8,
            "f9" => Code::F9,
            "f10" => Code::F10,
            "f11" => Code::F11,
            "f12" => Code::F12,

            // Special keys
            "space" => Code::Space,
            "enter" | "return" => Code::Enter,
            "tab" => Code::Tab,
            "escape" | "esc" => Code::Escape,
            "backspace" => Code::Backspace,
            "delete" => Code::Delete,
            "insert" => Code::Insert,
            "home" => Code::Home,
            "end" => Code::End,
            "pageup" => Code::PageUp,
            "pagedown" => Code::PageDown,
            "arrowup" | "up" => Code::ArrowUp,
            "arrowdown" | "down" => Code::ArrowDown,
            "arrowleft" | "left" => Code::ArrowLeft,
            "arrowright" | "right" => Code::ArrowRight,

            _ => return Err(AppError::Other(format!("Unknown key: {}", key))),
        };

        Ok(code)
    }
}

#[async_trait::async_trait]
impl HotkeyBackend for MacOSHotkeyBackend {
    async fn register(&self, accelerator: &str) -> Result<HotkeyId, AppError> {
        // Parse accelerator
        let (modifiers, code) = Self::parse_accelerator(accelerator)?;

        // Create hotkey (global-hotkey assigns unique ID internally)
        let hotkey = HotKey::new(Some(modifiers), code);

        // Get the internal ID that global-hotkey assigned
        let internal_id = hotkey.id();

        // Register with Carbon Event Manager via global-hotkey
        self.manager.register(hotkey)
            .map_err(|e| AppError::Other(format!("Failed to register hotkey '{}': {}", accelerator, e)))?;

        // Store registration using the HotKey's internal ID
        let mut registrations = self.registrations.lock().unwrap();
        registrations.insert(internal_id, (accelerator.to_string(), hotkey));

        log::info!("Hotkey registered: {} -> ID {}", accelerator, internal_id);

        Ok(HotkeyId(internal_id))
    }

    async fn unregister(&self, id: HotkeyId) -> Result<(), AppError> {
        let mut registrations = self.registrations.lock().unwrap();

        if let Some((accelerator, hotkey)) = registrations.remove(&id.0) {
            // Unregister from Carbon Event Manager
            self.manager.unregister(hotkey)
                .map_err(|e| AppError::Other(format!("Failed to unregister hotkey: {}", e)))?;

            log::info!("Hotkey unregistered: {} (ID: {:?})", accelerator, id);
            Ok(())
        } else {
            Err(AppError::Other(format!("Hotkey ID {:?} not found", id)))
        }
    }

    async fn is_registered(&self, id: HotkeyId) -> bool {
        let registrations = self.registrations.lock().unwrap();
        registrations.contains_key(&id.0)
    }
}

// TODO: Event callback integration
// The global-hotkey crate provides a GlobalHotKeyEvent channel that emits events
// when hotkeys are triggered. To integrate with Tauri:
//
// 1. Use GlobalHotKeyEvent::receiver() to get event channel
// 2. Spawn a tokio task that polls the channel
// 3. When an event arrives, emit a Tauri event or invoke a callback
// 4. This requires integration with the Tauri event loop, typically done in
//    the Tauri setup hook in main.rs or lib.rs
//
// Example integration pattern:
//   let receiver = GlobalHotKeyEvent::receiver();
//   tauri::async_runtime::spawn(async move {
//       loop {
//           if let Ok(event) = receiver.try_recv() {
//               // Emit Tauri event or invoke callback
//               app.emit_all("hotkey-triggered", event.id()).ok();
//           }
//           tokio::time::sleep(Duration::from_millis(100)).await;
//       }
//   });


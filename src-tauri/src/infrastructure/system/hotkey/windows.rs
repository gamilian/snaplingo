use crate::error::AppError;
use super::backend::{HotkeyBackend, HotkeyId};
use std::collections::HashMap;
use std::sync::Mutex;

/// Windows hotkey backend (placeholder implementation)
pub struct WindowsHotkeyBackend {
    registrations: Mutex<HashMap<u32, String>>,
    next_id: Mutex<u32>,
}

impl WindowsHotkeyBackend {
    pub fn new() -> Self {
        Self {
            registrations: Mutex::new(HashMap::new()),
            next_id: Mutex::new(1),
        }
    }
}

#[async_trait::async_trait]
impl HotkeyBackend for WindowsHotkeyBackend {
    async fn register(&self, accelerator: &str) -> Result<HotkeyId, AppError> {
        let mut next_id = self.next_id.lock().unwrap();
        let id = *next_id;
        *next_id += 1;

        let mut registrations = self.registrations.lock().unwrap();
        registrations.insert(id, accelerator.to_string());

        // TODO: Actual Windows hotkey registration in Phase 4
        // Will use RegisterHotKey Win32 API
        log::debug!("Hotkey registered (placeholder): {} -> {:?}", accelerator, id);

        Ok(HotkeyId(id))
    }

    async fn unregister(&self, id: HotkeyId) -> Result<(), AppError> {
        let mut registrations = self.registrations.lock().unwrap();

        if registrations.remove(&id.0).is_some() {
            // TODO: Actual Windows hotkey unregistration in Phase 4
            // Will use UnregisterHotKey Win32 API
            log::debug!("Hotkey unregistered (placeholder): {:?}", id);
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

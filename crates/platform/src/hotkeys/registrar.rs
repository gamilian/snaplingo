use super::registry::{register, unregister, ShortcutTriggerTiming};
use crate::Result;
use std::sync::Arc;

pub struct NativeHotkeyRegistrar {
    trigger: Arc<dyn Fn(String, String) + Send + Sync>,
}
impl NativeHotkeyRegistrar {
    pub fn new(trigger: impl Fn(String, String) + Send + Sync + 'static) -> Self {
        Self {
            trigger: Arc::new(trigger),
        }
    }
}
impl snaplingo_core::hotkeys::HotkeyRegistrar for NativeHotkeyRegistrar {
    fn register(&self, registration: snaplingo_core::hotkeys::HotkeyRegistration) -> Result<()> {
        use snaplingo_core::hotkeys::HotkeyTriggerTiming;
        let trigger = self.trigger.clone();
        let timing = match registration.timing {
            HotkeyTriggerTiming::Pressed => ShortcutTriggerTiming::Pressed,
            HotkeyTriggerTiming::Released => ShortcutTriggerTiming::Released,
        };
        register(&registration.accelerator, timing, move || {
            trigger(registration.category.clone(), registration.action.clone())
        })
    }
    fn unregister(&self, accelerator: &str) -> Result<()> {
        unregister(accelerator)
    }
}

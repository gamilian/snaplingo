use super::shortcut_native::{register, unregister, ShortcutTriggerTiming};
use crate::Result;
use std::sync::Arc;

pub(crate) struct NativeHotkeyRegistrar {
    trigger: Arc<dyn Fn(String, String) + Send + Sync>,
}
impl NativeHotkeyRegistrar {
    pub(crate) fn new(trigger: impl Fn(String, String) + Send + Sync + 'static) -> Self {
        Self {
            trigger: Arc::new(trigger),
        }
    }
}
impl crate::application::hotkeys::HotkeyRegistrar for NativeHotkeyRegistrar {
    fn register(
        &self,
        registration: crate::application::hotkeys::HotkeyRegistration,
    ) -> Result<()> {
        use crate::application::hotkeys::HotkeyTriggerTiming;
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

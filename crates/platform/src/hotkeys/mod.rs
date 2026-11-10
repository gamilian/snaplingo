mod recording;
mod registrar;
mod registry;

pub use recording::RecordedHotkey;
pub use registrar::NativeHotkeyRegistrar;
pub use registry::{initialize, is_registered, register, shutdown, unregister};

use snaplingo_core::error::Result;

pub use registry::{ShortcutTriggerTiming, UiTask};

/// Initialize on the platform event-loop thread with the host's dispatcher and
/// recording destination. No desktop shell owns the native shortcut registry.
pub fn initialize_native_shortcuts(
    recording: snaplingo_core::hotkeys::HotkeyRecording,
    dispatch: impl Fn(UiTask) -> Result<()> + Send + Sync + 'static,
    is_focused: impl Fn(&str) -> bool + Send + Sync + 'static,
    deliver: impl Fn(&str, RecordedHotkey) + Send + Sync + 'static,
) -> Result<()> {
    registry::initialize(dispatch, move |shortcut, state| {
        recording::forward_recorded_hotkey(&recording, shortcut, state, &is_focused, &deliver)
    })
}

pub fn shutdown_shortcuts() -> Result<()> {
    registry::shutdown()
}

/// Event injection for native host acceptance tests. The production registry,
/// callbacks and UI dispatcher are exercised without requiring physical keys.
#[cfg(feature = "test-support")]
pub mod test_support {
    pub fn dispatch_event(event: global_hotkey::GlobalHotKeyEvent) {
        super::registry::dispatch_event(event);
    }
}

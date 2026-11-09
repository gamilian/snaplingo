use snaplingo_core::error::Result;

pub use super::shortcut_native::{ShortcutTriggerTiming, UiTask};

#[cfg(target_os = "macos")]
pub use super::hotkey_recording::initialize_shortcuts;

/// Initialize on the platform event-loop thread with the host's dispatcher and
/// recording destination. No desktop shell owns the native shortcut registry.
pub fn initialize_native_shortcuts(
    recording: snaplingo_core::application::hotkeys::HotkeyRecording,
    dispatch: impl Fn(UiTask) -> Result<()> + Send + Sync + 'static,
    is_focused: impl Fn(&str) -> bool + Send + Sync + 'static,
    deliver: impl Fn(&str, super::RecordedHotkey) + Send + Sync + 'static,
) -> Result<()> {
    super::shortcut_native::initialize(dispatch, move |shortcut, state| {
        super::hotkey_recording::forward_recorded_hotkey(
            &recording,
            shortcut,
            state,
            &is_focused,
            &deliver,
        )
    })
}

pub fn shutdown_shortcuts() -> Result<()> {
    super::shortcut_native::shutdown()
}

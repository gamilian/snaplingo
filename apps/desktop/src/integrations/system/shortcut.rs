//! Bind global shortcut recording to the AppKit window host.
use snaplingo_core::hotkeys::HotkeyRecording;
use snaplingo_platform::hotkeys::{initialize_native_shortcuts, RecordedHotkey};

/// Initialize on AppKit's main thread; the system registry receives the host's
/// dispatcher and focus query without retaining application state.
pub fn initialize_shortcuts(
    recording: HotkeyRecording,
    deliver: impl Fn(&str, RecordedHotkey) + Send + Sync + 'static,
) -> crate::Result<()> {
    initialize_native_shortcuts(
        recording,
        |task| {
            crate::integrations::ui_thread::dispatch(task);
            Ok(())
        },
        native_window_is_focused,
        deliver,
    )
}

fn native_window_is_focused(label: &str) -> bool {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSUserInterfaceItemIdentification};

    let Some(mtm) = MainThreadMarker::new() else {
        return false;
    };
    let app = NSApplication::sharedApplication(mtm);
    app.isActive()
        && app.keyWindow().is_some_and(|window| {
            window.isVisible()
                && window
                    .identifier()
                    .is_some_and(|id| id.to_string() == label)
        })
}

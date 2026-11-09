use global_hotkey::{
    hotkey::{HotKey as Shortcut, Modifiers},
    HotKeyState as ShortcutState,
};

use snaplingo_core::application::hotkeys::HotkeyRecording;

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordedHotkey {
    pub recording_id: String,
    pub hotkey: String,
}

/// Initialize on AppKit's main thread without retaining a desktop shell handle.
#[cfg(target_os = "macos")]
pub fn initialize_shortcuts(
    recording: snaplingo_core::application::hotkeys::HotkeyRecording,
    deliver: impl Fn(&str, super::RecordedHotkey) + Send + Sync + 'static,
) -> crate::Result<()> {
    super::shortcut::initialize_native_shortcuts(
        recording,
        |task| {
            crate::infrastructure::ui_thread::dispatch(task);
            Ok(())
        },
        native_window_is_focused,
        deliver,
    )
}

pub(crate) fn forward_recorded_hotkey(
    recording: &HotkeyRecording,
    shortcut: &Shortcut,
    state: ShortcutState,
    is_focused: impl FnOnce(&str) -> bool,
    deliver: impl FnOnce(&str, RecordedHotkey),
) -> bool {
    let Some(session) = recording.session() else {
        return false;
    };
    if !is_focused(&session.window_label) {
        recording.end(&session.window_label, &session.id);
        return false;
    }

    // Registered keys never reach a local keydown handler. Consume both edges
    // while recording, and deliver the completed chord on release.
    if state == ShortcutState::Released {
        deliver(
            &session.window_label,
            RecordedHotkey {
                recording_id: session.id,
                hotkey: shortcut_display_hotkey(shortcut),
            },
        );
    }
    true
}

#[cfg(target_os = "macos")]
pub(crate) fn native_window_is_focused(label: &str) -> bool {
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

fn shortcut_display_hotkey(shortcut: &Shortcut) -> String {
    let mut display = String::new();
    if shortcut.mods.contains(Modifiers::SHIFT) {
        display.push('⇧');
    }
    if shortcut.mods.contains(Modifiers::ALT) {
        display.push('⌥');
    }
    if shortcut.mods.intersects(Modifiers::SUPER | Modifiers::META) {
        display.push(if cfg!(target_os = "windows") {
            '⊞'
        } else {
            '⌘'
        });
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consumes_both_edges_delivers_on_release_and_restores_routing_on_focus_loss() {
        let recording = HotkeyRecording::default();
        let controller = recording.clone();
        let shortcut = "Super+F20".parse::<Shortcut>().unwrap();
        let mut received = Vec::new();
        controller.begin("settings", "first".into());
        for state in [ShortcutState::Pressed, ShortcutState::Released] {
            assert!(forward_recorded_hotkey(
                &recording,
                &shortcut,
                state,
                |label| label == "settings",
                |label, payload| received.push((
                    label.to_string(),
                    payload.recording_id,
                    payload.hotkey
                )),
            ));
        }
        let modifier = if cfg!(target_os = "linux") {
            '⌃'
        } else {
            '⌘'
        };
        assert_eq!(
            received,
            vec![("settings".into(), "first".into(), format!("{modifier}F20"))]
        );
        assert!(!forward_recorded_hotkey(
            &recording,
            &shortcut,
            ShortcutState::Pressed,
            |_| false,
            |_, _| panic!("Unfocused recording was delivered"),
        ));
        assert!(controller.session().is_none());
        assert!(!forward_recorded_hotkey(
            &recording,
            &shortcut,
            ShortcutState::Released,
            |_| true,
            |_, _| panic!("Canceled recording was delivered"),
        ));
    }

    #[test]
    fn recorded_native_shortcuts_use_the_same_display_format_as_the_recorder() {
        let primary_modifier = if cfg!(target_os = "linux") {
            "⌃"
        } else {
            "⌘"
        };
        for display in [
            "F1".to_string(),
            "F2".to_string(),
            "F3".to_string(),
            "F20".to_string(),
            format!("{primary_modifier}F1"),
            format!("⇧{primary_modifier}R"),
            "⇧⌥S".to_string(),
            format!("{primary_modifier}2"),
        ] {
            let accelerator =
                snaplingo_core::application::hotkeys::display_hotkey_to_accelerator(&display)
                    .unwrap()
                    .unwrap();
            let shortcut = accelerator.parse::<Shortcut>().unwrap();
            assert_eq!(shortcut_display_hotkey(&shortcut), display);
        }
    }
}

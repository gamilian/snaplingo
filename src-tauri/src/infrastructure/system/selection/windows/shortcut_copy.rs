use std::time::Duration;

use async_trait::async_trait;

use crate::application::selected_text::SelectionMethod;
use crate::domain::{
    MethodAvailability, SelectionAttempt, SelectionContext, SelectionMethodKind, SelectionSource,
};
use crate::infrastructure::system::selection::common::clipboard_transaction;
use crate::infrastructure::system::selection::common::shortcut_copy::wait_for_shortcut_modifiers_to_clear_with;

const CLIPBOARD_TIMEOUT: Duration = Duration::from_millis(700);
const CLIPBOARD_POLL_INTERVAL: Duration = Duration::from_millis(15);
const MODIFIER_RELEASE_TIMEOUT: Duration = Duration::from_millis(1500);
const MODIFIER_POLL_INTERVAL: Duration = Duration::from_millis(15);
const EMPTY_SELECTION_ERROR: &str = "Selected text is empty";

pub struct ShortcutCopySelectionMethod;

#[async_trait]
impl SelectionMethod for ShortcutCopySelectionMethod {
    fn kind(&self) -> SelectionMethodKind {
        SelectionMethodKind::ShortcutCopy
    }

    fn availability(&self, _context: &SelectionContext) -> MethodAvailability {
        MethodAvailability::Available
    }

    async fn acquire(&self, context: &SelectionContext) -> SelectionAttempt {
        match acquire_shortcut_copy_text().await {
            Ok(text) => SelectionAttempt::success(
                self.kind(),
                SelectionSource::ShortcutCopy,
                text,
                context.clone(),
            ),
            Err(err) if err == EMPTY_SELECTION_ERROR => {
                SelectionAttempt::empty(self.kind(), context.clone())
            }
            Err(err) => SelectionAttempt::failed(self.kind(), context.clone(), err),
        }
    }
}

async fn acquire_shortcut_copy_text() -> Result<String, String> {
    wait_for_platform_shortcut_modifiers_to_release().await?;
    let original = super::clipboard::snapshot().map_err(|error| {
        log::warn!("Could not preserve Windows clipboard: {error}");
        error
    })?;
    let original_change_count = original.change_count();
    let before_copy_check = std::sync::Mutex::new(true);
    let observed_change = std::sync::Mutex::new(None);
    let result = clipboard_transaction::wait_for_clipboard_text_change_after_action_observing(
        || async { press_copy_shortcut() },
        || {
            let change_count = clipboard_change_count()?;
            let mut is_before_copy = before_copy_check.lock().unwrap();
            if *is_before_copy {
                *is_before_copy = false;
                if change_count != original_change_count {
                    return Err("Clipboard changed before selected text could be copied".into());
                }
            }
            Ok(change_count)
        },
        read_clipboard_text,
        CLIPBOARD_TIMEOUT,
        CLIPBOARD_POLL_INTERVAL,
        |before, change| *observed_change.lock().unwrap() = Some((before, change)),
    )
    .await;

    let clipboard_change = result.as_ref().ok().cloned().or_else(|| {
        observed_change
            .lock()
            .unwrap()
            .to_owned()
            .map(
                |(before_change_count, change_count)| clipboard_transaction::ClipboardTextChange {
                    text: String::new(),
                    before_change_count,
                    change_count,
                },
            )
    });
    if let Some(change) = clipboard_change {
        if let Err(error) = super::clipboard::restore(&original, &change) {
            log::warn!("Could not restore Windows clipboard: {error}");
            return Err(format!("Failed to restore the Windows clipboard: {error}"));
        }
    }

    result.map(|change| change.text)
}

#[cfg(target_os = "windows")]
async fn wait_for_platform_shortcut_modifiers_to_release() -> Result<(), String> {
    let waited = wait_for_shortcut_modifiers_to_clear_with(
        shortcut_modifier_keys_pressed,
        MODIFIER_RELEASE_TIMEOUT,
        MODIFIER_POLL_INTERVAL,
    )
    .await?;

    if waited >= Duration::from_millis(50) {
        log::info!(
            "Selection copy waited {}ms for shortcut modifiers to clear",
            waited.as_millis()
        );
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn shortcut_modifier_keys_pressed() -> Result<bool, String> {
    use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;

    Ok(any_windows_shortcut_modifier_pressed(|key| unsafe {
        GetAsyncKeyState(key) < 0
    }))
}

#[cfg(not(target_os = "windows"))]
async fn wait_for_platform_shortcut_modifiers_to_release() -> Result<(), String> {
    Ok(())
}

fn any_windows_shortcut_modifier_pressed(mut is_pressed: impl FnMut(i32) -> bool) -> bool {
    [0x10, 0x11, 0x12, 0x5b, 0x5c]
        .into_iter()
        .any(&mut is_pressed)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeyPress {
    Down(enigo::Key),
    Click(enigo::Key),
    Up(enigo::Key),
}

trait ShortcutKeySink {
    fn key_down(&mut self, key: enigo::Key);
    fn key_click(&mut self, key: enigo::Key);
    fn key_up(&mut self, key: enigo::Key);
}

impl ShortcutKeySink for enigo::Enigo {
    fn key_down(&mut self, key: enigo::Key) {
        use enigo::KeyboardControllable;

        KeyboardControllable::key_down(self, key);
    }

    fn key_click(&mut self, key: enigo::Key) {
        use enigo::KeyboardControllable;

        KeyboardControllable::key_click(self, key);
    }

    fn key_up(&mut self, key: enigo::Key) {
        use enigo::KeyboardControllable;

        KeyboardControllable::key_up(self, key);
    }
}

fn press_copy_shortcut() -> Result<(), String> {
    let mut enigo = enigo::Enigo::new();
    press_copy_shortcut_with(&mut enigo)
}

fn press_copy_shortcut_with(key_sink: &mut impl ShortcutKeySink) -> Result<(), String> {
    for key_press in copy_shortcut_presses() {
        match key_press {
            KeyPress::Down(key) => key_sink.key_down(key),
            KeyPress::Click(key) => key_sink.key_click(key),
            KeyPress::Up(key) => key_sink.key_up(key),
        }
    }

    Ok(())
}

fn copy_shortcut_presses() -> [KeyPress; 3] {
    [
        KeyPress::Down(enigo::Key::Control),
        KeyPress::Click(enigo::Key::Layout('c')),
        KeyPress::Up(enigo::Key::Control),
    ]
}

#[cfg(target_os = "windows")]
pub(super) fn clipboard_change_count() -> Result<i64, String> {
    Ok(unsafe { GetClipboardSequenceNumber() as i64 })
}

#[cfg(not(target_os = "windows"))]
pub(super) fn clipboard_change_count() -> Result<i64, String> {
    Err("Windows clipboard monitoring is unavailable on this platform".to_string())
}

fn read_clipboard_text() -> Result<String, String> {
    use arboard::Clipboard;

    let mut clipboard = Clipboard::new().map_err(|e| format!("Failed to open clipboard: {e}"))?;
    clipboard
        .get_text()
        .map_err(|e| format!("Failed to read selected text from clipboard: {e}"))
}

#[cfg(target_os = "windows")]
extern "system" {
    fn GetClipboardSequenceNumber() -> u32;
}

#[cfg(test)]
mod windows {
    mod shortcut_copy {
        use crate::domain::{MethodAvailability, SelectionContext, SelectionMethodKind};

        use super::super::{
            press_copy_shortcut_with, KeyPress, ShortcutCopySelectionMethod, ShortcutKeySink,
        };
        use crate::application::selected_text::SelectionMethod;

        #[test]
        fn method_kind_is_shortcut_copy() {
            let method = ShortcutCopySelectionMethod;

            assert_eq!(method.kind(), SelectionMethodKind::ShortcutCopy);
        }

        #[test]
        fn availability_is_explicit() {
            let method = ShortcutCopySelectionMethod;

            assert_eq!(
                method.availability(&SelectionContext::default()),
                MethodAvailability::Available
            );
        }

        #[test]
        fn checks_each_windows_shortcut_modifier_key() {
            let mut checked = Vec::new();

            let pressed = super::super::any_windows_shortcut_modifier_pressed(|key| {
                checked.push(key);
                key == 0x12
            });

            assert!(pressed);
            assert_eq!(checked, vec![0x10, 0x11, 0x12]);
        }

        #[tokio::test]
        async fn waits_until_all_shortcut_modifiers_are_released() {
            let modifier_states =
                std::sync::Arc::new(std::sync::Mutex::new(vec![true, true, false]));
            let read_modifier_states = modifier_states.clone();

            super::super::wait_for_shortcut_modifiers_to_clear_with(
                move || Ok(read_modifier_states.lock().unwrap().remove(0)),
                std::time::Duration::from_millis(100),
                std::time::Duration::ZERO,
            )
            .await
            .unwrap();

            assert!(modifier_states.lock().unwrap().is_empty());
        }

        #[test]
        fn helper_presses_ctrl_c() {
            let mut sink = RecordingKeySink::default();

            press_copy_shortcut_with(&mut sink).unwrap();

            assert_eq!(
                sink.presses,
                vec![
                    KeyPress::Down(enigo::Key::Control),
                    KeyPress::Click(enigo::Key::Layout('c')),
                    KeyPress::Up(enigo::Key::Control),
                ]
            );
        }

        #[derive(Default)]
        struct RecordingKeySink {
            presses: Vec<KeyPress>,
        }

        impl ShortcutKeySink for RecordingKeySink {
            fn key_down(&mut self, key: enigo::Key) {
                self.presses.push(KeyPress::Down(key));
            }

            fn key_click(&mut self, key: enigo::Key) {
                self.presses.push(KeyPress::Click(key));
            }

            fn key_up(&mut self, key: enigo::Key) {
                self.presses.push(KeyPress::Up(key));
            }
        }
    }
}

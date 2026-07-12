use std::time::Duration;

use async_trait::async_trait;

use crate::application::selected_text::SelectionMethod;
use crate::domain::{
    MethodAvailability, SelectionAttempt, SelectionContext, SelectionMethodKind, SelectionSource,
};
use crate::infrastructure::system::selection::common::clipboard_transaction;

const CLIPBOARD_TIMEOUT: Duration = Duration::from_millis(700);
const CLIPBOARD_POLL_INTERVAL: Duration = Duration::from_millis(15);
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
    clipboard_transaction::wait_for_clipboard_text_after_action(
        || async { press_copy_shortcut() },
        clipboard_change_count,
        read_clipboard_text,
        CLIPBOARD_TIMEOUT,
        CLIPBOARD_POLL_INTERVAL,
    )
    .await
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
fn clipboard_change_count() -> Result<i64, String> {
    Ok(unsafe { GetClipboardSequenceNumber() as i64 })
}

#[cfg(not(target_os = "windows"))]
fn clipboard_change_count() -> Result<i64, String> {
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

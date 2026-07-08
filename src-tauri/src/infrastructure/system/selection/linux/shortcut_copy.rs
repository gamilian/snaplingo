use std::time::Duration;
#[cfg(target_os = "linux")]
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use async_trait::async_trait;

use crate::domain::{
    MethodAvailability, SelectionAttempt, SelectionContext, SelectionMethodKind, SelectionSource,
};
use crate::infrastructure::system::selection::backend::SelectionMethod;
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
        let result = acquire_shortcut_copy_text().await;
        selection_attempt_from_shortcut_copy_result(context, result)
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

fn selection_attempt_from_shortcut_copy_result(
    context: &SelectionContext,
    result: Result<String, String>,
) -> SelectionAttempt {
    match result {
        Ok(text) => SelectionAttempt::success(
            SelectionMethodKind::ShortcutCopy,
            SelectionSource::ShortcutCopy,
            text,
            context.clone(),
        ),
        Err(err) if err == EMPTY_SELECTION_ERROR => {
            SelectionAttempt::empty(SelectionMethodKind::ShortcutCopy, context.clone())
        }
        Err(err) => {
            SelectionAttempt::failed(SelectionMethodKind::ShortcutCopy, context.clone(), err)
        }
    }
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
    #[cfg(target_os = "linux")]
    {
        let mut enigo = enigo::Enigo::new();
        return press_copy_shortcut_with(&mut enigo);
    }

    #[cfg(not(target_os = "linux"))]
    {
        Err("Linux synthetic Ctrl+C is unavailable on this platform".to_string())
    }
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

#[cfg(target_os = "linux")]
fn clipboard_change_count() -> Result<i64, String> {
    use arboard::{Clipboard, Error, GetExtLinux, LinuxClipboardKind};

    let mut clipboard = Clipboard::new()
        .map_err(|e| format!("Failed to open clipboard before shortcut copy: {e}"))?;
    let text = match clipboard
        .get()
        .clipboard(LinuxClipboardKind::Clipboard)
        .text()
    {
        Ok(text) => text,
        Err(Error::ContentNotAvailable) => String::new(),
        Err(err) => {
            return Err(format!(
                "Failed to inspect clipboard before shortcut copy: {err}"
            ))
        }
    };

    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    Ok(hasher.finish() as i64)
}

#[cfg(not(target_os = "linux"))]
fn clipboard_change_count() -> Result<i64, String> {
    Err("Linux clipboard monitoring is unavailable on this platform".to_string())
}

#[cfg(target_os = "linux")]
fn read_clipboard_text() -> Result<String, String> {
    use arboard::{Clipboard, GetExtLinux, LinuxClipboardKind};

    let mut clipboard = Clipboard::new()
        .map_err(|e| format!("Failed to open clipboard after shortcut copy: {e}"))?;
    clipboard
        .get()
        .clipboard(LinuxClipboardKind::Clipboard)
        .text()
        .map_err(|e| {
            format!("Failed to read selected text from clipboard after shortcut copy: {e}")
        })
}

#[cfg(not(target_os = "linux"))]
fn read_clipboard_text() -> Result<String, String> {
    Err("Linux clipboard reads are unavailable on this platform".to_string())
}

#[cfg(test)]
mod linux {
    mod shortcut_copy {
        use crate::domain::{SelectionAttemptStatus, SelectionContext, SelectionMethodKind};

        use super::super::{
            selection_attempt_from_shortcut_copy_result, ShortcutCopySelectionMethod,
        };
        use crate::infrastructure::system::selection::backend::SelectionMethod;

        #[test]
        fn method_kind_is_shortcut_copy() {
            let method = ShortcutCopySelectionMethod;

            assert_eq!(method.kind(), SelectionMethodKind::ShortcutCopy);
        }

        #[test]
        fn adapter_returns_explicit_failure_messages() {
            let attempt = selection_attempt_from_shortcut_copy_result(
                &SelectionContext::default(),
                Err("Failed to inspect clipboard before shortcut copy".to_string()),
            );

            assert_eq!(
                attempt.status,
                SelectionAttemptStatus::Failed(
                    "Failed to inspect clipboard before shortcut copy".to_string()
                )
            );
        }
    }
}

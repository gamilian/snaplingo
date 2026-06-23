use std::future::Future;
use std::time::Duration;

use crate::infrastructure::system::selection::common::clipboard_transaction::wait_for_clipboard_text_after_action;

const CLIPBOARD_TIMEOUT: Duration = Duration::from_millis(700);
const CLIPBOARD_POLL_INTERVAL: Duration = Duration::from_millis(15);

pub async fn with_temporary_pasteboard_text<Action, ActionFuture>(
    action: Action,
) -> Result<String, String>
where
    Action: FnOnce() -> ActionFuture,
    ActionFuture: Future<Output = Result<(), String>>,
{
    let original_text = read_clipboard_text().ok();

    let result = wait_for_clipboard_text_after_action(
        action,
        clipboard_change_count,
        read_clipboard_text,
        CLIPBOARD_TIMEOUT,
        CLIPBOARD_POLL_INTERVAL,
    )
    .await;

    // This first slice restores plain text when available. Rich NSPasteboardItem
    // preservation is deliberately left out until it is implemented and tested.
    if let Some(text) = original_text {
        let _ = write_clipboard_text(text);
    }

    result
}

pub fn clipboard_change_count() -> Result<i64, String> {
    use objc2_app_kit::NSPasteboard;

    let pasteboard = NSPasteboard::generalPasteboard();
    Ok(pasteboard.changeCount() as i64)
}

pub fn read_clipboard_text() -> Result<String, String> {
    use arboard::Clipboard;

    let mut clipboard = Clipboard::new().map_err(|e| format!("Failed to open clipboard: {e}"))?;
    clipboard
        .get_text()
        .map_err(|e| format!("Failed to read selected text from clipboard: {e}"))
}

fn write_clipboard_text(text: String) -> Result<(), String> {
    use arboard::Clipboard;

    let mut clipboard = Clipboard::new().map_err(|e| format!("Failed to open clipboard: {e}"))?;
    clipboard
        .set_text(text)
        .map_err(|e| format!("Failed to restore clipboard text: {e}"))
}

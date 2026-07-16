use std::future::Future;
use std::time::Duration;

use crate::infrastructure::system::selection::common::clipboard_transaction::wait_for_clipboard_text_change_after_action;

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

    let result = wait_for_clipboard_text_change_after_action(
        action,
        clipboard_change_count,
        read_clipboard_text,
        CLIPBOARD_TIMEOUT,
        CLIPBOARD_POLL_INTERVAL,
    )
    .await;

    // Only restore if the pasteboard still contains the synthetic copy. A user may
    // press Cmd+C while selection acquisition is completing; that newer copy wins.
    if let (Some(text), Ok(change)) = (original_text, result.as_ref()) {
        if should_restore_temporary_clipboard(change.change_count, clipboard_change_count().ok()) {
            let _ = write_clipboard_text(text);
        }
    }

    result.map(|change| change.text)
}

fn should_restore_temporary_clipboard(
    observed_change_count: i64,
    current_change_count: Option<i64>,
) -> bool {
    current_change_count == Some(observed_change_count)
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

#[cfg(test)]
mod tests {
    use super::should_restore_temporary_clipboard;

    #[test]
    fn does_not_overwrite_a_newer_user_copy() {
        assert!(should_restore_temporary_clipboard(8, Some(8)));
        assert!(!should_restore_temporary_clipboard(8, Some(9)));
        assert!(!should_restore_temporary_clipboard(8, None));
    }
}

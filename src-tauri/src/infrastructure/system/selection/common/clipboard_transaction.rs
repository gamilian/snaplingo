use std::future::Future;
use std::time::Duration;

pub async fn wait_for_clipboard_text_after_action<Action, ActionFuture, ChangeCount, Read>(
    action: Action,
    mut clipboard_change_count: ChangeCount,
    read_clipboard_text: Read,
    timeout: Duration,
    poll_interval: Duration,
) -> Result<String, String>
where
    Action: FnOnce() -> ActionFuture,
    ActionFuture: Future<Output = Result<(), String>>,
    ChangeCount: FnMut() -> Result<i64, String>,
    Read: FnOnce() -> Result<String, String>,
{
    let before_change_count = clipboard_change_count()?;
    action().await?;

    let started_at = tokio::time::Instant::now();
    loop {
        let current_change_count = clipboard_change_count()?;
        if current_change_count != before_change_count {
            let text = read_clipboard_text()?;
            if text.trim().is_empty() {
                return Err("Selected text is empty".to_string());
            }
            return Ok(text);
        }

        if started_at.elapsed() >= timeout {
            return Err("Timed out waiting for selected text to reach clipboard".to_string());
        }

        tokio::time::sleep(poll_interval).await;
    }
}

#[cfg(test)]
mod clipboard_transaction_tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    #[tokio::test]
    async fn reads_text_only_after_change_count_changes() {
        let counts = Arc::new(Mutex::new(vec![0, 1]));
        let read_counts = counts.clone();

        let text = wait_for_clipboard_text_after_action(
            || async { Ok(()) },
            move || Ok(read_counts.lock().unwrap().remove(0)),
            || Ok("selected text".to_string()),
            Duration::ZERO,
            Duration::ZERO,
        )
        .await
        .unwrap();

        assert_eq!(text, "selected text");
    }

    #[tokio::test]
    async fn rejects_unchanged_clipboard_instead_of_history() {
        let err = wait_for_clipboard_text_after_action(
            || async { Ok(()) },
            || Ok(7),
            || panic!("stale clipboard text should not be read"),
            Duration::ZERO,
            Duration::ZERO,
        )
        .await
        .unwrap_err();

        assert_eq!(
            err,
            "Timed out waiting for selected text to reach clipboard"
        );
    }
}

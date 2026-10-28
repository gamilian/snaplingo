use std::future::Future;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardTextChange {
    pub text: String,
    pub before_change_count: i64,
    pub change_count: i64,
}

pub async fn wait_for_clipboard_text_after_action<Action, ActionFuture, ChangeCount, Read>(
    action: Action,
    clipboard_change_count: ChangeCount,
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
    wait_for_clipboard_text_change_after_action(
        action,
        clipboard_change_count,
        read_clipboard_text,
        timeout,
        poll_interval,
    )
    .await
    .map(|change| change.text)
}

pub async fn wait_for_clipboard_text_change_after_action<Action, ActionFuture, ChangeCount, Read>(
    action: Action,
    clipboard_change_count: ChangeCount,
    read_clipboard_text: Read,
    timeout: Duration,
    poll_interval: Duration,
) -> Result<ClipboardTextChange, String>
where
    Action: FnOnce() -> ActionFuture,
    ActionFuture: Future<Output = Result<(), String>>,
    ChangeCount: FnMut() -> Result<i64, String>,
    Read: FnOnce() -> Result<String, String>,
{
    wait_for_clipboard_text_change_after_action_observing(
        action,
        clipboard_change_count,
        read_clipboard_text,
        timeout,
        poll_interval,
        |_, _| {},
    )
    .await
}

pub async fn wait_for_clipboard_text_change_after_action_observing<
    Action,
    ActionFuture,
    ChangeCount,
    Read,
    Observe,
>(
    action: Action,
    mut clipboard_change_count: ChangeCount,
    read_clipboard_text: Read,
    timeout: Duration,
    poll_interval: Duration,
    mut observe_change: Observe,
) -> Result<ClipboardTextChange, String>
where
    Action: FnOnce() -> ActionFuture,
    ActionFuture: Future<Output = Result<(), String>>,
    ChangeCount: FnMut() -> Result<i64, String>,
    Read: FnOnce() -> Result<String, String>,
    Observe: FnMut(i64, i64),
{
    let before_change_count = clipboard_change_count()?;
    action().await?;

    let started_at = tokio::time::Instant::now();
    loop {
        let current_change_count = clipboard_change_count()?;
        if current_change_count != before_change_count {
            observe_change(before_change_count, current_change_count);
            let text = read_clipboard_text()?;
            if text.trim().is_empty() {
                return Err("Selected text is empty".to_string());
            }
            return Ok(ClipboardTextChange {
                text,
                before_change_count,
                change_count: current_change_count,
            });
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

    #[tokio::test]
    async fn exposes_the_observed_change_count_for_safe_restoration() {
        let counts = Arc::new(Mutex::new(vec![4, 5]));
        let read_counts = counts.clone();

        let change = wait_for_clipboard_text_change_after_action(
            || async { Ok(()) },
            move || Ok(read_counts.lock().unwrap().remove(0)),
            || Ok("selected text".to_string()),
            Duration::ZERO,
            Duration::ZERO,
        )
        .await
        .unwrap();

        assert_eq!(change.text, "selected text");
        assert_eq!(change.before_change_count, 4);
        assert_eq!(change.change_count, 5);
    }

    #[tokio::test]
    async fn records_a_clipboard_change_even_when_its_data_is_not_text() {
        let counts = Arc::new(Mutex::new(vec![4, 5]));
        let read_counts = counts.clone();
        let observed = Arc::new(Mutex::new(Vec::new()));
        let callback = observed.clone();
        let result = wait_for_clipboard_text_change_after_action_observing(
            || async { Ok(()) },
            move || Ok(read_counts.lock().unwrap().remove(0)),
            || Err("Clipboard format is not text".into()),
            Duration::ZERO,
            Duration::ZERO,
            move |before, change| callback.lock().unwrap().push((before, change)),
        )
        .await;

        assert_eq!(result.unwrap_err(), "Clipboard format is not text");
        assert_eq!(*observed.lock().unwrap(), vec![(4, 5)]);
    }

    #[tokio::test]
    async fn does_not_observe_a_change_that_happens_after_the_timeout() {
        let observed = Arc::new(Mutex::new(Vec::new()));
        let callback = observed.clone();
        let result = wait_for_clipboard_text_change_after_action_observing(
            || async { Ok(()) },
            || Ok(7),
            || panic!("unchanged clipboard should not be read"),
            Duration::ZERO,
            Duration::ZERO,
            move |before, change| callback.lock().unwrap().push((before, change)),
        )
        .await;

        assert!(result.is_err());
        assert!(observed.lock().unwrap().is_empty());
    }
}

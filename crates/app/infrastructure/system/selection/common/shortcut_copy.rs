use std::time::Duration;

pub async fn wait_for_shortcut_modifiers_to_clear_with<ReadModifiers>(
    mut shortcut_modifiers_pressed: ReadModifiers,
    timeout: Duration,
    poll_interval: Duration,
) -> Result<Duration, String>
where
    ReadModifiers: FnMut() -> Result<bool, String>,
{
    let started_at = tokio::time::Instant::now();

    loop {
        if !shortcut_modifiers_pressed()? {
            return Ok(started_at.elapsed());
        }

        if started_at.elapsed() >= timeout {
            return Err("Timed out waiting for shortcut modifier keys to be released".to_string());
        }

        tokio::time::sleep(poll_interval).await;
    }
}

#[cfg(test)]
mod shortcut_copy_tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[tokio::test]
    async fn waits_until_shortcut_modifiers_are_released_before_copy() {
        let modifier_states = Arc::new(Mutex::new(vec![true, true, false]));
        let read_modifier_states = modifier_states.clone();

        wait_for_shortcut_modifiers_to_clear_with(
            move || Ok(read_modifier_states.lock().unwrap().remove(0)),
            Duration::from_millis(100),
            Duration::ZERO,
        )
        .await
        .unwrap();

        assert!(modifier_states.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn returns_clear_error_when_shortcut_modifiers_stay_pressed() {
        let err =
            wait_for_shortcut_modifiers_to_clear_with(|| Ok(true), Duration::ZERO, Duration::ZERO)
                .await
                .unwrap_err();

        assert_eq!(
            err,
            "Timed out waiting for shortcut modifier keys to be released"
        );
    }
}

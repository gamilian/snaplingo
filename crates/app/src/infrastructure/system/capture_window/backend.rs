pub(super) const CAPTURE_WINDOW_LABEL: &str = "capture";

pub(super) fn capture_snapshot_window_labels_to_restore(
    _hidden_window_labels: &[String],
) -> Vec<String> {
    Vec::new()
}

pub fn capture_snapshot_hide_settle_delay_ms(hidden_window_labels: &[String]) -> u64 {
    if hidden_window_labels.is_empty() {
        return 0;
    }

    100
}

#[cfg(test)]
mod tests {

    #[test]
    fn does_not_restore_hidden_non_capture_windows_after_capture_snapshot() {
        assert!(super::capture_snapshot_window_labels_to_restore(&[
            "settings".to_string(),
            "capture".to_string(),
            "pin-pin-1".to_string(),
        ])
        .is_empty());
    }

    #[test]
    fn plans_capture_snapshot_settle_delay_after_hiding_windows() {
        assert_eq!(
            super::capture_snapshot_hide_settle_delay_ms(&["capture".to_string()]),
            100
        );
        assert_eq!(super::capture_snapshot_hide_settle_delay_ms(&[]), 0);
    }
}

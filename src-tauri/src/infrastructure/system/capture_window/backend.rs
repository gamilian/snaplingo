use std::path::PathBuf;

pub(super) const CAPTURE_WINDOW_LABEL: &str = "capture";

pub(super) fn normalized_capture_mode(mode: &str) -> &'static str {
    match mode {
        "screenshot"
        | "screenshot-copy"
        | "screenshot-ocr"
        | "silent-screenshot-ocr"
        | "screenshot-translate" => match mode {
            "screenshot-copy" => "screenshot-copy",
            "screenshot-ocr" => "screenshot-ocr",
            "silent-screenshot-ocr" => "silent-screenshot-ocr",
            "screenshot-translate" => "screenshot-translate",
            _ => "screenshot",
        },
        _ => "screenshot",
    }
}

fn capture_window_url(mode: &str) -> PathBuf {
    PathBuf::from(format!(
        "index.html?window=capture&mode={}",
        normalized_capture_mode(mode)
    ))
}

pub(super) fn capture_window_prewarm_url() -> PathBuf {
    PathBuf::from("index.html?window=capture")
}

pub(super) fn capture_window_url_with_session(mode: &str, session_id: &str) -> PathBuf {
    PathBuf::from(format!(
        "{}&sessionId={}",
        capture_window_url(mode).to_string_lossy(),
        session_id
    ))
}

pub(super) fn capture_snapshot_window_labels_to_hide(
    visible_window_labels: &[String],
) -> Vec<String> {
    visible_window_labels
        .iter()
        .filter(|label| label.as_str() == CAPTURE_WINDOW_LABEL)
        .cloned()
        .collect()
}

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
    fn capture_window_url_encodes_supported_mode() {
        assert_eq!(
            super::capture_window_url("screenshot-ocr").to_string_lossy(),
            "index.html?window=capture&mode=screenshot-ocr"
        );
    }

    #[test]
    fn capture_window_url_encodes_screenshot_copy_mode() {
        assert_eq!(
            super::capture_window_url("screenshot-copy").to_string_lossy(),
            "index.html?window=capture&mode=screenshot-copy"
        );
    }

    #[test]
    fn capture_window_url_encodes_silent_screenshot_ocr_mode() {
        assert_eq!(
            super::capture_window_url("silent-screenshot-ocr").to_string_lossy(),
            "index.html?window=capture&mode=silent-screenshot-ocr"
        );
    }

    #[test]
    fn capture_window_url_falls_back_to_screenshot_for_unknown_mode() {
        assert_eq!(
            super::capture_window_url("unknown").to_string_lossy(),
            "index.html?window=capture&mode=screenshot"
        );
    }

    #[test]
    fn capture_window_url_with_session_appends_session_id() {
        assert_eq!(
            super::capture_window_url_with_session("screenshot", "session-1").to_string_lossy(),
            "index.html?window=capture&mode=screenshot&sessionId=session-1"
        );
    }

    #[test]
    fn capture_window_prewarm_url_has_no_launch_mode() {
        assert_eq!(
            super::capture_window_prewarm_url().to_string_lossy(),
            "index.html?window=capture"
        );
    }

    #[test]
    fn hides_only_existing_capture_overlay_before_capture_snapshot() {
        assert_eq!(
            super::capture_snapshot_window_labels_to_hide(&[
                "settings".to_string(),
                "capture".to_string(),
                "pin-pin-1".to_string(),
            ]),
            vec!["capture".to_string()]
        );
    }

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

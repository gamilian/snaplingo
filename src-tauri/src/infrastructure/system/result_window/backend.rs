use std::path::PathBuf;

pub(super) const RESULT_WINDOW_LABEL: &str = "capture-result";

pub(super) fn result_window_url() -> PathBuf {
    PathBuf::from("index.html?window=capture-result")
}

#[cfg(test)]
mod tests {
    #[test]
    fn result_window_url_targets_capture_result_route() {
        assert_eq!(
            super::result_window_url().to_string_lossy(),
            "index.html?window=capture-result"
        );
    }

    #[test]
    fn result_window_label_matches_frontend_route() {
        assert_eq!(super::RESULT_WINDOW_LABEL, "capture-result");
    }
}

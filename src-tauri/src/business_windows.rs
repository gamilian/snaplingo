use tauri::Manager;

pub(crate) const CAPTURE_RESULT_WINDOW_LABEL: &str = "capture-result";
pub(crate) const CAPTURE_WINDOW_LABEL: &str = "capture";

pub(crate) fn is_business_window_label(label: &str) -> bool {
    label == CAPTURE_RESULT_WINDOW_LABEL
        || label == CAPTURE_WINDOW_LABEL
        || label.starts_with("pin-")
}

pub(crate) fn has_visible_business_window(app: &tauri::AppHandle) -> bool {
    app.webview_windows().into_iter().any(|(label, window)| {
        is_business_window_label(&label) && window.is_visible().unwrap_or(false)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_business_window_labels() {
        assert!(is_business_window_label("capture-result"));
        assert!(is_business_window_label("capture"));
        assert!(is_business_window_label("pin-abc"));
    }

    #[test]
    fn rejects_settings_and_unknown_labels() {
        assert!(!is_business_window_label("settings"));
        assert!(!is_business_window_label("random"));
    }
}

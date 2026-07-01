use tauri::{Manager, WebviewUrl, WebviewWindowBuilder, Window};

pub(crate) const SETTINGS_WINDOW_LABEL: &str = "main";

pub(crate) fn should_hide_settings_window_instead_of_close(window_label: &str) -> bool {
    window_label == SETTINGS_WINDOW_LABEL
}

pub(crate) fn show_settings_window(app: &tauri::AppHandle) -> Result<(), String> {
    let window = match app.get_webview_window(SETTINGS_WINDOW_LABEL) {
        Some(window) => window,
        None => WebviewWindowBuilder::new(
            app,
            SETTINGS_WINDOW_LABEL,
            WebviewUrl::App("index.html?window=settings".into()),
        )
        .title("SnapLingo")
        .inner_size(900.0, 650.0)
        .min_inner_size(700.0, 500.0)
        .resizable(true)
        .visible(false)
        .build()
        .map_err(|e| e.to_string())?,
    };

    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())
}

pub(crate) fn hide_settings_window(window: &Window) -> Result<(), String> {
    window.hide().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_window_label_stays_legacy_main_during_migration() {
        assert_eq!(SETTINGS_WINDOW_LABEL, "main");
    }

    #[test]
    fn should_hide_settings_window_on_close() {
        assert!(should_hide_settings_window_instead_of_close(
            SETTINGS_WINDOW_LABEL
        ));
    }

    #[test]
    fn does_not_hide_non_settings_windows_on_close() {
        assert!(!should_hide_settings_window_instead_of_close(
            "capture-result"
        ));
        assert!(!should_hide_settings_window_instead_of_close("capture"));
        assert!(!should_hide_settings_window_instead_of_close("pin-1"));
    }
}

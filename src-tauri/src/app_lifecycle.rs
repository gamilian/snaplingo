pub(crate) fn should_hide_window_instead_of_close(window_label: &str) -> bool {
    crate::settings_window::should_hide_settings_window_instead_of_close(window_label)
}

pub(crate) fn should_show_main_window_on_reopen_for_state(
    has_visible_windows: bool,
    is_capture_presentation_active: bool,
    has_visible_business_window: bool,
) -> bool {
    !has_visible_windows && !is_capture_presentation_active && !has_visible_business_window
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_settings_window_close_keeps_app_running_in_background() {
        assert!(should_hide_window_instead_of_close(
            crate::settings_window::SETTINGS_WINDOW_LABEL
        ));
    }

    #[test]
    fn auxiliary_windows_keep_default_close_behavior() {
        assert!(!should_hide_window_instead_of_close("capture"));
        assert!(!should_hide_window_instead_of_close("capture-result"));
        assert!(!should_hide_window_instead_of_close("pin-pin-1"));
    }

    #[test]
    fn dock_reopen_shows_main_window_when_all_windows_are_hidden() {
        assert!(should_show_main_window_on_reopen_for_state(
            false, false, false
        ));
    }

    #[test]
    fn dock_reopen_keeps_visible_windows_unchanged() {
        assert!(!should_show_main_window_on_reopen_for_state(
            true, false, false
        ));
    }

    #[test]
    fn capture_reopen_does_not_show_settings_window() {
        assert!(!should_show_main_window_on_reopen_for_state(
            false, true, false
        ));
    }

    #[test]
    fn business_window_reopen_does_not_show_settings_window() {
        assert!(!should_show_main_window_on_reopen_for_state(
            false, false, true
        ));
    }
}

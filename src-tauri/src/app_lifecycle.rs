use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub(crate) const MAIN_WINDOW_LABEL: &str = "main";
const BUSINESS_HOTKEY_REOPEN_SUPPRESSION_MS: u64 = 1_500;
static SUPPRESS_MAIN_WINDOW_REOPEN_UNTIL_MS: AtomicU64 = AtomicU64::new(0);

pub(crate) fn should_hide_window_instead_of_close(window_label: &str) -> bool {
    window_label == MAIN_WINDOW_LABEL
}

pub(crate) fn should_show_main_window_on_reopen_for_state(
    has_visible_windows: bool,
    is_capture_presentation_active: bool,
    has_visible_business_window: bool,
    is_business_hotkey_reopen_suppressed: bool,
) -> bool {
    !has_visible_windows
        && !is_capture_presentation_active
        && !has_visible_business_window
        && !is_business_hotkey_reopen_suppressed
}

pub(crate) fn suppress_main_window_reopen_after_hotkey() {
    let suppress_until =
        current_time_millis().saturating_add(BUSINESS_HOTKEY_REOPEN_SUPPRESSION_MS);
    SUPPRESS_MAIN_WINDOW_REOPEN_UNTIL_MS.store(suppress_until, Ordering::Relaxed);
}

pub(crate) fn is_main_window_reopen_suppressed() -> bool {
    current_time_millis() < SUPPRESS_MAIN_WINDOW_REOPEN_UNTIL_MS.load(Ordering::Relaxed)
}

fn current_time_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_settings_window_close_keeps_app_running_in_background() {
        assert!(should_hide_window_instead_of_close(MAIN_WINDOW_LABEL));
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
            false, false, false, false
        ));
    }

    #[test]
    fn dock_reopen_keeps_visible_windows_unchanged() {
        assert!(!should_show_main_window_on_reopen_for_state(
            true, false, false, false
        ));
    }

    #[test]
    fn capture_reopen_does_not_show_settings_window() {
        assert!(!should_show_main_window_on_reopen_for_state(
            false, true, false, false
        ));
    }

    #[test]
    fn business_window_reopen_does_not_show_settings_window() {
        assert!(!should_show_main_window_on_reopen_for_state(
            false, false, true, false
        ));
    }

    #[test]
    fn business_hotkey_reopen_does_not_show_settings_window() {
        assert!(!should_show_main_window_on_reopen_for_state(
            false, false, false, true
        ));
    }
}

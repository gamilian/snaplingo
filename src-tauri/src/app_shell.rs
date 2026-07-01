#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AppShellMode {
    MenuBar,
    DockDebug,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReopenAction {
    Ignore,
    ShowSettings,
}

pub(crate) fn current_app_shell_mode() -> AppShellMode {
    #[cfg(target_os = "macos")]
    {
        AppShellMode::MenuBar
    }

    #[cfg(not(target_os = "macos"))]
    {
        AppShellMode::DockDebug
    }
}

pub(crate) fn reopen_action_for_mode(mode: AppShellMode) -> ReopenAction {
    match mode {
        AppShellMode::MenuBar => ReopenAction::Ignore,
        AppShellMode::DockDebug => ReopenAction::ShowSettings,
    }
}

pub(crate) fn handle_reopen(app: &tauri::AppHandle, has_visible_windows: bool) {
    match reopen_action_for_mode(current_app_shell_mode()) {
        ReopenAction::Ignore => {}
        ReopenAction::ShowSettings => {
            if !crate::app_lifecycle::should_show_main_window_on_reopen_for_state(
                has_visible_windows,
                crate::infrastructure::system::capture_window::is_capture_presentation_active(),
                crate::business_windows::has_visible_business_window(app),
                crate::app_lifecycle::is_main_window_reopen_suppressed(),
            ) {
                return;
            }

            if let Err(err) = crate::settings_window::show_settings_window(app) {
                log::warn!("Failed to show settings window on app reopen: {}", err);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_bar_mode_ignores_reopen() {
        assert_eq!(
            reopen_action_for_mode(AppShellMode::MenuBar),
            ReopenAction::Ignore
        );
    }

    #[test]
    fn dock_debug_mode_can_show_settings() {
        assert_eq!(
            reopen_action_for_mode(AppShellMode::DockDebug),
            ReopenAction::ShowSettings
        );
    }
}

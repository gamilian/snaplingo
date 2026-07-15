use serde::Serialize;
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder, Window};

pub(crate) const SETTINGS_WINDOW_LABEL: &str = "settings";
const SETTINGS_NAVIGATION_REQUESTED_EVENT: &str = "settings-navigation-requested";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SettingsWindowRoute {
    About,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct SettingsNavigationRequest {
    tab: &'static str,
    section: &'static str,
}

impl SettingsWindowRoute {
    fn request(self) -> SettingsNavigationRequest {
        match self {
            Self::About => SettingsNavigationRequest {
                tab: "general",
                section: "about",
            },
        }
    }
}

pub(crate) fn should_hide_settings_window_instead_of_close(window_label: &str) -> bool {
    window_label == SETTINGS_WINDOW_LABEL
}

pub(crate) fn show_settings_window(app: &tauri::AppHandle) -> Result<(), String> {
    show_settings_window_at(app, None)
}

pub(crate) fn show_settings_window_at(
    app: &tauri::AppHandle,
    route: Option<SettingsWindowRoute>,
) -> Result<(), String> {
    let (window, created) = match app.get_webview_window(SETTINGS_WINDOW_LABEL) {
        Some(window) => (window, false),
        None => (
            WebviewWindowBuilder::new(
                app,
                SETTINGS_WINDOW_LABEL,
                WebviewUrl::App(settings_window_url(route).into()),
            )
            .title("SnapLingo")
            .inner_size(900.0, 650.0)
            .min_inner_size(700.0, 500.0)
            .resizable(true)
            .visible(false)
            .build()
            .map_err(|e| e.to_string())?,
            true,
        ),
    };

    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    if !created {
        if let Some(route) = route {
            window
                .emit(SETTINGS_NAVIGATION_REQUESTED_EVENT, route.request())
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn settings_window_url(route: Option<SettingsWindowRoute>) -> String {
    match route {
        Some(SettingsWindowRoute::About) => {
            "index.html?window=settings&tab=general&section=about".to_string()
        }
        None => "index.html?window=settings".to_string(),
    }
}

pub(crate) fn hide_settings_window(window: &Window) -> Result<(), String> {
    window.hide().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_window_label_is_settings_domain_name() {
        assert_eq!(SETTINGS_WINDOW_LABEL, "settings");
    }

    #[test]
    fn about_route_is_encoded_for_a_new_settings_window() {
        assert_eq!(
            settings_window_url(Some(SettingsWindowRoute::About)),
            "index.html?window=settings&tab=general&section=about"
        );
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

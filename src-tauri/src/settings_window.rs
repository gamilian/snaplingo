use serde::Serialize;
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder, Window};

#[cfg(target_os = "macos")]
use objc2_app_kit::NSWindow;

pub(crate) const SETTINGS_WINDOW_LABEL: &str = "settings";
const SETTINGS_NAVIGATION_REQUESTED_EVENT: &str = "settings-navigation-requested";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SettingsWindowRoute {
    About,
    History,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct SettingsNavigationRequest {
    tab: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    section: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SettingsWindowGeometry {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

impl SettingsWindowRoute {
    fn request(self) -> SettingsNavigationRequest {
        match self {
            Self::About => SettingsNavigationRequest {
                tab: "general",
                section: Some("about"),
            },
            Self::History => SettingsNavigationRequest {
                tab: "history",
                section: None,
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
        None => {
            let mut builder = WebviewWindowBuilder::new(
                app,
                SETTINGS_WINDOW_LABEL,
                WebviewUrl::App(settings_window_url(route).into()),
            )
            .title("SnapLingo")
            .inner_size(900.0, 650.0)
            .min_inner_size(700.0, 500.0)
            .resizable(true)
            .visible(false);
            if let Some(geometry) = saved_settings_window_geometry(app) {
                builder = builder
                    .inner_size(geometry.width as f64, geometry.height as f64)
                    .position(geometry.x as f64, geometry.y as f64);
            }
            (builder.build().map_err(|e| e.to_string())?, true)
        }
    };

    configure_settings_window_for_deactivation(&window)?;
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

#[cfg(target_os = "macos")]
fn configure_settings_window_for_deactivation(window: &tauri::WebviewWindow) -> Result<(), String> {
    let ns_window = window.ns_window().map_err(|error| error.to_string())?;
    if ns_window.is_null() {
        return Err("Settings window has no native NSWindow".to_string());
    }

    let ns_window: &NSWindow = unsafe { &*ns_window.cast() };
    ns_window.setHidesOnDeactivate(settings_window_hides_on_deactivate());
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn configure_settings_window_for_deactivation(
    _window: &tauri::WebviewWindow,
) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "macos")]
fn settings_window_hides_on_deactivate() -> bool {
    false
}

fn settings_window_url(route: Option<SettingsWindowRoute>) -> String {
    match route {
        Some(SettingsWindowRoute::About) => {
            "index.html?window=settings&tab=general&section=about".to_string()
        }
        Some(SettingsWindowRoute::History) => "index.html?window=settings&tab=history".to_string(),
        None => "index.html?window=settings".to_string(),
    }
}

pub(crate) fn hide_settings_window(window: &Window) -> Result<(), String> {
    window.hide().map_err(|e| e.to_string())
}

pub(crate) fn remember_settings_window_geometry(window: &Window) -> Result<(), String> {
    let scale_factor = window.scale_factor().map_err(|error| error.to_string())?;
    let position = window.outer_position().map_err(|error| error.to_string())?;
    let size = window.inner_size().map_err(|error| error.to_string())?;
    let geometry = logical_settings_window_geometry(
        position.x,
        position.y,
        size.width,
        size.height,
        scale_factor,
    );
    save_settings_window_geometry(window.app_handle(), window.label(), geometry)
}

pub(crate) fn remember_settings_webview_window_geometry(
    window: &tauri::WebviewWindow,
) -> Result<(), String> {
    let scale_factor = window.scale_factor().map_err(|error| error.to_string())?;
    let position = window.outer_position().map_err(|error| error.to_string())?;
    let size = window.inner_size().map_err(|error| error.to_string())?;
    let geometry = logical_settings_window_geometry(
        position.x,
        position.y,
        size.width,
        size.height,
        scale_factor,
    );
    save_settings_window_geometry(window.app_handle(), window.label(), geometry)
}

fn save_settings_window_geometry(
    app: &tauri::AppHandle,
    window_label: &str,
    geometry: SettingsWindowGeometry,
) -> Result<(), String> {
    if window_label != SETTINGS_WINDOW_LABEL {
        return Ok(());
    }
    let state = app.state::<crate::AppState>();
    let mut general = state
        .settings
        .configuration
        .snapshot()
        .map_err(|error| error.to_string())?
        .general;
    general.settings_window_x = Some(geometry.x);
    general.settings_window_y = Some(geometry.y);
    general.settings_window_width = Some(geometry.width);
    general.settings_window_height = Some(geometry.height);
    state
        .settings
        .configuration
        .update_general(general)
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn logical_settings_window_geometry(
    physical_x: i32,
    physical_y: i32,
    physical_width: u32,
    physical_height: u32,
    scale_factor: f64,
) -> SettingsWindowGeometry {
    SettingsWindowGeometry {
        x: (physical_x as f64 / scale_factor).round() as i32,
        y: (physical_y as f64 / scale_factor).round() as i32,
        width: (physical_width as f64 / scale_factor).round() as u32,
        height: (physical_height as f64 / scale_factor).round() as u32,
    }
}

fn saved_settings_window_geometry(app: &tauri::AppHandle) -> Option<SettingsWindowGeometry> {
    let state = app.try_state::<crate::AppState>()?;
    let general = state.settings.configuration.snapshot().ok()?.general;
    Some(SettingsWindowGeometry {
        x: general.settings_window_x?,
        y: general.settings_window_y?,
        width: general.settings_window_width?,
        height: general.settings_window_height?,
    })
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
    fn history_route_is_encoded_for_a_new_settings_window() {
        assert_eq!(
            settings_window_url(Some(SettingsWindowRoute::History)),
            "index.html?window=settings&tab=history"
        );
        assert_eq!(SettingsWindowRoute::History.request().tab, "history");
        assert_eq!(SettingsWindowRoute::History.request().section, None);
    }

    #[test]
    fn stores_settings_window_geometry_in_logical_pixels() {
        assert_eq!(
            logical_settings_window_geometry(-200, 120, 1800, 1300, 2.0),
            SettingsWindowGeometry {
                x: -100,
                y: 60,
                width: 900,
                height: 650,
            }
        );
    }

    #[test]
    fn should_hide_settings_window_on_close() {
        assert!(should_hide_settings_window_instead_of_close(
            SETTINGS_WINDOW_LABEL
        ));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn settings_window_stays_visible_when_application_deactivates() {
        assert!(!settings_window_hides_on_deactivate());
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

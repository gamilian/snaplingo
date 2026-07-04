#[cfg(target_os = "macos")]
use objc2_app_kit::{
    NSScreenSaverWindowLevel, NSWindow, NSWindowAnimationBehavior, NSWindowCollectionBehavior,
    NSWindowStyleMask,
};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use super::backend::{result_window_url, RESULT_WINDOW_LABEL};

pub fn show_or_create_result_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    let window = match app.get_webview_window(RESULT_WINDOW_LABEL) {
        Some(window) => window,
        None => WebviewWindowBuilder::new(
            app,
            RESULT_WINDOW_LABEL,
            WebviewUrl::App(result_window_url()),
        )
        .title("SnapLingo Result")
        .inner_size(660.0, 660.0)
        .position(120.0, 120.0)
        .decorations(false)
        .always_on_top(true)
        .visible_on_all_workspaces(true)
        .transparent(true)
        .visible(false)
        .skip_taskbar(true)
        .focused(false)
        .shadow(true)
        .build()
        .map_err(|e| e.to_string())?,
    };

    reveal_result_window(&window)?;
    Ok(window)
}

fn reveal_result_window(window: &WebviewWindow) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        configure_result_window_for_current_space(window)?;
        window.show().map_err(|e| e.to_string())?;
        let ns_window = window.ns_window().map_err(|e| e.to_string())?;
        if ns_window.is_null() {
            return Err("Result window has no native NSWindow".to_string());
        }

        let ns_window: &NSWindow = unsafe { &*ns_window.cast() };
        ns_window.orderFrontRegardless();
        window.set_focus().map_err(|e| e.to_string())?;
    }

    #[cfg(not(target_os = "macos"))]
    {
        window.show().map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[cfg(target_os = "macos")]
fn configure_result_window_for_current_space(window: &WebviewWindow) -> Result<(), String> {
    let ns_window = window.ns_window().map_err(|e| e.to_string())?;
    if ns_window.is_null() {
        return Err("Result window has no native NSWindow".to_string());
    }

    let ns_window: &NSWindow = unsafe { &*ns_window.cast() };
    ns_window.setStyleMask(ns_window.styleMask() | NSWindowStyleMask::Borderless);
    ns_window.setCollectionBehavior(
        ns_window.collectionBehavior()
            | NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::FullScreenAuxiliary
            | NSWindowCollectionBehavior::Stationary
            | NSWindowCollectionBehavior::Transient
            | NSWindowCollectionBehavior::IgnoresCycle,
    );
    ns_window.setLevel(NSScreenSaverWindowLevel);
    ns_window.setCanHide(false);
    ns_window.setHidesOnDeactivate(false);
    if result_window_disables_window_animation() {
        ns_window.setAnimationBehavior(NSWindowAnimationBehavior::None);
    }

    Ok(())
}

#[cfg(target_os = "macos")]
fn result_window_disables_window_animation() -> bool {
    true
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    #[test]
    fn result_window_disables_appkit_window_animation() {
        assert!(super::result_window_disables_window_animation());
    }
}

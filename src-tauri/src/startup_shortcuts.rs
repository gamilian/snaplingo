use tauri::Manager;

use crate::{commands, infrastructure, AppState, Result};

const SCREENSHOT_SHORTCUT: &str = "CmdOrCtrl+Shift+KeyR";
const SCREENSHOT_OCR_SHORTCUT: &str = "CmdOrCtrl+Shift+KeyS";

pub(crate) async fn register_startup_shortcuts(app: tauri::AppHandle) {
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    if let Err(e) = register_screenshot_shortcut(&app) {
        log::error!("Failed to register screenshot shortcut: {}", e);
    } else {
        log::info!("Screenshot shortcut registered: {}", SCREENSHOT_SHORTCUT);
    }

    if let Err(e) = register_screenshot_ocr_shortcut(&app) {
        log::error!("Failed to register screenshot OCR shortcut: {}", e);
    } else {
        log::info!(
            "Screenshot OCR shortcut registered: {}",
            SCREENSHOT_OCR_SHORTCUT
        );
    }

    if let Err(e) = register_pin_shortcut(&app) {
        log::error!("Failed to register pinned image shortcut: {}", e);
    } else {
        log::info!("Pinned image shortcut registered: F3");
    }

    if let Err(e) = register_pin_toggle_shortcut(&app) {
        log::error!("Failed to register pinned image toggle shortcut: {}", e);
    } else {
        log::info!("Pinned image toggle shortcut registered: Shift+F3");
    }

    if let Err(e) = register_pin_group_switch_shortcut(&app) {
        log::error!(
            "Failed to register pinned image group switch shortcut: {}",
            e
        );
    } else {
        log::info!("Pinned image group switch shortcut registered: Cmd+F3");
    }
}

fn register_screenshot_shortcut(app: &tauri::AppHandle) -> Result<()> {
    let app_clone = app.clone();

    infrastructure::system::register_shortcut(app, SCREENSHOT_SHORTCUT, move || {
        log::info!("Screenshot shortcut triggered!");

        let app = app_clone.clone();
        tauri::async_runtime::spawn(commands::open_capture_window_from_shortcut(
            app,
            "screenshot",
        ));
    })?;

    Ok(())
}

fn register_screenshot_ocr_shortcut(app: &tauri::AppHandle) -> Result<()> {
    let app_clone = app.clone();

    infrastructure::system::register_shortcut(app, SCREENSHOT_OCR_SHORTCUT, move || {
        log::info!("Screenshot OCR shortcut triggered!");

        let app = app_clone.clone();
        tauri::async_runtime::spawn(commands::open_capture_window_from_shortcut(
            app,
            "screenshot-ocr",
        ));
    })?;

    Ok(())
}

fn register_pin_shortcut(app: &tauri::AppHandle) -> Result<()> {
    let app_clone = app.clone();

    infrastructure::system::register_shortcut(app, "F3", move || {
        log::info!("Pinned image shortcut triggered!");

        let state = app_clone.state::<AppState>();
        if let Err(err) = commands::pin_clipboard_image_for_state(&app_clone, state.inner()) {
            log::error!("Failed to pin clipboard image: {}", err);
        }
    })?;

    Ok(())
}

fn register_pin_toggle_shortcut(app: &tauri::AppHandle) -> Result<()> {
    let app_clone = app.clone();

    infrastructure::system::register_shortcut(app, "Shift+F3", move || {
        log::info!("Pinned image toggle shortcut triggered!");

        if let Err(err) = commands::toggle_pinned_images_visibility(app_clone.clone()) {
            log::error!("Failed to toggle pinned images: {}", err);
        }
    })?;

    Ok(())
}

fn register_pin_group_switch_shortcut(app: &tauri::AppHandle) -> Result<()> {
    let app_clone = app.clone();

    infrastructure::system::register_shortcut(app, "Cmd+F3", move || {
        log::info!("Pinned image group switch shortcut triggered!");

        let state = app_clone.state::<AppState>();
        if let Err(err) = commands::switch_pinned_image_group_for_state(&app_clone, state.inner()) {
            log::error!("Failed to switch pinned image group: {}", err);
        }
    })?;

    Ok(())
}

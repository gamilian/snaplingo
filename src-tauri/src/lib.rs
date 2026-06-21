// Module declarations
mod app_state;
mod application;
mod commands;
mod composition;
mod domain;
mod error;
mod infrastructure;

// Public exports for new infrastructure layer
pub use app_state::{AppState, ScreenshotState};
pub use application::*;
pub use domain::*;
pub use error::{AppError, Result};
pub use infrastructure::*;

use tauri::Manager;

const SCREENSHOT_SHORTCUT: &str = "CmdOrCtrl+Shift+KeyR";
const SCREENSHOT_OCR_SHORTCUT: &str = "CmdOrCtrl+Shift+KeyS";

/// Register screenshot shortcut
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

/// Register screenshot OCR shortcut
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

/// Register pinned image shortcut
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

/// Register pinned image toggle shortcut
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

/// Register pinned image group switch shortcut
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config_dir = dirs::home_dir()
        .expect("Cannot determine home directory")
        .join(".snaplingo");
    std::fs::create_dir_all(&config_dir).expect("Failed to create config directory");
    let config_path = config_dir.join("config.json");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_screenshots::init())
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .setup(|app| {
            let app_state = composition::build_app_state(config_path, app.handle().clone());
            composition::subscribe_history_service(&app_state);

            app.manage(app_state);

            // Register global shortcuts using Tauri plugin after setup completes
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                if let Err(e) = register_screenshot_shortcut(&app_handle) {
                    log::error!("Failed to register screenshot shortcut: {}", e);
                } else {
                    log::info!("Screenshot shortcut registered: {}", SCREENSHOT_SHORTCUT);
                }

                if let Err(e) = register_screenshot_ocr_shortcut(&app_handle) {
                    log::error!("Failed to register screenshot OCR shortcut: {}", e);
                } else {
                    log::info!(
                        "Screenshot OCR shortcut registered: {}",
                        SCREENSHOT_OCR_SHORTCUT
                    );
                }

                if let Err(e) = register_pin_shortcut(&app_handle) {
                    log::error!("Failed to register pinned image shortcut: {}", e);
                } else {
                    log::info!("Pinned image shortcut registered: F3");
                }

                if let Err(e) = register_pin_toggle_shortcut(&app_handle) {
                    log::error!("Failed to register pinned image toggle shortcut: {}", e);
                } else {
                    log::info!("Pinned image toggle shortcut registered: Shift+F3");
                }

                if let Err(e) = register_pin_group_switch_shortcut(&app_handle) {
                    log::error!(
                        "Failed to register pinned image group switch shortcut: {}",
                        e
                    );
                } else {
                    log::info!("Pinned image group switch shortcut registered: Cmd+F3");
                }
            });

            // TODO: Create system tray

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::open_result_window,
            commands::open_translation_result_window,
            commands::trigger_screenshot,
            commands::open_capture_window,
            commands::create_screenshot_window,
            commands::create_screenshot_window_simple,
            commands::screenshot_overlay_ready,
            commands::close_screenshot_window,
            commands::crop_screenshot,
            commands::translate_text_v2,
            commands::list_translation_providers,
            commands::activate_translation_provider,
            commands::deactivate_translation_provider,
            commands::reorder_active_translation_providers,
            commands::configure_translation_provider,
            commands::get_provider_credential_schema,
            commands::configure_translation_provider_credentials,
            commands::add_custom_translation_provider,
            commands::remove_custom_translation_provider,
            commands::recognize_image,
            commands::list_ocr_providers,
            commands::activate_ocr_provider,
            commands::configure_ocr_provider,
            commands::capture_full_screen,
            commands::capture_region,
            commands::save_screenshot,
            commands::create_capture_session,
            commands::get_capture_session,
            commands::cancel_capture_session,
            commands::restore_capture_snapshot_windows_for_session,
            commands::render_capture_output,
            commands::default_capture_save_path,
            commands::quick_capture_save_path,
            commands::output_capture,
            commands::pin_clipboard_image,
            commands::get_pinned_image,
            commands::copy_pinned_image,
            commands::replace_pinned_image_from_clipboard,
            commands::save_pinned_image,
            commands::close_pinned_image,
            commands::remove_pinned_image,
            commands::toggle_pinned_images_visibility,
            commands::switch_pinned_image_group,
            commands::move_pinned_image_to_next_group,
            commands::hide_pinned_image_group,
            commands::destroy_pinned_image_group,
            commands::run_capture_ocr,
            commands::get_translation_history,
            commands::get_ocr_history,
            commands::search_history,
            commands::delete_history,
            commands::clear_all_history,
            commands::trigger_workflow,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

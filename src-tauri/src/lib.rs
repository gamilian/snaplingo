// Module declarations
mod app_state;
mod application;
mod commands;
mod composition;
mod domain;
mod error;
mod infrastructure;
mod startup_shortcuts;

// Public exports for new infrastructure layer
pub use app_state::{AppState, ScreenshotState};
pub use application::*;
pub use domain::*;
pub use error::{AppError, Result};
pub use infrastructure::*;

use tauri::Manager;

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

            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(startup_shortcuts::register_startup_shortcuts(app_handle));

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

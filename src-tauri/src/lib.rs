// Module declarations
mod app_lifecycle;
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
#[allow(ambiguous_glob_reexports)]
pub use domain::*;
pub use error::{AppError, Result};
#[allow(ambiguous_glob_reexports)]
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
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_screenshots::init())
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .on_window_event(|window, event| {
            if !app_lifecycle::should_hide_window_instead_of_close(window.label()) {
                return;
            }

            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if let Err(err) = window.hide() {
                    log::warn!("Failed to hide main window on close request: {}", err);
                }
            }
        })
        .setup(|app| {
            let app_state = composition::build_app_state(config_path, app.handle().clone());
            composition::subscribe_history_service(&app_state);
            composition::hydrate_provider_credentials_in_background(&app_state);

            app.manage(app_state);

            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(startup_shortcuts::register_startup_shortcuts(app_handle));

            if let Err(err) =
                infrastructure::system::capture_window::prewarm_capture_window(app.handle())
            {
                log::warn!("Failed to prewarm capture window: {}", err);
            }

            // TODO: Create system tray

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::open_result_window,
            commands::open_ocr_result_window,
            commands::open_translation_result_window,
            commands::open_capture_ocr_result_window,
            commands::open_capture_translation_result_window,
            commands::take_capture_result_window_payload,
            commands::open_selection_translation_window,
            commands::copy_text_to_clipboard,
            commands::configure_hotkey,
            commands::configure_translation_hotkey,
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
            commands::update_custom_translation_provider,
            commands::remove_custom_translation_provider,
            commands::list_translation_prompt_strategies,
            commands::save_translation_prompt_strategies,
            commands::list_openai_compatible_models,
            commands::test_openai_compatible_provider,
            commands::test_openai_responses_provider,
            commands::list_anthropic_models,
            commands::test_anthropic_provider,
            commands::list_gemini_models,
            commands::test_gemini_provider,
            commands::test_custom_translation_provider,
            commands::recognize_image,
            commands::recognize_image_file,
            commands::list_ocr_providers,
            commands::activate_ocr_provider,
            commands::configure_ocr_provider,
            commands::get_ocr_provider_credential_schema,
            commands::configure_ocr_provider_credentials,
            commands::capture_full_screen,
            commands::capture_region,
            commands::save_screenshot,
            commands::create_capture_session,
            commands::get_capture_session,
            commands::hydrate_capture_session_snapshots,
            commands::log_capture_frontend_perf,
            commands::current_capture_cursor_position,
            commands::cancel_capture_session,
            commands::prepare_capture_window_for_reveal,
            commands::reveal_capture_window,
            commands::hide_capture_window,
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
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Reopen {
                has_visible_windows,
                ..
            } = event
            {
                let has_visible_capture_result_window = app_handle
                    .get_webview_window(commands::CAPTURE_RESULT_WINDOW_LABEL)
                    .and_then(|window| window.is_visible().ok())
                    .unwrap_or(false);

                if !app_lifecycle::should_show_main_window_on_reopen_for_state(
                    has_visible_windows,
                    infrastructure::system::capture_window::is_capture_presentation_active(),
                    has_visible_capture_result_window,
                    app_lifecycle::is_main_window_reopen_suppressed(),
                ) {
                    return;
                }

                let Some(window) = app_handle.get_webview_window(app_lifecycle::MAIN_WINDOW_LABEL)
                else {
                    return;
                };

                if let Err(err) = window.show() {
                    log::warn!("Failed to show main window on app reopen: {}", err);
                    return;
                }

                if let Err(err) = window.set_focus() {
                    log::warn!("Failed to focus main window on app reopen: {}", err);
                }
            }
        });
}

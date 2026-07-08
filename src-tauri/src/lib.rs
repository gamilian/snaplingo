// Module declarations
mod app_shell;
mod app_state;
mod application;
mod commands;
mod composition;
mod domain;
mod error;
mod infrastructure;
mod settings_window;
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
            if !settings_window::should_hide_settings_window_instead_of_close(window.label()) {
                return;
            }

            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if let Err(err) = settings_window::hide_settings_window(window) {
                    log::warn!("Failed to hide settings window on close request: {}", err);
                }
            }
        })
        .setup(|app| {
            let app_state = composition::build_app_state(config_path, app.handle().clone());
            composition::subscribe_history_service(&app_state);
            composition::hydrate_provider_credentials_in_background(&app_state);
            let hotkey_runtime = app_state.hotkey_runtime.clone();

            app.manage(app_state);

            if let Err(err) = app_shell::apply_resting_activation_policy(app.handle()) {
                log::warn!("Failed to apply resting activation policy: {}", err);
            }

            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                if let Err(err) = hotkey_runtime.register_startup_hotkeys(&app_handle) {
                    log::error!("Failed to register startup hotkeys: {}", err);
                }
            });

            if let Err(err) =
                infrastructure::system::capture_window::prewarm_capture_window(app.handle())
            {
                log::warn!("Failed to prewarm capture window: {}", err);
            }

            if let Err(err) = app_shell::setup_menu_bar(app) {
                log::warn!("Failed to setup menu bar: {}", err);
            }

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
            commands::get_hotkey_snapshot,
            commands::update_hotkey,
            commands::get_settings_snapshot,
            commands::trigger_screenshot,
            commands::update_general_settings,
            commands::update_screenshot_settings,
            commands::update_translation_settings,
            commands::open_capture_window,
            commands::translate_text_v2,
            commands::translate_text_with_provider,
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
        .run(|_, event| {
            if let tauri::RunEvent::ExitRequested { code, api, .. } = event {
                if app_shell::should_prevent_implicit_exit(code) {
                    api.prevent_exit();
                }
            }
        });
}

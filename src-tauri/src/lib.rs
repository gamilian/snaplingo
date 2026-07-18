// Module declarations
mod app_actions;
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
pub use app_state::AppState;
#[allow(ambiguous_glob_reexports)]
pub use application::*;
#[allow(ambiguous_glob_reexports)]
pub use domain::*;
pub use error::{AppError, Result};
#[allow(ambiguous_glob_reexports)]
pub use infrastructure::*;

use std::sync::Arc;

use chrono::{Days, Local, NaiveDateTime, TimeZone};
use tauri::Manager;
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_log::{Target, TargetKind};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let database_path = infrastructure::system::get_database_path()
        .expect("Failed to resolve application database path");
    let is_first_launch = !database_path.exists();
    let database = Arc::new(
        infrastructure::storage::Database::open(&database_path)
            .expect("Failed to initialize database"),
    );
    let log_repository = infrastructure::storage::SqliteAppLogRepository::new(database.clone());
    let app_database = database.clone();
    let app_database_path = database_path.clone();
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None::<Vec<&'static str>>,
        ))
        .plugin(tauri_plugin_screenshots::init())
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Debug)
                .clear_targets()
                .target(Target::new(TargetKind::Dispatch(
                    fern::Dispatch::new().chain(fern::Output::call(move |record| {
                        let _ = log_repository.record(
                            record.level().as_str(),
                            record.target(),
                            &record.args().to_string(),
                        );
                    })),
                )))
                .build(),
        )
        .on_window_event(|window, event| {
            if !settings_window::should_hide_settings_window_instead_of_close(window.label()) {
                return;
            }

            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if let Err(err) = settings_window::remember_settings_window_geometry(window) {
                    log::warn!("Failed to remember settings window geometry: {}", err);
                }
                if let Err(err) = settings_window::hide_settings_window(window) {
                    log::warn!("Failed to hide settings window on close request: {}", err);
                }
            }
        })
        .setup(move |app| {
            let app_state =
                composition::build_app_state(app_database, app_database_path, app.handle().clone());
            composition::subscribe_history(&app_state);
            let start_on_boot = app_state
                .settings
                .configuration
                .snapshot()
                .map(|snapshot| snapshot.general.start_on_boot)
                .unwrap_or(false);
            use tauri_plugin_autostart::ManagerExt;
            let autostart_result = if start_on_boot {
                app.handle().autolaunch().enable()
            } else {
                app.handle().autolaunch().disable()
            };
            if let Err(error) = autostart_result {
                log::warn!("Failed to synchronize start on boot: {}", error);
            }
            let hotkey_runtime = app_state.settings.hotkeys.clone();
            let permissions = app_state.permissions.clone();
            let permissions_granted = permissions.status().all_granted();
            let log_settings = app_state.settings.configuration.clone();
            let scheduled_log_repository = app_state.logs.repository.clone();

            app.manage(app_state);

            tauri::async_runtime::spawn(run_daily_log_cleanup(
                log_settings,
                scheduled_log_repository,
            ));

            if let Err(err) = app_shell::apply_resting_activation_policy(app.handle()) {
                log::warn!("Failed to apply resting activation policy: {}", err);
            }

            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                while !permissions.status().all_granted() {
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                }
                let registrar = infrastructure::system::TauriHotkeyRegistrar::new(
                    app_handle.clone(),
                    startup_shortcuts::trigger_hotkey_action,
                );
                if let Err(err) = hotkey_runtime.register_startup_hotkeys_with(&registrar) {
                    log::error!("Failed to register startup hotkeys: {}", err);
                }
                if let Err(err) =
                    infrastructure::system::capture_window::prewarm_capture_window(&app_handle)
                {
                    log::warn!("Failed to prewarm capture window: {}", err);
                }
            });

            if let Err(err) = app_shell::setup_menu_bar(app) {
                log::warn!("Failed to setup menu bar: {}", err);
            }

            if should_show_settings_on_startup(is_first_launch, permissions_granted) {
                if let Err(err) = settings_window::show_settings_window(app.handle()) {
                    log::error!("Failed to show startup settings window: {}", err);
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::open_result_window,
            commands::open_ocr_result_window,
            commands::open_translation_result_window,
            commands::open_capture_ocr_result_window,
            commands::open_capture_translation_result_window,
            commands::current_capture_result_window_request_id,
            commands::take_capture_result_window_payload,
            commands::open_selection_translation_window,
            commands::copy_text_to_clipboard,
            commands::get_hotkey_snapshot,
            commands::get_default_hotkey_snapshot,
            commands::update_hotkey,
            commands::reset_hotkey,
            commands::reset_hotkey_category,
            commands::get_settings_snapshot,
            commands::get_required_permissions_status,
            commands::request_required_permissions,
            commands::trigger_screenshot,
            commands::update_general_settings,
            commands::update_screenshot_settings,
            commands::update_annotation_colors,
            commands::update_translation_settings,
            commands::update_ocr_settings,
            commands::update_history_settings,
            commands::list_app_logs,
            commands::clear_app_logs,
            commands::list_system_tts_voices,
            commands::speak_text,
            commands::open_capture_window,
            commands::translate_text_v2,
            commands::translate_text_with_provider,
            commands::record_translation_history,
            commands::list_translation_providers,
            commands::activate_translation_provider,
            commands::deactivate_translation_provider,
            commands::reorder_active_translation_providers,
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
            commands::create_capture_session,
            commands::get_capture_session,
            commands::hydrate_capture_session_snapshots,
            commands::hydrate_capture_monitor_snapshot,
            commands::log_capture_frontend_perf,
            commands::current_capture_cursor_position,
            commands::current_capture_control_candidate,
            commands::move_capture_cursor,
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
            commands::query_translation_history,
            commands::query_ocr_history,
            commands::query_library_history_index,
            commands::query_library_favorite_index,
            commands::search_history,
            commands::delete_history,
            commands::favorite_translation_result,
            commands::favorite_ocr_result,
            commands::query_favorites,
            commands::update_favorite_metadata,
            commands::delete_favorite,
            commands::rerun_ocr_favorite,
            commands::list_favorite_tags,
            commands::update_history_note,
            commands::replace_history_tags,
            commands::clear_all_history,
            commands::clear_history,
            commands::rerun_ocr_history,
            commands::favorite_capture_selection,
            commands::query_screenshot_favorites,
            commands::update_screenshot_favorite_metadata,
            commands::delete_screenshot_favorite,
            commands::copy_screenshot_favorite,
            commands::reveal_screenshot_favorite,
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

fn should_show_settings_on_startup(is_first_launch: bool, permissions_granted: bool) -> bool {
    is_first_launch || !permissions_granted
}

fn next_daily_log_cleanup_at(now: NaiveDateTime) -> NaiveDateTime {
    let today_at_eight = now.date().and_hms_opt(8, 0, 0).expect("08:00 is valid");
    if now < today_at_eight {
        today_at_eight
    } else {
        today_at_eight
            .checked_add_days(Days::new(1))
            .expect("next cleanup date is valid")
    }
}

async fn run_daily_log_cleanup(
    settings: Arc<SettingsConfiguration>,
    repository: Arc<infrastructure::storage::SqliteAppLogRepository>,
) {
    loop {
        let now = Local::now();
        let next = Local
            .from_local_datetime(&next_daily_log_cleanup_at(now.naive_local()))
            .earliest()
            .expect("local 08:00 is valid");
        let delay = (next - now).to_std().unwrap_or_default();
        tokio::time::sleep(delay).await;

        let retention_days = settings
            .snapshot()
            .map(|snapshot| snapshot.general.log_retention_days)
            .unwrap_or(7);
        if let Err(error) = repository.delete_expired(retention_days) {
            log::warn!("Failed to delete expired application logs: {}", error);
        }
    }
}

#[cfg(test)]
mod log_cleanup_schedule_tests {
    use chrono::NaiveDate;

    use super::next_daily_log_cleanup_at;

    #[test]
    fn schedules_the_same_day_before_eight_and_the_next_day_at_or_after_eight() {
        let date = NaiveDate::from_ymd_opt(2026, 7, 16).unwrap();
        assert_eq!(
            next_daily_log_cleanup_at(date.and_hms_opt(7, 59, 59).unwrap()),
            date.and_hms_opt(8, 0, 0).unwrap()
        );
        assert_eq!(
            next_daily_log_cleanup_at(date.and_hms_opt(8, 0, 0).unwrap()),
            NaiveDate::from_ymd_opt(2026, 7, 17)
                .unwrap()
                .and_hms_opt(8, 0, 0)
                .unwrap()
        );
    }
}

#[cfg(test)]
mod startup_settings_tests {
    use super::should_show_settings_on_startup;

    #[test]
    fn shows_settings_on_first_launch_or_when_permissions_are_missing() {
        assert!(should_show_settings_on_startup(true, true));
        assert!(should_show_settings_on_startup(false, false));
        assert!(!should_show_settings_on_startup(false, true));
    }
}

use std::path::PathBuf;
use std::sync::Arc;

use tauri::AppHandle;

mod capture_runtime;
mod history_runtime;
mod provider_runtime;
mod selection_runtime;

use capture_runtime::build_capture_runtime;
use history_runtime::build_history_service;
use provider_runtime::{
    build_ocr_coordinator, build_translation_coordinator, hydrate_provider_credentials,
};
use selection_runtime::build_selected_text_acquirer;

pub(crate) use history_runtime::subscribe_history_service;

use crate::infrastructure::events::EventBus;
use crate::infrastructure::http::{HttpClient, ReqwestHttpClient};
use crate::infrastructure::storage::{ConfigFile, Keychain};
use crate::AppState;
use crate::{HotkeyConfiguration, HotkeyRuntime, SettingsConfiguration};

pub(crate) fn build_app_state(config_path: PathBuf, app: AppHandle) -> AppState {
    let config_file = Arc::new(ConfigFile::new(config_path));
    let settings_configuration = Arc::new(SettingsConfiguration::new(config_file.clone()));
    let hotkey_configuration = Arc::new(HotkeyConfiguration::new(config_file.clone()));
    let hotkey_runtime = Arc::new(HotkeyRuntime::new(hotkey_configuration));
    let keychain = Arc::new(Keychain::new());
    let http_client: Arc<dyn HttpClient> = Arc::new(ReqwestHttpClient::new());
    let event_bus = Arc::new(EventBus::new());

    let history_service = build_history_service();

    let translation_coordinator = build_translation_coordinator(
        config_file.clone(),
        keychain.clone(),
        http_client.clone(),
        event_bus.clone(),
    );
    let ocr_coordinator = build_ocr_coordinator(
        config_file.clone(),
        keychain.clone(),
        http_client.clone(),
        event_bus.clone(),
    );

    let capture_runtime = build_capture_runtime(app.clone(), ocr_coordinator.clone());
    let selected_text_acquirer = build_selected_text_acquirer(app);

    AppState {
        config_file,
        settings_configuration,
        hotkey_runtime,
        keychain,
        http_client,
        translation_coordinator,
        ocr_coordinator,
        capture_service: capture_runtime.capture_service,
        capture_session_service: capture_runtime.capture_session_service,
        image_composition_service: capture_runtime.image_composition_service,
        capture_output_service: capture_runtime.capture_output_service,
        capture_session_runtime: capture_runtime.capture_session_runtime,
        pinned_image_service: capture_runtime.pinned_image_service,
        screenshot_state: capture_runtime.screenshot_state,
        history_service,
        event_bus,
        selected_text_acquirer,
    }
}

pub(crate) fn hydrate_provider_credentials_in_background(app_state: &AppState) {
    let config_file = app_state.config_file.clone();
    let keychain = app_state.keychain.clone();
    let http_client = app_state.http_client.clone();
    let translation_coordinator = app_state.translation_coordinator.clone();
    let ocr_coordinator = app_state.ocr_coordinator.clone();

    tauri::async_runtime::spawn_blocking(move || {
        hydrate_provider_credentials(
            config_file,
            keychain,
            http_client,
            translation_coordinator,
            ocr_coordinator,
        );
    });
}

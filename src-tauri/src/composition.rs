use std::path::PathBuf;
use std::sync::Arc;

use tauri::AppHandle;

mod capture_runtime;
mod history_runtime;
mod provider_runtime;
mod selection_runtime;

use capture_runtime::build_capture_runtime;
use history_runtime::build_history;
use provider_runtime::{
    build_llm_introspection, build_llm_runtime, build_ocr_coordinator,
    build_provider_configuration, build_translation_coordinator, hydrate_provider_credentials,
};
use selection_runtime::build_selected_text_acquirer;

pub(crate) use history_runtime::subscribe_history;

use crate::app_state::{
    AppState, CaptureRuntimeState, HistoryRuntime, ProviderRuntime, SelectionRuntime,
    SettingsRuntime,
};
use crate::application::hotkeys::HotkeyStore;
use crate::application::providers::ocr::OcrProviderConfiguration;
use crate::application::providers::HttpClient;
use crate::application::providers::{
    ProviderConfigStore, ProviderCredentialStore, TranslationPromptConfiguration,
};
use crate::application::result_window::ResultWindowRuntime;
use crate::application::settings::SettingsStore;
use crate::infrastructure::events::EventBus;
use crate::infrastructure::http::ReqwestHttpClient;
use crate::infrastructure::storage::{ConfigFile, Keychain};
use crate::infrastructure::system::clipboard::ArboardResultWindowClipboard;
use crate::infrastructure::system::result_window::{
    TauriResultWindowNotifier, TauriResultWindowRuntimeHost,
};
use crate::{HotkeyConfiguration, HotkeyRuntime, SettingsConfiguration};

pub(crate) fn build_app_state(config_path: PathBuf, app: AppHandle) -> AppState {
    let config_file = Arc::new(ConfigFile::new(config_path));
    let settings_store: Arc<dyn SettingsStore> = config_file.clone();
    let hotkey_store: Arc<dyn HotkeyStore> = config_file.clone();
    let provider_config_store: Arc<dyn ProviderConfigStore> = config_file.clone();
    let settings_configuration = Arc::new(SettingsConfiguration::new(settings_store));
    let hotkey_configuration = Arc::new(HotkeyConfiguration::new(hotkey_store));
    let hotkey_runtime = Arc::new(HotkeyRuntime::new(hotkey_configuration));
    let keychain = Arc::new(Keychain::new());
    let provider_credential_store: Arc<dyn ProviderCredentialStore> = keychain.clone();
    let http_client: Arc<dyn HttpClient> = Arc::new(ReqwestHttpClient::new());
    let event_bus = Arc::new(EventBus::new());

    let history = build_history();

    let llm_runtime = build_llm_runtime(http_client.clone());
    let llm_introspection = build_llm_introspection(llm_runtime.clone());
    let translation_coordinator = build_translation_coordinator(
        provider_config_store.clone(),
        http_client.clone(),
        event_bus.clone(),
    );
    let provider_configuration = build_provider_configuration(
        provider_config_store.clone(),
        provider_credential_store.clone(),
        llm_runtime,
        translation_coordinator.clone(),
        llm_introspection.clone(),
    );
    let prompt_strategies = Arc::new(TranslationPromptConfiguration::new(
        provider_config_store.clone(),
    ));
    let ocr_coordinator = build_ocr_coordinator(
        provider_config_store.clone(),
        http_client.clone(),
        event_bus.clone(),
    );
    let ocr_configuration = Arc::new(OcrProviderConfiguration::new(
        ocr_coordinator.clone(),
        provider_credential_store.clone(),
    ));

    let capture_runtime = build_capture_runtime(app.clone(), ocr_coordinator.clone());
    let selected_text_acquirer = build_selected_text_acquirer(app.clone());
    let result_window = Arc::new(ResultWindowRuntime::new(
        Arc::new(TauriResultWindowRuntimeHost::new(app.clone())),
        Arc::new(ArboardResultWindowClipboard::new()),
        Arc::new(TauriResultWindowNotifier::new(app)),
    ));

    hydrate_provider_credentials_in_background(
        provider_credential_store,
        provider_configuration.clone(),
        ocr_coordinator.clone(),
    );

    AppState {
        settings: Arc::new(SettingsRuntime {
            configuration: settings_configuration,
            hotkeys: hotkey_runtime,
        }),
        providers: Arc::new(ProviderRuntime {
            translation: translation_coordinator,
            ocr: ocr_coordinator,
            ocr_configuration,
            llm_introspection,
            configuration: provider_configuration,
            prompt_strategies,
        }),
        capture: Arc::new(CaptureRuntimeState {
            sessions: capture_runtime.sessions,
            output: capture_runtime.output,
            runtime: capture_runtime.runtime,
            pinned_images: capture_runtime.pinned_images,
        }),
        history: Arc::new(HistoryRuntime {
            history,
            events: event_bus,
        }),
        selection: Arc::new(SelectionRuntime {
            acquirer: selected_text_acquirer,
        }),
        result_window,
    }
}

fn hydrate_provider_credentials_in_background(
    credential_store: Arc<dyn ProviderCredentialStore>,
    provider_configuration: Arc<crate::application::providers::ProviderConfiguration>,
    ocr_coordinator: Arc<crate::application::providers::ocr::OcrCoordinator>,
) {
    tauri::async_runtime::spawn_blocking(move || {
        hydrate_provider_credentials(provider_configuration, credential_store, ocr_coordinator);
    });
}

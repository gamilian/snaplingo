use std::path::PathBuf;
use std::sync::Arc;

use tauri::{AppHandle, Emitter};

mod capture_runtime;
mod history_runtime;
mod provider_runtime;
mod screenshot_favorites_runtime;
mod selection_runtime;

use capture_runtime::build_capture_runtime;
use history_runtime::{build_history, OcrCoordinatorHistoryRecognizer};
use provider_runtime::{
    build_llm_introspection, build_llm_runtime, build_ocr_coordinator,
    build_provider_configuration, build_translation_coordinator, hydrate_provider_credentials,
};
use screenshot_favorites_runtime::build_screenshot_favorites;
use selection_runtime::build_selected_text_acquirer;

pub(crate) use history_runtime::subscribe_history;

use crate::app_state::{
    AppState, CaptureRuntimeState, FavoritesRuntime, HistoryRuntime, ProviderRuntime,
    ScreenshotFavoritesRuntime, SelectionRuntime, SettingsRuntime,
};
use crate::application::favorites::{FavoriteChangeNotifier, FavoriteRepository};
use crate::application::hotkeys::HotkeyChangeNotifier;
use crate::application::hotkeys::HotkeyStore;
use crate::application::providers::ocr::OcrProviderConfiguration;
use crate::application::providers::HttpClient;
use crate::application::providers::{
    ProviderChangeNotifier, ProviderConfigStore, ProviderCredentialStore,
    TranslationPromptConfiguration,
};
use crate::application::result_window::ResultWindowRuntime;
use crate::application::settings::{SettingsChangeNotifier, SettingsStore};
use crate::infrastructure::events::EventBus;
use crate::infrastructure::http::ReqwestHttpClient;
use crate::infrastructure::storage::{
    Database, Keychain, SqliteConfigStore, SqliteFavoriteCapacityRepository,
    SqliteLibraryIndexRepository,
};
use crate::infrastructure::storage::{FilesystemOcrHistoryAssets, SqliteFavoriteRepository};
use crate::infrastructure::system::result_window::{
    TauriResultWindowNotifier, TauriResultWindowRuntimeHost,
};
use crate::{
    FavoriteCapacity, HotkeyConfiguration, HotkeyRuntime, LibraryIndex, SettingsConfiguration,
};

struct TauriSettingsChangeNotifier {
    app: AppHandle,
}

struct TauriFavoriteChangeNotifier {
    app: AppHandle,
}

impl FavoriteChangeNotifier for TauriFavoriteChangeNotifier {
    fn favorites_changed(&self) {
        if let Err(error) = self.app.emit("favorites-changed", ()) {
            log::warn!("Failed to emit favorites-changed: {}", error);
        }
    }
}

impl SettingsChangeNotifier for TauriSettingsChangeNotifier {
    fn settings_changed(&self) {
        if let Err(error) = self.app.emit("settings-changed", ()) {
            log::warn!("Failed to emit settings-changed: {}", error);
        }
    }
}

struct TauriHotkeyChangeNotifier {
    app: AppHandle,
}

impl HotkeyChangeNotifier for TauriHotkeyChangeNotifier {
    fn hotkeys_changed(&self) {
        if let Err(error) = self.app.emit("hotkeys-changed", ()) {
            log::warn!("Failed to emit hotkeys-changed: {}", error);
        }
    }
}

struct TauriProviderChangeNotifier {
    app: AppHandle,
}

impl ProviderChangeNotifier for TauriProviderChangeNotifier {
    fn providers_changed(&self) {
        if let Err(error) = self.app.emit("providers-changed", ()) {
            log::warn!("Failed to emit providers-changed: {}", error);
        }
    }
}

pub(crate) fn build_app_state(database_path: PathBuf, app: AppHandle) -> AppState {
    let asset_root = database_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("assets");
    let database = Arc::new(Database::open(database_path).expect("Failed to initialize database"));
    let config_store = Arc::new(SqliteConfigStore::new(database.clone()));
    let settings_store: Arc<dyn SettingsStore> = config_store.clone();
    let hotkey_store: Arc<dyn HotkeyStore> = config_store.clone();
    let provider_config_store: Arc<dyn ProviderConfigStore> = config_store.clone();
    let settings_configuration = Arc::new(SettingsConfiguration::with_change_notifier(
        settings_store,
        Arc::new(TauriSettingsChangeNotifier { app: app.clone() }),
    ));
    let hotkey_configuration = Arc::new(HotkeyConfiguration::new(hotkey_store));
    let hotkey_runtime = Arc::new(HotkeyRuntime::with_change_notifier(
        hotkey_configuration,
        Arc::new(TauriHotkeyChangeNotifier { app: app.clone() }),
    ));
    let keychain = Arc::new(Keychain::new());
    let provider_credential_store: Arc<dyn ProviderCredentialStore> = keychain.clone();
    let http_client: Arc<dyn HttpClient> = Arc::new(ReqwestHttpClient::new());
    let event_bus = Arc::new(EventBus::new());
    let provider_change_notifier: Arc<dyn ProviderChangeNotifier> =
        Arc::new(TauriProviderChangeNotifier { app: app.clone() });

    let history = build_history(
        database.clone(),
        asset_root.clone(),
        settings_configuration.clone(),
        app.clone(),
    );
    let favorite_repository: Arc<dyn FavoriteRepository> =
        Arc::new(SqliteFavoriteRepository::new(database.clone()));
    let favorite_capacity = Arc::new(FavoriteCapacity::new(
        Arc::new(SqliteFavoriteCapacityRepository::new(database.clone())),
        settings_configuration.clone(),
    ));
    let library_index = Arc::new(LibraryIndex::new(Arc::new(
        SqliteLibraryIndexRepository::new(database.clone()),
    )));
    let favorites = Arc::new(crate::application::Favorites::with_notifier_and_capacity(
        favorite_repository,
        Arc::new(FilesystemOcrHistoryAssets::new(
            asset_root.join("favorites"),
        )),
        Arc::new(TauriFavoriteChangeNotifier { app: app.clone() }),
        favorite_capacity.clone(),
    ));

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
        provider_change_notifier.clone(),
    );
    let prompt_strategies = Arc::new(
        TranslationPromptConfiguration::new(provider_config_store.clone())
            .with_change_notifier(provider_change_notifier.clone()),
    );
    let ocr_coordinator = build_ocr_coordinator(
        provider_config_store.clone(),
        http_client.clone(),
        event_bus.clone(),
        provider_change_notifier.clone(),
    );
    let ocr_configuration = Arc::new(
        OcrProviderConfiguration::new(ocr_coordinator.clone(), provider_credential_store.clone())
            .with_change_notifier(provider_change_notifier),
    );
    let ocr_history_replay = Arc::new(crate::application::OcrHistoryReplay::new(
        history.clone(),
        Arc::new(OcrCoordinatorHistoryRecognizer::new(
            ocr_coordinator.clone(),
        )),
    ));

    let capture_runtime = build_capture_runtime(app.clone(), ocr_coordinator.clone());
    let (screenshot_favorites, screenshot_favorite_capture) = build_screenshot_favorites(
        database.clone(),
        asset_root,
        capture_runtime.runtime.clone(),
        capture_runtime.output.clone(),
        favorite_capacity,
        app.clone(),
    );
    let selected_text_acquirer = build_selected_text_acquirer(app.clone());
    let result_window = Arc::new(ResultWindowRuntime::new(
        Arc::new(TauriResultWindowRuntimeHost::new(app.clone())),
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
            ocr_replay: ocr_history_replay,
            events: event_bus,
        }),
        favorites: Arc::new(FavoritesRuntime { favorites }),
        screenshot_favorites: Arc::new(ScreenshotFavoritesRuntime {
            favorites: screenshot_favorites,
            capture: screenshot_favorite_capture,
        }),
        library_index,
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

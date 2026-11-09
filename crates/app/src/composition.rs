use std::path::PathBuf;
use std::sync::Arc;

mod capture_runtime;
mod history_runtime;
mod provider_runtime;
mod screenshot_favorites_runtime;
mod selection_runtime;
mod tts_runtime;

#[cfg(target_os = "macos")]
use capture_runtime::build_capture_runtime;
use history_runtime::{build_history, OcrCoordinatorHistoryRecognizer};
use provider_runtime::{
    build_llm_introspection, build_llm_runtime, build_ocr_coordinator,
    build_provider_administration, build_provider_configuration, build_translation_coordinator,
    hydrate_provider_credentials,
};
use screenshot_favorites_runtime::build_screenshot_favorites;
use tts_runtime::build_tts_runtime;

pub(crate) use history_runtime::subscribe_history;

use crate::app_state::{
    AppState, CaptureRuntimeState, FavoritesRuntime, HistoryRuntime, LogsRuntime, ProviderRuntime,
    ScreenshotFavoritesRuntime, SelectionRuntime, SettingsRuntime,
};
use crate::application::favorites::FavoriteRepository;
use crate::application::hotkeys::HotkeyStore;
use crate::application::providers::ocr::OcrProviderConfiguration;
use crate::application::providers::HttpClient;
use crate::application::providers::{
    ProviderChangeNotifier, ProviderConfigStore, ProviderCredentialStore,
    TranslationPromptConfiguration,
};
use crate::application::result_window::ResultWindowRuntime;
use crate::application::settings::SettingsStore;
use crate::infrastructure::desktop_events::{DesktopEvent, DesktopEvents};
use crate::infrastructure::events::EventBus;
use crate::infrastructure::http::ReqwestHttpClient;
use crate::infrastructure::storage::{
    Database, SqliteAppLogRepository, SqliteConfigStore, SqliteCredentialStore,
    SqliteFavoriteCapacityRepository, SqliteLibraryIndexRepository,
};
use crate::infrastructure::storage::{FilesystemOcrHistoryAssets, SqliteFavoriteRepository};
use crate::infrastructure::system::NativeHotkeyRegistrar;
use crate::{
    FavoriteCapacity, HotkeyConfiguration, HotkeyRuntime, LibraryIndex, SettingsConfiguration,
};

pub struct CompositionOptions {
    pub identifier: String,
    pub version: String,
    pub ocr_resource_root: Option<PathBuf>,
    pub cache_dir: std::result::Result<PathBuf, String>,
    pub selected_text: Arc<crate::application::SelectedTextAcquirer>,
    #[cfg(not(any(
        target_os = "macos",
        all(target_os = "windows", feature = "native-app")
    )))]
    pub result_window: Arc<ResultWindowRuntime>,
    #[cfg(not(any(
        target_os = "macos",
        all(target_os = "windows", feature = "native-app")
    )))]
    pub capture: Box<
        dyn FnOnce(
            Arc<crate::application::providers::ocr::OcrCoordinator>,
            Arc<ResultWindowRuntime>,
            Arc<SettingsConfiguration>,
        ) -> capture_runtime::CaptureRuntimeParts,
    >,
}

pub fn build_app_state(
    database: Arc<Database>,
    database_path: PathBuf,
    options: CompositionOptions,
) -> AppState {
    let desktop_events = Arc::new(DesktopEvents::default());
    #[cfg(any(
        target_os = "macos",
        all(target_os = "windows", feature = "native-app")
    ))]
    let native_result_window =
        Arc::new(crate::infrastructure::system::result_window::NativeResultWindowHost::default());
    #[cfg(any(
        target_os = "macos",
        all(target_os = "windows", feature = "native-app")
    ))]
    let native_pinned_images = Arc::new(
        crate::infrastructure::system::pinned_window::NativePinnedImageRuntimeHost::default(),
    );
    let asset_root = database_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("assets");
    let config_store = Arc::new(SqliteConfigStore::new(database.clone()));
    let logs = Arc::new(LogsRuntime {
        repository: Arc::new(SqliteAppLogRepository::new(database.clone())),
    });
    let settings_store: Arc<dyn SettingsStore> = config_store.clone();
    let hotkey_store: Arc<dyn HotkeyStore> = config_store.clone();
    let provider_config_store: Arc<dyn ProviderConfigStore> = config_store.clone();
    let settings_configuration = Arc::new(SettingsConfiguration::with_change_notifier(
        settings_store,
        desktop_events.clone(),
    ));
    let hotkey_configuration = Arc::new(HotkeyConfiguration::new(hotkey_store));
    let hotkey_runtime = Arc::new(HotkeyRuntime::with_change_notifier(
        hotkey_configuration,
        Arc::new(NativeHotkeyRegistrar::new({
            let events = desktop_events.clone();
            move |category, action| {
                events.publish(DesktopEvent::HotkeyTriggered { category, action })
            }
        })),
        desktop_events.clone(),
    ));
    let credential_store = Arc::new(SqliteCredentialStore::new(database.clone()));
    let provider_credential_store: Arc<dyn ProviderCredentialStore> = credential_store.clone();
    let http_client: Arc<dyn HttpClient> = Arc::new(ReqwestHttpClient::with_settings(
        settings_configuration.clone(),
    ));
    let event_bus = Arc::new(EventBus::new());
    let provider_change_notifier: Arc<dyn ProviderChangeNotifier> = desktop_events.clone();

    let history = build_history(
        database.clone(),
        asset_root.clone(),
        settings_configuration.clone(),
        desktop_events.clone(),
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
        desktop_events.clone(),
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
        Arc::new(crate::infrastructure::ocr::LocalPaddleOcrEngine::new(
            options.ocr_resource_root.clone(),
            database_path
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."))
                .join("cache/ocr"),
        )),
        settings_configuration.clone(),
    );
    let ocr_configuration = Arc::new(
        OcrProviderConfiguration::new(ocr_coordinator.clone(), provider_credential_store.clone())
            .with_change_notifier(provider_change_notifier),
    );
    let provider_administration = build_provider_administration(
        translation_coordinator.clone(),
        ocr_coordinator.clone(),
        provider_configuration,
        ocr_configuration,
        llm_introspection,
        prompt_strategies,
    );
    let ocr_history_replay = Arc::new(crate::application::OcrHistoryReplay::new(
        history.clone(),
        Arc::new(OcrCoordinatorHistoryRecognizer::new(
            ocr_coordinator.clone(),
        )),
    ));
    let ocr_favorite_application = Arc::new(crate::application::OcrFavoriteApplication::new(
        history.clone(),
        favorites.clone(),
        ocr_coordinator.clone(),
    ));

    #[cfg(not(any(
        target_os = "macos",
        all(target_os = "windows", feature = "native-app")
    )))]
    let result_window = options.result_window;
    #[cfg(any(
        target_os = "macos",
        all(target_os = "windows", feature = "native-app")
    ))]
    let result_window = Arc::new(ResultWindowRuntime::new(
        native_result_window.clone(),
        native_result_window.clone(),
    ));
    #[cfg(any(
        target_os = "macos",
        all(target_os = "windows", feature = "native-app")
    ))]
    let result_session = Arc::new(
        crate::application::result_window::ResultWindowSession::new(
            settings_configuration.clone(),
            translation_coordinator.clone(),
            history.clone(),
            Arc::new(crate::infrastructure::system::result_window::NativeResultWindowPresentation),
        )
        .expect("Result window settings are available"),
    );

    #[cfg(target_os = "macos")]
    let capture_runtime = build_capture_runtime(
        options.identifier.clone(),
        ocr_coordinator.clone(),
        result_window.clone(),
        settings_configuration.clone(),
        native_pinned_images.clone(),
    );
    #[cfg(not(any(
        target_os = "macos",
        all(target_os = "windows", feature = "native-app")
    )))]
    let capture_runtime = (options.capture)(
        ocr_coordinator.clone(),
        result_window.clone(),
        settings_configuration.clone(),
    );
    #[cfg(all(target_os = "windows", feature = "native-app"))]
    let capture_runtime = capture_runtime::build_native_windows_runtime(
        database_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join("capture-history.json"),
        ocr_coordinator.clone(),
        result_window.clone(),
        settings_configuration.clone(),
        native_pinned_images.clone(),
    );
    let (screenshot_favorites, screenshot_favorite_capture) = build_screenshot_favorites(
        database.clone(),
        asset_root,
        capture_runtime.runtime.clone(),
        capture_runtime.output.clone(),
        favorite_capacity,
        desktop_events.clone(),
    );
    let selected_text_acquirer = options.selected_text;

    hydrate_provider_credentials_in_background(provider_administration.clone());

    let settings_application = Arc::new(crate::application::SettingsApplication::new(
        settings_configuration.clone(),
        Arc::new(crate::infrastructure::system::start_on_boot::NativeStartOnBoot::new()),
        history.clone(),
        logs.repository.clone(),
    ));

    let permissions = Arc::new(crate::application::RequiredPermissions::new(Arc::new(
        crate::infrastructure::system::required_permissions::SystemRequiredPermissions::new(
            options.identifier,
            {
                let events = desktop_events.clone();
                move || events.request(DesktopEvent::Restart).map_err(Into::into)
            },
        ),
    )));
    let capture = Arc::new(CaptureRuntimeState {
        sessions: capture_runtime.sessions,
        output: capture_runtime.output,
        runtime: capture_runtime.runtime,
        pinned_images: capture_runtime.pinned_images,
        cursor: Arc::new(crate::infrastructure::system::capture_cursor::EnigoCaptureCursorMover),
    });
    let selection = Arc::new(SelectionRuntime {
        translation: Arc::new(
            crate::application::selected_text::SelectionTranslation::new(
                settings_configuration.clone(),
                selected_text_acquirer,
                result_window.clone(),
            ),
        ),
    });
    let actions = crate::app_actions::AppActionDispatcher::new(
        permissions.clone(),
        capture.clone(),
        selection.clone(),
        result_window.clone(),
        desktop_events.clone(),
    );
    let mut state = AppState {
        actions,
        desktop_events: desktop_events.clone(),
        desktop_subscriptions: Vec::new(),
        updates: Arc::new(crate::application::updates::AppUpdates::new(
            Arc::new(
                crate::infrastructure::system::updates::SystemAppUpdateHost::new(
                    Arc::new(ReqwestHttpClient::with_settings(
                        settings_configuration.clone(),
                    )),
                    options.version.clone(),
                    options.cache_dir,
                ),
            ),
            options.version.clone(),
            std::env::consts::OS.into(),
            crate::infrastructure::system::updates::update_architecture().into(),
        )),
        permissions,
        settings: Arc::new(SettingsRuntime {
            configuration: settings_configuration.clone(),
            navigation: config_store.clone(),
            hotkeys: hotkey_runtime,
            application: settings_application,
        }),
        providers: Arc::new(ProviderRuntime {
            translation: translation_coordinator,
            file_ocr: Arc::new(crate::application::providers::ocr::FileOcr::new(
                Arc::new(crate::infrastructure::system::ocr::FilesystemImageReader),
                ocr_coordinator.clone(),
            )),
            ocr: ocr_coordinator,
            administration: provider_administration,
        }),
        capture,
        history: Arc::new(HistoryRuntime {
            history,
            ocr_replay: ocr_history_replay,
            events: event_bus,
        }),
        favorites: Arc::new(FavoritesRuntime {
            favorites,
            ocr_application: ocr_favorite_application,
        }),
        screenshot_favorites: Arc::new(ScreenshotFavoritesRuntime {
            favorites: screenshot_favorites,
            capture: screenshot_favorite_capture,
        }),
        library_index,
        selection,
        logs,
        tts: build_tts_runtime(settings_configuration.clone()),
        result_window,
        #[cfg(any(
            target_os = "macos",
            all(target_os = "windows", feature = "native-app")
        ))]
        result_session,
        #[cfg(any(
            target_os = "macos",
            all(target_os = "windows", feature = "native-app")
        ))]
        native_result_window,
        #[cfg(any(
            target_os = "macos",
            all(target_os = "windows", feature = "native-app")
        ))]
        native_pinned_images,
    };
    #[cfg(any(
        target_os = "macos",
        all(target_os = "windows", feature = "native-app")
    ))]
    state.native_result_window.bind(&state);
    #[cfg(target_os = "macos")]
    capture_runtime.capture_host.bind(&state);
    #[cfg(target_os = "macos")]
    capture_runtime.scrolling_host.bind(&state, {
        let events = desktop_events.clone();
        move |session_id, completed| {
            events.publish(DesktopEvent::ScrollingCaptureEnded {
                session_id: session_id.clone(),
                completed,
            })
        }
    });
    #[cfg(any(
        target_os = "macos",
        all(target_os = "windows", feature = "native-app")
    ))]
    state.native_pinned_images.bind(&state, {
        let events = desktop_events.clone();
        move || events.request(DesktopEvent::OpenSettings)
    });
    #[cfg(all(target_os = "windows", feature = "native-app"))]
    {
        capture_runtime.native_capture_host.bind(&state);
        capture_runtime.native_scrolling_host.bind(&state, {
            let events = desktop_events.clone();
            move |session_id, completed| {
                let session = session_id.clone();
                crate::infrastructure::ui_thread::dispatch(move || {
                    crate::infrastructure::system::capture_window::windows_scrolling_ended(
                        &session, completed,
                    );
                });
                events.publish(DesktopEvent::ScrollingCaptureEnded {
                    session_id: session_id.clone(),
                    completed,
                })
            }
        });
    }
    #[cfg(target_os = "macos")]
    bind_native_notifications(&mut state);
    #[cfg(all(target_os = "windows", feature = "native-app"))]
    {
        let result = Arc::downgrade(&state.native_result_window);
        let pins = Arc::downgrade(&state.native_pinned_images);
        let capture = Arc::downgrade(&capture_runtime.native_capture_host);
        let settings = Arc::downgrade(&state.settings.configuration);
        state
            .desktop_subscriptions
            .push(state.desktop_events.subscribe(move |event| {
                if matches!(event, DesktopEvent::SettingsChanged) {
                    if let Some(result) = result.upgrade() {
                        result.settings_changed();
                    }
                    if let Some(pins) = pins.upgrade() {
                        pins.settings_changed();
                    }
                    if let Some(capture) = capture.upgrade() {
                        capture.settings_changed();
                    }
                    if let Some(settings) = settings.upgrade().and_then(|s| s.snapshot().ok()) {
                        crate::infrastructure::system::scrolling_host_windows::settings_changed(
                            settings.general,
                        );
                    }
                }
            }));
    }
    state
}

fn hydrate_provider_credentials_in_background(
    administration: Arc<crate::application::providers::ProviderAdministration>,
) {
    crate::infrastructure::runtime::spawn_blocking(move || {
        hydrate_provider_credentials(administration);
    });
}

#[cfg(target_os = "macos")]
fn bind_native_notifications(state: &mut AppState) {
    let settings = Arc::downgrade(&state.settings.configuration);
    let result = Arc::downgrade(&state.native_result_window);
    let pins = Arc::downgrade(&state.native_pinned_images);
    let hotkeys = Arc::downgrade(&state.settings.hotkeys);
    state
        .desktop_subscriptions
        .push(state.desktop_events.subscribe(move |event| {
            match event {
                DesktopEvent::SettingsChanged => {
                    if let Some(result) = result.upgrade() {
                        result.settings_changed();
                    }
                    if let Some(pins) = pins.upgrade() {
                        pins.settings_changed();
                    }
                    if let Some(settings) = settings
                        .upgrade()
                        .and_then(|settings| settings.snapshot().ok())
                    {
                        crate::infrastructure::system::scrolling_host_macos::settings_changed(
                            settings.general,
                        );
                    }
                }
                DesktopEvent::HotkeysChanged => {
                    if let Some(snapshot) = hotkeys
                        .upgrade()
                        .and_then(|hotkeys| hotkeys.snapshot().ok())
                    {
                        crate::app_shell::refresh_menu_bar_shortcuts_native(snapshot);
                    }
                }
                _ => {}
            }
        }));
}

#[cfg(all(test, target_os = "macos"))]
mod native_state_tests {
    use super::*;

    #[test]
    fn constructs_state_and_releases_the_capture_runtime_without_a_framework_owner() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.db");
        let database = Arc::new(Database::open(&path).unwrap());
        let state = build_app_state(
            database,
            path,
            CompositionOptions {
                identifier: "com.snaplingo.native-state-test".into(),
                version: "0.1.23".into(),
                ocr_resource_root: None,
                cache_dir: Ok(directory.path().join("cache")),
                selected_text: selection_runtime::build_selected_text_acquirer(
                    "com.snaplingo.native-state-test".into(),
                ),
            },
        );
        let runtime = Arc::downgrade(&state.capture.runtime);
        let permissions = Arc::downgrade(&state.permissions);
        let selection = Arc::downgrade(&state.selection);
        let result = Arc::downgrade(&state.result_window);
        let notifications = state.desktop_events.clone();
        assert!(
            notifications.request(DesktopEvent::OpenSettings).is_err(),
            "Action routing must not count as a window controller"
        );
        let routes = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let routed = routes.clone();
        let _controller = notifications.connect_controller(move |event| {
            if *event
                == DesktopEvent::OpenSettingsAt(
                    crate::infrastructure::desktop_events::SettingsRoute::History,
                )
            {
                routed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
            Ok(())
        });
        notifications.publish(DesktopEvent::AppActionRequested(
            crate::app_actions::AppAction::OpenHistory,
        ));
        assert_eq!(routes.load(std::sync::atomic::Ordering::SeqCst), 1);
        notifications.publish(DesktopEvent::HotkeyTriggered {
            category: "unknown".into(),
            action: "unknown".into(),
        });
        assert_eq!(routes.load(std::sync::atomic::Ordering::SeqCst), 1);
        let settings = state.settings.configuration.clone();
        let received = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let events = received.clone();
        let _subscription = notifications.subscribe(move |event| {
            if *event == DesktopEvent::SettingsChanged {
                events.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
        });
        let mut general = settings.snapshot().unwrap().general;
        general.log_retention_days = 8;
        settings.update_general(general).unwrap();
        assert_eq!(received.load(std::sync::atomic::Ordering::SeqCst), 1);
        drop(state);
        assert!(
            runtime.upgrade().is_none(),
            "Native notifications kept the application capture runtime alive"
        );
        assert!(permissions.upgrade().is_none());
        assert!(selection.upgrade().is_none());
        assert!(result.upgrade().is_none());
        notifications.publish(DesktopEvent::AppActionRequested(
            crate::app_actions::AppAction::OpenHistory,
        ));
        assert_eq!(
            routes.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "Dropped application routing must stop delivery"
        );
        notifications.publish(DesktopEvent::SettingsChanged);
        assert_eq!(received.load(std::sync::atomic::Ordering::SeqCst), 2);
    }
}

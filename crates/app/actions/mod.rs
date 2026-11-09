pub(crate) mod shortcuts;
use std::sync::{Arc, Mutex};

use crate::app_state::{CaptureRuntimeState, SelectionRuntime};
use crate::infrastructure::desktop_events::{
    DesktopEvent, DesktopEvents, DesktopSubscription, SettingsRoute,
};
#[cfg(any(test, not(target_os = "macos")))]
use crate::AppState;
use snaplingo_core::application::result_window::{ResultWindowOpenRequest, ResultWindowRuntime};
use snaplingo_core::application::{RequiredPermissions, SystemPermission};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CaptureLaunchMode {
    Screenshot,
    ScreenshotCopy,
    ScreenshotTranslate,
    ScreenshotOcr,
    SilentScreenshotOcr,
}

impl CaptureLaunchMode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Screenshot => "screenshot",
            Self::ScreenshotCopy => "screenshot-copy",
            Self::ScreenshotTranslate => "screenshot-translate",
            Self::ScreenshotOcr => "screenshot-ocr",
            Self::SilentScreenshotOcr => "silent-screenshot-ocr",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AppAction {
    OpenCapture(CaptureLaunchMode),
    TranslateSelection,
    OpenTranslationWindow,
    RunFileOcr,
    PinClipboardImage,
    TogglePinnedImagesVisibility,
    SwitchPinnedImageGroup,
    OpenSettings,
    OpenHistory,
    OpenAbout,
    Quit,
}

#[cfg(any(test, not(target_os = "macos")))]
pub(crate) fn dispatch_app_action(state: &AppState, action: AppAction) {
    state.actions.dispatch(action);
}

struct ActionTargets {
    permissions: Arc<RequiredPermissions>,
    capture: Arc<CaptureRuntimeState>,
    selection: Arc<SelectionRuntime>,
    result_window: Arc<ResultWindowRuntime>,
    desktop_events: Arc<DesktopEvents>,
}

/// Application-owned routing survives replacement or shutdown of a frontend
/// notification bridge. Its event callback holds only a weak service owner.
pub(crate) struct AppActionDispatcher {
    _targets: Arc<ActionTargets>,
    subscription: Mutex<Option<DesktopSubscription>>,
}

impl AppActionDispatcher {
    pub(crate) fn new(
        permissions: Arc<RequiredPermissions>,
        capture: Arc<CaptureRuntimeState>,
        selection: Arc<SelectionRuntime>,
        result_window: Arc<ResultWindowRuntime>,
        desktop_events: Arc<DesktopEvents>,
    ) -> Self {
        let targets = Arc::new(ActionTargets {
            permissions,
            capture,
            selection,
            result_window,
            desktop_events: desktop_events.clone(),
        });
        let weak = Arc::downgrade(&targets);
        let subscription = desktop_events.subscribe(move |event| {
            let Some(targets) = weak.upgrade() else {
                return;
            };
            match event {
                DesktopEvent::AppActionRequested(action) => targets.dispatch(*action),
                DesktopEvent::HotkeyTriggered { category, action } => {
                    targets.trigger_hotkey_action(category, action)
                }
                _ => {}
            }
        });
        Self {
            _targets: targets,
            subscription: Mutex::new(Some(subscription)),
        }
    }

    #[cfg(any(test, not(target_os = "macos")))]
    pub(crate) fn dispatch(&self, action: AppAction) {
        self._targets.dispatch(action);
    }

    pub(crate) fn shutdown(&self) {
        self.subscription.lock().unwrap().take();
    }
}

impl ActionTargets {
    fn trigger_hotkey_action(&self, category: &str, action: &str) {
        let Some(binding) = crate::actions::shortcuts::hotkey_action_binding(category, action)
        else {
            log::warn!("Unknown hotkey action: {category}:{action}");
            return;
        };
        self.dispatch(binding.action);
    }

    fn dispatch(&self, action: AppAction) {
        if !action_permissions_granted(action, self.permissions.status()) {
            let permission = match action {
                AppAction::OpenCapture(_) => Some(SystemPermission::ScreenRecording),
                AppAction::TranslateSelection => Some(SystemPermission::Accessibility),
                _ => None,
            };
            if let Some(permission) = permission {
                if let Err(error) = self.permissions.request(permission) {
                    log::warn!("Failed to request permission: {}", error);
                }
            }
            request_window_action(
                &self.desktop_events,
                DesktopEvent::OpenSettingsAt(SettingsRoute::Permissions),
            );
            return;
        }

        match action {
            AppAction::OpenCapture(mode) => {
                let runtime = self.capture.runtime.clone();
                spawn_action("open capture window", async move {
                    runtime.open_capture_window_for_mode(mode.as_str()).await
                });
            }
            AppAction::TranslateSelection => {
                let translation = self.selection.translation.clone();
                spawn_action("open selection translation window", async move {
                    translation.open().await
                });
            }
            AppAction::OpenTranslationWindow => {
                dispatch_result_window_open(
                    self.result_window.clone(),
                    ResultWindowOpenRequest::show_translation(),
                );
            }
            AppAction::RunFileOcr => {
                dispatch_result_window_open(
                    self.result_window.clone(),
                    ResultWindowOpenRequest::file_ocr(),
                );
            }
            AppAction::PinClipboardImage => {
                let runtime = self.capture.runtime.clone();
                spawn_action("pin clipboard image", async move {
                    runtime.pin_from_shortcut().await
                });
            }
            AppAction::TogglePinnedImagesVisibility => {
                let pins = self.capture.pinned_images.clone();
                spawn_action("toggle pinned images", async move {
                    pins.toggle_visibility().await.map(|_| ())
                });
            }
            AppAction::SwitchPinnedImageGroup => {
                let pins = self.capture.pinned_images.clone();
                spawn_action("switch pinned image group", async move {
                    pins.switch_group().await.map(|_| ())
                });
            }
            AppAction::OpenSettings => {
                request_window_action(&self.desktop_events, DesktopEvent::OpenSettings)
            }
            AppAction::OpenHistory => request_window_action(
                &self.desktop_events,
                DesktopEvent::OpenSettingsAt(SettingsRoute::History),
            ),
            AppAction::OpenAbout => request_window_action(
                &self.desktop_events,
                DesktopEvent::OpenSettingsAt(SettingsRoute::About),
            ),
            AppAction::Quit => request_window_action(&self.desktop_events, DesktopEvent::Quit),
        }
    }
}

fn request_window_action(events: &DesktopEvents, event: DesktopEvent) {
    if let Err(error) = events.request(event) {
        log::error!("Failed to handle application window action: {error}");
    }
}

fn spawn_action(
    name: &'static str,
    action: impl std::future::Future<Output = crate::Result<()>> + Send + 'static,
) {
    crate::infrastructure::runtime::spawn(async move {
        if let Err(error) = action.await {
            log::error!("Failed to {name}: {error}");
        }
    });
}

fn action_permissions_granted(
    action: AppAction,
    status: snaplingo_core::application::RequiredPermissionsStatus,
) -> bool {
    match action {
        AppAction::OpenCapture(_) => status.screen_recording,
        AppAction::TranslateSelection => status.accessibility,
        _ => true,
    }
}

fn dispatch_result_window_open(
    runtime: std::sync::Arc<ResultWindowRuntime>,
    request: ResultWindowOpenRequest,
) {
    spawn_action("open result window", async move {
        open_result_window_request(&runtime, request)
            .await
            .map_err(Into::into)
    });
}

async fn open_result_window_request(
    runtime: &ResultWindowRuntime,
    request: ResultWindowOpenRequest,
) -> Result<(), String> {
    runtime
        .open(request)
        .await
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;

    use snaplingo_core::application::result_window::{
        ResultWindowMode, ResultWindowNotifierPort, ResultWindowOpenRequest, ResultWindowRequestId,
        ResultWindowRuntime, ResultWindowWindowPort,
    };

    use super::{
        action_permissions_granted, open_result_window_request, AppAction, CaptureLaunchMode,
    };

    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn settings_actions_and_permission_fallback_route_without_a_framework_owner() {
        use crate::infrastructure::desktop_events::{DesktopEvent, SettingsRoute};
        use snaplingo_core::application::{
            RequiredPermissions, RequiredPermissionsContext, RequiredPermissionsHost,
            RequiredPermissionsStatus, SelectedTextAcquirer, SelectionScheme, SystemPermission,
            SystemSelectionProvider,
        };
        use std::sync::Mutex;

        #[derive(Default)]
        struct Permissions(Mutex<Vec<SystemPermission>>);
        impl RequiredPermissionsHost for Permissions {
            fn status(&self) -> RequiredPermissionsStatus {
                RequiredPermissionsStatus {
                    screen_recording: false,
                    accessibility: false,
                }
            }
            fn context(&self) -> RequiredPermissionsContext {
                panic!("No permission repair is requested")
            }
            fn request(&self, permission: SystemPermission) -> crate::Result<()> {
                self.0.lock().unwrap().push(permission);
                Err("fixture denies the permission request".into())
            }
            fn reset(&self, _: SystemPermission) -> crate::Result<()> {
                panic!("No permission reset is requested")
            }
            fn restart(&self) -> crate::Result<()> {
                panic!("No restart is requested")
            }
        }
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("actions.db");
        let provider = Arc::new(
            crate::infrastructure::system::selection::macos::MacSelectionProvider::new(Some(
                "com.snaplingo.actions-test".into(),
            )),
        );
        let mut state = crate::composition::build_app_state(
            Arc::new(crate::infrastructure::storage::Database::open(&path).unwrap()),
            path,
            crate::composition::CompositionOptions {
                identifier: "com.snaplingo.actions-test".into(),
                version: "0.1.23".into(),
                ocr_resource_root: None,
                cache_dir: Ok(directory.path().join("cache")),
                selected_text: Arc::new(SelectedTextAcquirer::new(
                    SelectionScheme::new(provider.default_scheme()),
                    provider.methods(),
                    provider,
                )),
            },
        );
        let permissions = Arc::new(Permissions::default());
        state.permissions = Arc::new(RequiredPermissions::new(permissions.clone()));
        struct Opened(std::sync::atomic::AtomicUsize);
        #[async_trait]
        impl ResultWindowWindowPort for Opened {
            async fn show_or_create(&self) -> crate::Result<()> {
                self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            }
        }
        #[async_trait]
        impl ResultWindowNotifierPort for Opened {
            async fn notify_payload_ready(&self, _: ResultWindowRequestId) -> crate::Result<()> {
                Ok(())
            }
        }
        let opened = Arc::new(Opened(std::sync::atomic::AtomicUsize::new(0)));
        state.result_window = Arc::new(ResultWindowRuntime::new(opened.clone(), opened.clone()));
        state.actions = super::AppActionDispatcher::new(
            state.permissions.clone(),
            state.capture.clone(),
            state.selection.clone(),
            state.result_window.clone(),
            state.desktop_events.clone(),
        );
        let received = Arc::new(Mutex::new(Vec::new()));
        let output = received.clone();
        let _controller = state.desktop_events.connect_controller(move |event| {
            output.lock().unwrap().push(event.clone());
            Ok(())
        });
        for action in [
            AppAction::OpenSettings,
            AppAction::OpenHistory,
            AppAction::OpenAbout,
            AppAction::Quit,
            AppAction::OpenCapture(CaptureLaunchMode::Screenshot),
            AppAction::TranslateSelection,
        ] {
            super::dispatch_app_action(&state, action);
        }
        assert_eq!(
            *received.lock().unwrap(),
            vec![
                DesktopEvent::OpenSettings,
                DesktopEvent::OpenSettingsAt(SettingsRoute::History),
                DesktopEvent::OpenSettingsAt(SettingsRoute::About),
                DesktopEvent::Quit,
                DesktopEvent::OpenSettingsAt(SettingsRoute::Permissions),
                DesktopEvent::OpenSettingsAt(SettingsRoute::Permissions),
            ]
        );
        assert_eq!(
            *permissions.0.lock().unwrap(),
            vec![
                SystemPermission::ScreenRecording,
                SystemPermission::Accessibility
            ]
        );
        let before = received.lock().unwrap().len();
        state
            .desktop_events
            .publish(DesktopEvent::AppActionRequested(AppAction::OpenHistory));
        state.desktop_events.publish(DesktopEvent::HotkeyTriggered {
            category: snaplingo_core::domain::hotkey_config::TRANSLATION_CATEGORY.into(),
            action: snaplingo_core::domain::hotkey_config::SELECTION_TRANSLATE_ACTION.into(),
        });
        // The observer sees the published request and exactly one derived route.
        assert_eq!(
            received.lock().unwrap()[before..]
                .iter()
                .filter(|event| matches!(event, DesktopEvent::OpenSettingsAt(_)))
                .count(),
            2
        );
        for (index, event) in [
            DesktopEvent::AppActionRequested(AppAction::OpenTranslationWindow),
            DesktopEvent::HotkeyTriggered {
                category: snaplingo_core::domain::hotkey_config::TRANSLATION_CATEGORY.into(),
                action: snaplingo_core::domain::hotkey_config::SHOW_TRANSLATION_WINDOW_ACTION
                    .into(),
            },
        ]
        .into_iter()
        .enumerate()
        {
            state.desktop_events.publish(event);
            tokio::time::timeout(std::time::Duration::from_secs(3), async {
                while opened.0.load(std::sync::atomic::Ordering::SeqCst) < index + 1 {
                    tokio::task::yield_now().await;
                }
            })
            .await
            .unwrap();
            assert_eq!(
                opened.0.load(std::sync::atomic::Ordering::SeqCst),
                index + 1
            );
        }
        state.actions.shutdown();
        let before = received.lock().unwrap().len();
        state
            .desktop_events
            .publish(DesktopEvent::AppActionRequested(AppAction::OpenHistory));
        assert_eq!(received.lock().unwrap().len(), before + 1);
    }

    #[test]
    fn capture_launch_modes_keep_existing_ipc_strings() {
        assert_eq!(CaptureLaunchMode::Screenshot.as_str(), "screenshot");
        assert_eq!(
            CaptureLaunchMode::ScreenshotCopy.as_str(),
            "screenshot-copy"
        );
        assert_eq!(
            CaptureLaunchMode::ScreenshotTranslate.as_str(),
            "screenshot-translate"
        );
        assert_eq!(CaptureLaunchMode::ScreenshotOcr.as_str(), "screenshot-ocr");
        assert_eq!(
            CaptureLaunchMode::SilentScreenshotOcr.as_str(),
            "silent-screenshot-ocr"
        );
    }

    #[test]
    fn each_action_requires_only_the_permissions_it_uses() {
        use snaplingo_core::application::RequiredPermissionsStatus;
        let none = RequiredPermissionsStatus {
            screen_recording: false,
            accessibility: false,
        };
        let screen = RequiredPermissionsStatus {
            screen_recording: true,
            ..none
        };
        let accessibility = RequiredPermissionsStatus {
            accessibility: true,
            ..none
        };
        let capture = AppAction::OpenCapture(CaptureLaunchMode::Screenshot);
        assert!(!action_permissions_granted(capture, none));
        assert!(action_permissions_granted(capture, screen));
        assert!(!action_permissions_granted(capture, accessibility));
        assert!(action_permissions_granted(
            AppAction::TranslateSelection,
            accessibility
        ));
        assert!(!action_permissions_granted(
            AppAction::TranslateSelection,
            screen
        ));
        for action in [
            AppAction::OpenTranslationWindow,
            AppAction::RunFileOcr,
            AppAction::PinClipboardImage,
            AppAction::OpenSettings,
            AppAction::Quit,
        ] {
            assert!(action_permissions_granted(action, none));
        }
    }

    struct Window;

    #[async_trait]
    impl ResultWindowWindowPort for Window {
        async fn show_or_create(&self) -> crate::Result<()> {
            Ok(())
        }
    }

    struct Notifier;

    #[async_trait]
    impl ResultWindowNotifierPort for Notifier {
        async fn notify_payload_ready(&self, _: ResultWindowRequestId) -> crate::Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn result_window_actions_delegate_open_requests_to_the_runtime() {
        let runtime = ResultWindowRuntime::new(Arc::new(Window), Arc::new(Notifier));

        open_result_window_request(
            &runtime,
            ResultWindowOpenRequest::selection_translation("selection".into()),
        )
        .await
        .unwrap();

        let request_id = runtime.current_request_id().unwrap().unwrap();
        let payload = runtime.take_if_current(request_id).unwrap().unwrap();
        assert_eq!(payload.mode, ResultWindowMode::Translation);
        assert_eq!(payload.text, "selection");
        assert!(payload.auto_translate);
    }
}

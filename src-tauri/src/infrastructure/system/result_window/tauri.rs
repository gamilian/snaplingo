#[cfg(target_os = "macos")]
use objc2_app_kit::{
    NSApplicationActivationOptions, NSRunningApplication, NSScreenSaverWindowLevel, NSWindow,
    NSWindowAnimationBehavior, NSWindowCollectionBehavior, NSWindowStyleMask,
};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use super::backend::{result_window_definition, RESULT_WINDOW_LABEL};

pub fn show_or_create_result_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    show_or_create_result_window_with(&TauriResultWindowSystem { app })
}

pub(super) fn show_or_create_result_window_without_context(
    app: &AppHandle,
) -> Result<WebviewWindow, String> {
    show_or_create_result_window_on_system(&TauriResultWindowSystem { app })
}

trait ResultWindowSystem {
    type Window;

    fn existing_window(&self, label: &str) -> Option<Self::Window>;
    fn create_window(
        &self,
        definition: super::backend::ResultWindowDefinition,
    ) -> Result<Self::Window, String>;
    fn reveal_window(&self, window: &Self::Window) -> Result<(), String>;
}

fn show_or_create_result_window_with<S: ResultWindowSystem>(
    system: &S,
) -> Result<S::Window, String> {
    show_or_create_result_window_on_system(system)
        .map_err(|error| format!("Failed to show result window: {error}"))
}

fn show_or_create_result_window_on_system<S: ResultWindowSystem>(
    system: &S,
) -> Result<S::Window, String> {
    let window = system
        .existing_window(RESULT_WINDOW_LABEL)
        .map_or_else(|| system.create_window(result_window_definition()), Ok)?;
    system.reveal_window(&window)?;
    Ok(window)
}

struct TauriResultWindowSystem<'a> {
    app: &'a AppHandle,
}

impl ResultWindowSystem for TauriResultWindowSystem<'_> {
    type Window = WebviewWindow;

    fn existing_window(&self, label: &str) -> Option<Self::Window> {
        self.app.get_webview_window(label)
    }

    fn create_window(
        &self,
        definition: super::backend::ResultWindowDefinition,
    ) -> Result<Self::Window, String> {
        WebviewWindowBuilder::new(self.app, definition.label, WebviewUrl::App(definition.url))
            .title(definition.title)
            .inner_size(definition.inner_size.0, definition.inner_size.1)
            .position(definition.position.0, definition.position.1)
            .decorations(definition.decorations)
            .always_on_top(definition.always_on_top)
            .visible_on_all_workspaces(definition.visible_on_all_workspaces)
            .transparent(definition.transparent)
            .visible(definition.visible)
            .skip_taskbar(definition.skip_taskbar)
            .focused(definition.focused)
            .shadow(definition.shadow)
            .build()
            .map_err(|error| error.to_string())
    }

    fn reveal_window(&self, window: &Self::Window) -> Result<(), String> {
        reveal_result_window(window)
    }
}

fn reveal_result_window(window: &WebviewWindow) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        configure_result_window_for_current_space(window)?;
        window.show().map_err(|e| e.to_string())?;
        let ns_window = window.ns_window().map_err(|e| e.to_string())?;
        if ns_window.is_null() {
            return Err("Result window has no native NSWindow".to_string());
        }

        let ns_window: &NSWindow = unsafe { &*ns_window.cast() };
        if result_window_activates_application_on_reveal() {
            NSRunningApplication::currentApplication()
                .activateWithOptions(NSApplicationActivationOptions::empty());
        }
        if result_window_becomes_key_on_reveal() {
            ns_window.makeKeyAndOrderFront(None);
        } else {
            ns_window.orderFrontRegardless();
        }
        window.set_focus().map_err(|e| e.to_string())?;
    }

    #[cfg(not(target_os = "macos"))]
    {
        window.show().map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[cfg(target_os = "macos")]
fn configure_result_window_for_current_space(window: &WebviewWindow) -> Result<(), String> {
    let ns_window = window.ns_window().map_err(|e| e.to_string())?;
    if ns_window.is_null() {
        return Err("Result window has no native NSWindow".to_string());
    }

    let ns_window: &NSWindow = unsafe { &*ns_window.cast() };
    ns_window.setStyleMask(ns_window.styleMask() | NSWindowStyleMask::Borderless);
    ns_window.setCollectionBehavior(
        ns_window.collectionBehavior()
            | NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::FullScreenAuxiliary
            | NSWindowCollectionBehavior::Stationary
            | NSWindowCollectionBehavior::Transient
            | NSWindowCollectionBehavior::IgnoresCycle,
    );
    ns_window.setLevel(NSScreenSaverWindowLevel);
    ns_window.setCanHide(false);
    ns_window.setHidesOnDeactivate(false);
    if result_window_disables_window_animation() {
        ns_window.setAnimationBehavior(NSWindowAnimationBehavior::None);
    }

    Ok(())
}

#[cfg(target_os = "macos")]
fn result_window_disables_window_animation() -> bool {
    true
}

#[cfg(target_os = "macos")]
fn result_window_activates_application_on_reveal() -> bool {
    true
}

#[cfg(target_os = "macos")]
fn result_window_becomes_key_on_reveal() -> bool {
    true
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    #[test]
    fn result_window_disables_appkit_window_animation() {
        assert!(super::result_window_disables_window_animation());
    }

    #[test]
    fn result_window_takes_keyboard_focus_on_reveal() {
        assert!(super::result_window_activates_application_on_reveal());
        assert!(super::result_window_becomes_key_on_reveal());
    }
}

#[cfg(test)]
mod contract_tests {
    use std::cell::RefCell;

    use super::super::backend::ResultWindowDefinition;

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum FakeWindow {
        Existing,
        Created,
    }

    struct FakeResultWindowSystem {
        existing: Option<FakeWindow>,
        create_error: Option<&'static str>,
        reveal_error: Option<&'static str>,
        requested_labels: RefCell<Vec<String>>,
        created_definitions: RefCell<Vec<ResultWindowDefinition>>,
        revealed_windows: RefCell<Vec<FakeWindow>>,
    }

    impl FakeResultWindowSystem {
        fn missing() -> Self {
            Self {
                existing: None,
                create_error: None,
                reveal_error: None,
                requested_labels: RefCell::new(Vec::new()),
                created_definitions: RefCell::new(Vec::new()),
                revealed_windows: RefCell::new(Vec::new()),
            }
        }
    }

    impl super::ResultWindowSystem for FakeResultWindowSystem {
        type Window = FakeWindow;

        fn existing_window(&self, label: &str) -> Option<Self::Window> {
            self.requested_labels.borrow_mut().push(label.to_string());
            self.existing.clone()
        }

        fn create_window(
            &self,
            definition: ResultWindowDefinition,
        ) -> Result<Self::Window, String> {
            self.created_definitions.borrow_mut().push(definition);
            self.create_error
                .map_or(Ok(FakeWindow::Created), |error| Err(error.to_string()))
        }

        fn reveal_window(&self, window: &Self::Window) -> Result<(), String> {
            self.revealed_windows.borrow_mut().push(window.clone());
            self.reveal_error
                .map_or(Ok(()), |error| Err(error.to_string()))
        }
    }

    #[test]
    fn existing_result_window_is_revealed_without_creating_a_second_window() {
        let system = FakeResultWindowSystem {
            existing: Some(FakeWindow::Existing),
            ..FakeResultWindowSystem::missing()
        };

        assert_eq!(
            super::show_or_create_result_window_with(&system).unwrap(),
            FakeWindow::Existing
        );
        assert_eq!(
            *system.requested_labels.borrow(),
            vec!["capture-result".to_string()]
        );
        assert!(system.created_definitions.borrow().is_empty());
        assert_eq!(
            *system.revealed_windows.borrow(),
            vec![FakeWindow::Existing]
        );
    }

    #[test]
    fn missing_result_window_is_created_with_the_exact_contract_then_revealed() {
        let system = FakeResultWindowSystem::missing();

        assert_eq!(
            super::show_or_create_result_window_with(&system).unwrap(),
            FakeWindow::Created
        );
        let definition = system.created_definitions.borrow();
        assert_eq!(definition.len(), 1);
        assert_eq!(definition[0].label, "capture-result");
        assert_eq!(
            definition[0].url.to_string_lossy(),
            "index.html?window=capture-result"
        );
        assert_eq!(definition[0].inner_size, (660.0, 660.0));
        assert_eq!(definition[0].position, (120.0, 120.0));
        assert!(!definition[0].decorations);
        assert!(definition[0].always_on_top);
        assert!(definition[0].visible_on_all_workspaces);
        assert!(definition[0].transparent);
        assert!(!definition[0].visible);
        assert!(definition[0].skip_taskbar);
        assert!(!definition[0].focused);
        assert!(definition[0].shadow);
        assert_eq!(*system.revealed_windows.borrow(), vec![FakeWindow::Created]);
    }

    #[test]
    fn create_and_reveal_failures_include_show_operation_context() {
        let create_failure = FakeResultWindowSystem {
            create_error: Some("builder failed"),
            ..FakeResultWindowSystem::missing()
        };
        assert_eq!(
            super::show_or_create_result_window_with(&create_failure).unwrap_err(),
            "Failed to show result window: builder failed"
        );

        let reveal_failure = FakeResultWindowSystem {
            reveal_error: Some("show failed"),
            ..FakeResultWindowSystem::missing()
        };
        assert_eq!(
            super::show_or_create_result_window_with(&reveal_failure).unwrap_err(),
            "Failed to show result window: show failed"
        );
    }
}

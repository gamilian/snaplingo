//! Global-hotkey ownership, registration and delivery independent of the desktop shell.
use global_hotkey::{
    hotkey::HotKey as Shortcut, GlobalHotKeyEvent, GlobalHotKeyManager,
    HotKeyState as ShortcutState,
};
use snaplingo_core::error::{AppError, Result};
use std::cell::RefCell;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, LazyLock, Mutex, Once};
use std::thread::ThreadId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShortcutTriggerTiming {
    Pressed,
    Released,
}

type ShortcutCallback = Arc<dyn Fn(ShortcutState) + Send + Sync>;

struct RegisteredShortcut {
    shortcut: Shortcut,
    callback: ShortcutCallback,
    pressed: bool,
}

#[derive(Default)]
struct ShortcutRegistry {
    shortcuts: Mutex<HashMap<u32, RegisteredShortcut>>,
}

impl ShortcutRegistry {
    fn reserve(&self, shortcut: Shortcut, callback: ShortcutCallback) -> Result<()> {
        let mut shortcuts = self.shortcuts.lock().unwrap();
        if shortcuts.contains_key(&shortcut.id()) {
            return Err(AppError::Other(format!(
                "Shortcut is already registered: {shortcut}"
            )));
        }
        shortcuts.insert(
            shortcut.id(),
            RegisteredShortcut {
                shortcut,
                callback,
                pressed: false,
            },
        );
        Ok(())
    }

    fn contains(&self, id: u32) -> bool {
        self.shortcuts.lock().unwrap().contains_key(&id)
    }

    fn remove(&self, id: u32) {
        let removed = self.shortcuts.lock().unwrap().remove(&id);
        drop(removed);
    }

    fn handler_for(&self, event: GlobalHotKeyEvent) -> Option<ShortcutCallback> {
        let mut shortcuts = self.shortcuts.lock().unwrap();
        let shortcut = shortcuts.get_mut(&event.id)?;
        match event.state {
            ShortcutState::Pressed => shortcut.pressed = true,
            ShortcutState::Released if shortcut.pressed => shortcut.pressed = false,
            // A release from an old registration must not trigger a replacement callback.
            ShortcutState::Released => return None,
        }
        Some(shortcut.callback.clone())
    }

    fn clear(&self) -> Vec<Shortcut> {
        let shortcuts = std::mem::take(&mut *self.shortcuts.lock().unwrap());
        shortcuts
            .into_values()
            .map(|entry| entry.shortcut)
            .collect()
    }
}

static SHORTCUTS: LazyLock<ShortcutRegistry> = LazyLock::new(ShortcutRegistry::default);
static EVENT_HANDLER: Once = Once::new();
thread_local! {
    // macOS and Windows require creation, mutation and destruction on their event-loop thread.
    static SHORTCUT_MANAGER: RefCell<Option<GlobalHotKeyManager>> = const { RefCell::new(None) };
}

pub type UiTask = Box<dyn FnOnce() + Send>;
type Dispatcher = Arc<dyn Fn(UiTask) -> Result<()> + Send + Sync>;
type EventFilter = Arc<dyn Fn(&Shortcut, ShortcutState) -> bool + Send + Sync>;

struct Runtime {
    thread: ThreadId,
    dispatch: Dispatcher,
    filter: EventFilter,
}
static RUNTIME: Mutex<Option<Arc<Runtime>>> = Mutex::new(None);

fn dispatch_event(event: GlobalHotKeyEvent) {
    let Ok(runtime) = runtime() else { return };
    let deliver = move || {
        if let Some(handler) = SHORTCUTS.handler_for(event) {
            handler(event.state);
        }
    };
    // Windows reports key releases from its polling worker. Recording focus
    // checks and destination callbacks belong to the UI owner.
    if std::thread::current().id() == runtime.thread {
        deliver();
    } else if let Err(error) = (runtime.dispatch)(Box::new(deliver)) {
        log::warn!("Failed to deliver native shortcut event: {error}");
    }
}

#[cfg(test)]
pub(crate) fn test_dispatch_event(event: GlobalHotKeyEvent) {
    dispatch_event(event);
}

/// Call on the platform event-loop thread; the shell supplies only event recording.
pub fn initialize(
    dispatch: impl Fn(UiTask) -> Result<()> + Send + Sync + 'static,
    filter: impl Fn(&Shortcut, ShortcutState) -> bool + Send + Sync + 'static,
) -> Result<()> {
    if RUNTIME.lock().unwrap().is_some() {
        return Ok(());
    }
    EVENT_HANDLER.call_once(|| GlobalHotKeyEvent::set_event_handler(Some(dispatch_event)));
    let manager = GlobalHotKeyManager::new().map_err(|error| {
        AppError::Other(format!("Failed to initialize native shortcuts: {error}"))
    })?;
    SHORTCUT_MANAGER.with_borrow_mut(|current| *current = Some(manager));
    *RUNTIME.lock().unwrap() = Some(Arc::new(Runtime {
        thread: std::thread::current().id(),
        dispatch: Arc::new(dispatch),
        filter: Arc::new(filter),
    }));
    Ok(())
}

pub fn shutdown() -> Result<()> {
    if RUNTIME.lock().unwrap().is_none() {
        return Ok(());
    }
    let result = on_event_thread(|| {
        let shortcuts = SHORTCUTS.clear();
        SHORTCUT_MANAGER
            .with_borrow_mut(Option::take)
            .map(|manager| {
                manager.unregister_all(&shortcuts).map_err(|error| {
                    AppError::Other(format!("Failed to release native shortcuts: {error}"))
                })
            })
            .unwrap_or(Ok(()))
    });
    let stopped = RUNTIME.lock().unwrap().take();
    drop(stopped);
    result
}

fn runtime() -> Result<Arc<Runtime>> {
    RUNTIME
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| AppError::Other("Native shortcuts are not initialized".into()))
}

fn on_thread<T: Send + 'static>(
    action: impl FnOnce(&GlobalHotKeyManager) -> Result<T> + Send + 'static,
) -> Result<T> {
    on_event_thread(move || {
        SHORTCUT_MANAGER.with(|current| {
            let current = current.borrow();
            let manager = current
                .as_ref()
                .ok_or_else(|| AppError::Other("Native shortcuts are shut down".into()))?;
            action(manager)
        })
    })
}

fn on_event_thread<T: Send + 'static>(
    task: impl FnOnce() -> Result<T> + Send + 'static,
) -> Result<T> {
    let runtime = runtime()?;
    if std::thread::current().id() == runtime.thread {
        return task();
    }
    let (sender, receiver) = std::sync::mpsc::channel();
    (runtime.dispatch)(Box::new(move || {
        let _ = sender.send(task());
    }))?;
    receiver.recv().map_err(|error| {
        AppError::Other(format!(
            "Native shortcut operation was interrupted: {error}"
        ))
    })?
}

pub fn register<F>(accelerator: &str, timing: ShortcutTriggerTiming, handler: F) -> Result<()>
where
    F: Fn() + Send + Sync + 'static,
{
    let shortcut = Shortcut::from_str(accelerator)
        .map_err(|e| AppError::Other(format!("Invalid shortcut: {}", e)))?;
    let normalized_accelerator = shortcut.to_string();
    let shortcut_label = normalized_accelerator.clone();

    let filter = runtime()?.filter.clone();
    let callback: ShortcutCallback = Arc::new(move |state| {
        log::info!("Global shortcut event: {} ({:?})", shortcut_label, state);
        if filter(&shortcut, state) {
            return;
        }
        if should_trigger_shortcut(timing, state) {
            handler();
        }
    });
    on_thread(move |manager| {
        SHORTCUTS.reserve(shortcut, callback)?;
        if let Err(error) = manager.register(shortcut) {
            SHORTCUTS.remove(shortcut.id());
            return Err(AppError::Other(format!(
                "Failed to register shortcut: {error}"
            )));
        }
        Ok(())
    })?;

    log::info!(
        "Registered global shortcut: {} ({})",
        accelerator,
        normalized_accelerator
    );
    Ok(())
}

fn should_trigger_shortcut(timing: ShortcutTriggerTiming, state: ShortcutState) -> bool {
    matches!(
        (timing, state),
        (ShortcutTriggerTiming::Pressed, ShortcutState::Pressed)
            | (ShortcutTriggerTiming::Released, ShortcutState::Released)
    )
}

/// Unregister a global shortcut
pub fn unregister(accelerator: &str) -> Result<()> {
    let shortcut = Shortcut::from_str(accelerator)
        .map_err(|e| AppError::Other(format!("Invalid shortcut: {}", e)))?;

    on_thread(move |manager| {
        if SHORTCUTS.contains(shortcut.id()) {
            manager.unregister(shortcut).map_err(|error| {
                AppError::Other(format!("Failed to unregister shortcut: {error}"))
            })?;
            SHORTCUTS.remove(shortcut.id());
        }
        Ok(())
    })?;

    log::info!("Unregistered global shortcut: {}", accelerator);
    Ok(())
}

/// Check if a shortcut is registered
pub fn is_registered(accelerator: &str) -> Result<bool> {
    let shortcut = Shortcut::from_str(accelerator)
        .map_err(|e| AppError::Other(format!("Invalid shortcut: {}", e)))?;

    runtime()?;
    Ok(SHORTCUTS.contains(shortcut.id()))
}

#[cfg(test)]
mod tests {
    use super::{should_trigger_shortcut, ShortcutRegistry, ShortcutTriggerTiming};
    use global_hotkey::GlobalHotKeyEvent;
    use global_hotkey::{hotkey::HotKey as Shortcut, HotKeyState as ShortcutState};
    use std::sync::{Arc, Mutex};

    #[test]
    fn pressed_timing_only_triggers_on_pressed_events() {
        assert!(should_trigger_shortcut(
            ShortcutTriggerTiming::Pressed,
            ShortcutState::Pressed
        ));
        assert!(!should_trigger_shortcut(
            ShortcutTriggerTiming::Pressed,
            ShortcutState::Released
        ));
    }

    #[test]
    fn released_timing_only_triggers_on_released_events() {
        assert!(should_trigger_shortcut(
            ShortcutTriggerTiming::Released,
            ShortcutState::Released
        ));
        assert!(!should_trigger_shortcut(
            ShortcutTriggerTiming::Released,
            ShortcutState::Pressed
        ));
    }

    #[test]
    fn native_registry_routes_both_edges_and_ignores_removed_and_stale_releases() {
        let registry = ShortcutRegistry::default();
        let shortcut = "Super+F20".parse::<Shortcut>().unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let output = events.clone();
        registry
            .reserve(
                shortcut,
                Arc::new(move |state| output.lock().unwrap().push(state)),
            )
            .unwrap();
        let event = |state| GlobalHotKeyEvent {
            id: shortcut.id(),
            state,
        };
        assert!(registry
            .handler_for(event(ShortcutState::Released))
            .is_none());
        for state in [ShortcutState::Pressed, ShortcutState::Released] {
            registry.handler_for(event(state)).unwrap()(state);
        }
        assert_eq!(
            *events.lock().unwrap(),
            vec![ShortcutState::Pressed, ShortcutState::Released]
        );
        registry.handler_for(event(ShortcutState::Pressed)).unwrap()(ShortcutState::Pressed);
        registry.remove(shortcut.id());
        assert!(registry
            .handler_for(event(ShortcutState::Released))
            .is_none());
        registry
            .reserve(
                shortcut,
                Arc::new(|_| panic!("An old release reached the replacement")),
            )
            .unwrap();
        assert!(registry
            .handler_for(event(ShortcutState::Released))
            .is_none());
    }

    #[test]
    fn native_callback_can_unregister_itself_without_holding_the_registry_lock() {
        let registry = Arc::new(ShortcutRegistry::default());
        let shortcut = "F20".parse::<Shortcut>().unwrap();
        let runtime = Arc::downgrade(&registry);
        registry
            .reserve(
                shortcut,
                Arc::new(move |_| runtime.upgrade().unwrap().remove(shortcut.id())),
            )
            .unwrap();
        registry
            .handler_for(GlobalHotKeyEvent {
                id: shortcut.id(),
                state: ShortcutState::Pressed,
            })
            .unwrap()(ShortcutState::Pressed);
        assert!(!registry.contains(shortcut.id()));
    }

    #[test]
    fn duplicate_registration_preserves_callback_and_clear_releases_all_keys() {
        let registry = ShortcutRegistry::default();
        let shortcut = "F20".parse::<Shortcut>().unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let output = events.clone();
        registry
            .reserve(
                shortcut,
                Arc::new(move |state| output.lock().unwrap().push(state)),
            )
            .unwrap();
        assert!(registry
            .reserve(
                shortcut,
                Arc::new(|_| panic!("Duplicate replaced the callback"))
            )
            .is_err());
        registry
            .handler_for(GlobalHotKeyEvent {
                id: shortcut.id(),
                state: ShortcutState::Pressed,
            })
            .unwrap()(ShortcutState::Pressed);
        assert_eq!(*events.lock().unwrap(), vec![ShortcutState::Pressed]);
        assert_eq!(registry.clear(), vec![shortcut]);
        assert!(!registry.contains(shortcut.id()));
        assert!(registry
            .handler_for(GlobalHotKeyEvent {
                id: shortcut.id(),
                state: ShortcutState::Released
            })
            .is_none());
    }
}

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, Weak};

use crate::domain::capture::CaptureSessionId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SettingsRoute {
    About,
    History,
    Permissions,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum DesktopEvent {
    SettingsChanged,
    HotkeysChanged,
    ProvidersChanged,
    HistoryChanged,
    FavoritesChanged,
    ScreenshotFavoritesChanged,
    OpenSettings,
    OpenSettingsAt(SettingsRoute),
    Quit,
    AppActionRequested(crate::app_actions::AppAction),
    Restart,
    HotkeyTriggered {
        category: String,
        action: String,
    },
    HotkeyRecorded {
        window_label: String,
        recording_id: String,
        hotkey: String,
    },
    ScrollingCaptureEnded {
        session_id: CaptureSessionId,
        completed: bool,
    },
}

struct Listener {
    active: AtomicBool,
    controller: bool,
    receive: Box<dyn Fn(&DesktopEvent) -> Result<(), String> + Send + Sync>,
}

/// The publisher holds only weak subscriptions. UI/state owners control their
/// lifetime; delivering an event never holds the registry lock.
#[derive(Default)]
pub(crate) struct DesktopEvents {
    next_id: AtomicU64,
    listeners: Mutex<BTreeMap<u64, Weak<Listener>>>,
}

pub(crate) struct DesktopSubscription {
    id: u64,
    publisher: Weak<DesktopEvents>,
    listener: Arc<Listener>,
}

impl DesktopEvents {
    pub(crate) fn subscribe(
        self: &Arc<Self>,
        receive: impl Fn(&DesktopEvent) + Send + Sync + 'static,
    ) -> DesktopSubscription {
        self.listen(false, move |event| {
            receive(event);
            Ok(())
        })
    }

    pub(crate) fn connect_controller(
        self: &Arc<Self>,
        receive: impl Fn(&DesktopEvent) -> Result<(), String> + Send + Sync + 'static,
    ) -> DesktopSubscription {
        self.listen(true, receive)
    }

    fn listen(
        self: &Arc<Self>,
        controller: bool,
        receive: impl Fn(&DesktopEvent) -> Result<(), String> + Send + Sync + 'static,
    ) -> DesktopSubscription {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let listener = Arc::new(Listener {
            active: AtomicBool::new(true),
            controller,
            receive: Box::new(receive),
        });
        self.listeners
            .lock()
            .unwrap()
            .insert(id, Arc::downgrade(&listener));
        DesktopSubscription {
            id,
            publisher: Arc::downgrade(self),
            listener,
        }
    }

    pub(crate) fn publish(&self, event: DesktopEvent) {
        for listener in self.listeners() {
            if listener.active.load(Ordering::Acquire) {
                if let Err(error) = (listener.receive)(&event) {
                    log::warn!("Failed to handle desktop event {event:?}: {error}");
                }
            }
        }
    }

    pub(crate) fn request(&self, event: DesktopEvent) -> Result<(), String> {
        let mut handled = false;
        for listener in self.listeners() {
            if listener.controller && listener.active.load(Ordering::Acquire) {
                handled = true;
                (listener.receive)(&event)?;
            }
        }
        if handled {
            Ok(())
        } else {
            Err("Desktop window controller is not available".into())
        }
    }

    fn listeners(&self) -> Vec<Arc<Listener>> {
        let mut registry = self.listeners.lock().unwrap();
        registry.retain(|_, listener| listener.strong_count() != 0);
        registry
            .values()
            .filter_map(Weak::upgrade)
            .collect::<Vec<_>>()
    }
}

impl Drop for DesktopSubscription {
    fn drop(&mut self) {
        self.listener.active.store(false, Ordering::Release);
        if let Some(publisher) = self.publisher.upgrade() {
            publisher.listeners.lock().unwrap().remove(&self.id);
        }
    }
}

impl crate::application::settings::SettingsChangeNotifier for DesktopEvents {
    fn settings_changed(&self) {
        self.publish(DesktopEvent::SettingsChanged);
    }
}
impl crate::application::hotkeys::HotkeyChangeNotifier for DesktopEvents {
    fn hotkeys_changed(&self) {
        self.publish(DesktopEvent::HotkeysChanged);
    }
}
impl crate::application::providers::ProviderChangeNotifier for DesktopEvents {
    fn providers_changed(&self) {
        self.publish(DesktopEvent::ProvidersChanged);
    }
}
impl crate::application::history::HistoryChangeNotifier for DesktopEvents {
    fn history_changed(&self) {
        self.publish(DesktopEvent::HistoryChanged);
    }
}
impl crate::application::favorites::FavoriteChangeNotifier for DesktopEvents {
    fn favorites_changed(&self) {
        self.publish(DesktopEvent::FavoritesChanged);
    }
}
impl crate::application::screenshot_favorites::ScreenshotFavoriteChangeNotifier for DesktopEvents {
    fn screenshot_favorites_changed(&self) {
        self.publish(DesktopEvent::ScreenshotFavoritesChanged);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_requests_preserve_errors_and_require_a_live_controller() {
        let events = Arc::new(DesktopEvents::default());
        let _observer = events.subscribe(|_| {});
        assert!(events.request(DesktopEvent::OpenSettings).is_err());
        let controller = events.connect_controller(|_| Err("window creation failed".into()));
        assert_eq!(
            events.request(DesktopEvent::OpenSettings),
            Err("window creation failed".into())
        );
        drop(controller);
        assert!(events.request(DesktopEvent::OpenSettings).is_err());
    }

    #[test]
    fn workers_deliver_changes_and_dropping_the_subscription_releases_its_owner() {
        let events = Arc::new(DesktopEvents::default());
        let received = Arc::new(Mutex::new(Vec::new()));
        let owner = Arc::new(());
        let lifetime = Arc::downgrade(&owner);
        let output = received.clone();
        let subscription = events.subscribe(move |event| {
            let _ = &owner;
            output.lock().unwrap().push(event.clone());
        });
        let worker = events.clone();
        std::thread::spawn(move || worker.publish(DesktopEvent::SettingsChanged))
            .join()
            .unwrap();
        assert_eq!(
            *received.lock().unwrap(),
            vec![DesktopEvent::SettingsChanged]
        );
        drop(subscription);
        assert!(lifetime.upgrade().is_none());
        events.publish(DesktopEvent::ProvidersChanged);
        assert_eq!(received.lock().unwrap().len(), 1);
    }

    #[test]
    fn callbacks_can_publish_and_unsubscribe_without_holding_the_registry_lock() {
        let events = Arc::new(DesktopEvents::default());
        let received = Arc::new(Mutex::new(Vec::new()));
        let output = received.clone();
        let publisher = Arc::downgrade(&events);
        let _subscription = events.subscribe(move |event| {
            output.lock().unwrap().push(event.clone());
            if *event == DesktopEvent::SettingsChanged {
                publisher
                    .upgrade()
                    .unwrap()
                    .publish(DesktopEvent::HotkeysChanged);
            }
        });
        let slot = Arc::new(Mutex::new(None));
        let weak_slot = Arc::downgrade(&slot);
        *slot.lock().unwrap() = Some(events.subscribe(move |_| {
            weak_slot.upgrade().unwrap().lock().unwrap().take();
        }));
        events.publish(DesktopEvent::SettingsChanged);
        events.publish(DesktopEvent::ProvidersChanged);
        assert!(slot.lock().unwrap().is_none());
        assert_eq!(
            *received.lock().unwrap(),
            vec![
                DesktopEvent::SettingsChanged,
                DesktopEvent::HotkeysChanged,
                DesktopEvent::ProvidersChanged
            ]
        );
    }
}

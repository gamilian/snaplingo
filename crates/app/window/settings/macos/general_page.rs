//! Binds the General page to Rust services and the AppKit main queue. The full
//! Settings shell can mount this page without an IPC or WebView owner.
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use objc2::{rc::Retained, MainThreadMarker};

use super::{GeneralAction, GeneralController, GeneralServices, GeneralView};
use crate::infrastructure::desktop_events::{DesktopEvent, DesktopEvents, DesktopSubscription};
use crate::infrastructure::{runtime, ui_thread};

static NEXT_PAGE: AtomicU64 = AtomicU64::new(1);
thread_local! {
    static PAGES: RefCell<BTreeMap<u64, Weak<GeneralPage>>> = const { RefCell::new(BTreeMap::new()) };
}

pub(crate) struct GeneralPage {
    id: u64,
    view: Retained<GeneralView>,
    controller: Arc<GeneralController>,
    subscription: RefCell<Option<DesktopSubscription>>,
    disposed: Cell<bool>,
}

impl GeneralPage {
    pub(crate) fn from_state(
        mtm: MainThreadMarker,
        state: &crate::AppState,
        version: String,
    ) -> Result<Rc<Self>, String> {
        let id = NEXT_PAGE.fetch_add(1, Ordering::Relaxed);
        let controller = Arc::new(GeneralController::new(GeneralServices {
            configuration: state.settings.configuration.clone(),
            settings: state.settings.application.clone(),
            history: state.history.history.clone(),
            permissions: state.permissions.clone(),
            updates: state.updates.clone(),
            version,
            open_url: Arc::new(crate::infrastructure::system::external_open::open_url),
            open_notices: Arc::new(move || {
                ui_thread::dispatch(move || {
                    if let Some(page) = lookup(id) {
                        if let Some(parent) = page.view.window() {
                            match page.controller.projection() {
                                Ok(projection) => {
                                    super::notices::show(&parent, &projection.general)
                                }
                                Err(error) => {
                                    log::warn!("Failed to open component notices: {error}")
                                }
                            }
                        }
                    }
                });
                Ok(())
            }),
        }));
        Self::mount(mtm, id, controller, &state.desktop_events)
    }

    pub(crate) fn new(
        mtm: MainThreadMarker,
        controller: Arc<GeneralController>,
        events: &Arc<DesktopEvents>,
    ) -> Result<Rc<Self>, String> {
        Self::mount(
            mtm,
            NEXT_PAGE.fetch_add(1, Ordering::Relaxed),
            controller,
            events,
        )
    }

    fn mount(
        mtm: MainThreadMarker,
        id: u64,
        controller: Arc<GeneralController>,
        events: &Arc<DesktopEvents>,
    ) -> Result<Rc<Self>, String> {
        let dispatch_controller = Arc::downgrade(&controller);
        let view = GeneralView::new(
            mtm,
            controller.projection()?,
            Rc::new(move |action| {
                let dispatch_controller = dispatch_controller
                    .upgrade()
                    .ok_or("Native General page has been disposed")?;
                if let GeneralAction::Edit(field, value) = action {
                    return dispatch_controller.edit(field, value).map(Some);
                }
                let controller = dispatch_controller.clone();
                runtime::spawn(async move {
                    // The controller records the error in its projection, including
                    // failed permission repairs and failed update downloads.
                    let _ = controller.perform(action, move |_| refresh_later(id)).await;
                });
                Ok(None)
            }),
        );
        let subscription = events.subscribe(move |event| {
            if *event == DesktopEvent::SettingsChanged {
                refresh_later(id);
            }
        });
        let page = Rc::new(Self {
            id,
            view,
            controller,
            subscription: RefCell::new(Some(subscription)),
            disposed: Cell::new(false),
        });
        PAGES.with_borrow_mut(|pages| {
            pages.insert(id, Rc::downgrade(&page));
        });
        page.ensure_checked();
        Ok(page)
    }

    pub(crate) fn view(&self) -> &GeneralView {
        &self.view
    }

    /// Refresh on focus as well as SettingsChanged: OS grants can change while
    /// System Settings is in front and do not publish an application event.
    pub(crate) fn refresh(&self) -> Result<(), String> {
        if self.disposed.get() {
            return Ok(());
        }
        self.view.refresh(self.controller.projection()?);
        Ok(())
    }

    pub(crate) fn ensure_checked(&self) {
        if self.disposed.get() {
            return;
        }
        let controller = self.controller.clone();
        let id = self.id;
        runtime::spawn(async move {
            let _ = controller.ensure_checked(move |_| refresh_later(id)).await;
        });
    }

    pub(crate) fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        if let Some(parent) = self.view.window() {
            super::notices::close_for(&parent);
        }
        PAGES.with_borrow_mut(|pages| {
            pages.remove(&self.id);
        });
        self.subscription.borrow_mut().take();
        self.controller.invalidate_pending_updates();
    }
}

fn lookup(id: u64) -> Option<Rc<GeneralPage>> {
    PAGES.with_borrow(|pages| pages.get(&id).and_then(Weak::upgrade))
}

fn refresh_later(id: u64) {
    ui_thread::dispatch(move || {
        if let Some(page) = lookup(id) {
            // Read when delivered, so an earlier worker snapshot cannot undo a
            // setting edited on the main thread while this callback was queued.
            if let Err(error) = page.refresh() {
                log::warn!("Failed to refresh native General settings: {error}");
            }
        }
    });
}

impl Drop for GeneralPage {
    fn drop(&mut self) {
        self.dispose();
    }
}

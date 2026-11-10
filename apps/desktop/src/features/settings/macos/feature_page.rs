//! Live OCR and Translation preferences call Rust application services directly.
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use objc2::{rc::Retained, MainThreadMarker};
use objc2_app_kit::NSView;

use super::feature::{FeatureField, FeatureKind, FeatureProjection, FeatureView};
use super::hotkeys::{HotkeyView, HotkeyViewServices};
use crate::integrations::desktop_events::DesktopEvent;
use crate::integrations::desktop_events::DesktopSubscription;
use crate::integrations::ui_thread;
use snaplingo_core::hotkeys::HotkeyRecording;
use snaplingo_core::settings::SettingsConfiguration;

static NEXT_PAGE: AtomicU64 = AtomicU64::new(1);
thread_local! { static PAGES: RefCell<BTreeMap<u64,Weak<FeaturePage>>> = const { RefCell::new(BTreeMap::new()) }; }

pub(crate) struct FeaturePage {
    id: u64,
    kind: FeatureKind,
    view: Retained<FeatureView>,
    hotkeys: Retained<HotkeyView>,
    configuration: Arc<SettingsConfiguration>,
    subscription: RefCell<Option<DesktopSubscription>>,
    disposed: Cell<bool>,
}

impl FeaturePage {
    pub(crate) fn new(
        mtm: MainThreadMarker,
        kind: FeatureKind,
        state: &crate::features::settings::Services,
        recording: HotkeyRecording,
    ) -> Result<Rc<Self>, String> {
        let settings = state.application.clone();
        let configuration = state.configuration.clone();
        let initial = projection(&configuration)?;
        let hotkeys = HotkeyView::new(
            mtm,
            match kind {
                FeatureKind::Ocr => "ocr",
                FeatureKind::Translation => "translation",
            },
            HotkeyViewServices {
                runtime: state.hotkeys.clone(),
                recording,
                events: state.events.clone(),
                window_label: "settings".into(),
            },
            initial.general.clone(),
        )?;
        let edits = Arc::downgrade(&settings);
        let reads = Arc::downgrade(&configuration);
        let view = FeatureView::new(
            mtm,
            kind,
            initial,
            hotkeys.clone(),
            Rc::new(move |field, value| {
                let settings = edits
                    .upgrade()
                    .ok_or("Native feature preferences have been disposed")?;
                match field {
                    FeatureField::Ocr(field) => settings.edit_ocr(field, value),
                    FeatureField::Translation(field) => settings.edit_translation(field, value),
                }
                .map_err(|error| error.to_string())?;
                let configuration = reads
                    .upgrade()
                    .ok_or("Native feature preferences have been disposed")?;
                projection(&configuration)
            }),
        );
        let id = NEXT_PAGE.fetch_add(1, Ordering::Relaxed);
        let subscription = state.events.subscribe(move |event| {
            if *event == DesktopEvent::SettingsChanged {
                ui_thread::dispatch(move || {
                    if let Some(page) =
                        PAGES.with_borrow(|pages| pages.get(&id).and_then(Weak::upgrade))
                    {
                        if let Err(error) = page.refresh() {
                            log::warn!("Failed to refresh native feature preferences: {error}");
                        }
                    }
                });
            }
        });
        let page = Rc::new(Self {
            id,
            kind,
            view,
            hotkeys,
            configuration,
            subscription: RefCell::new(Some(subscription)),
            disposed: Cell::new(false),
        });
        PAGES.with_borrow_mut(|pages| {
            pages.insert(id, Rc::downgrade(&page));
        });
        Ok(page)
    }

    pub(crate) fn view(&self) -> &FeatureView {
        &self.view
    }
    pub(crate) fn refresh(&self) -> Result<(), String> {
        if !self.disposed.get() {
            let next = projection(&self.configuration)?;
            self.hotkeys.refresh(next.general.clone());
            self.view.refresh(next);
        }
        Ok(())
    }
    pub(crate) fn jump(&self, section: &str) -> Result<(), String> {
        let sections: &[&str] = match self.kind {
            FeatureKind::Ocr => &["hotkeys", "recognition", "window"],
            FeatureKind::Translation => &["hotkeys", "language", "window", "behavior"],
        };
        if !sections.contains(&section) {
            return Err("Unknown feature preferences section".into());
        }
        self.view.jump(section);
        Ok(())
    }
    pub(crate) fn suspend(&self) {
        self.hotkeys.cancel_recording();
    }
    pub(crate) fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        self.hotkeys.dispose();
        self.view.dispose();
        self.subscription.borrow_mut().take();
        PAGES.with_borrow_mut(|pages| {
            pages.remove(&self.id);
        });
    }
}

fn projection(configuration: &SettingsConfiguration) -> Result<FeatureProjection, String> {
    let snapshot = configuration
        .snapshot()
        .map_err(|error| error.to_string())?;
    Ok(FeatureProjection {
        general: snapshot.general,
        preferences: snaplingo_core::settings::FeaturePreferences {
            ocr: snapshot.ocr,
            translation: snapshot.translation,
        },
        error: None,
    })
}

impl Drop for FeaturePage {
    fn drop(&mut self) {
        self.dispose();
    }
}

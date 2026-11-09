//! Binds shared-drawn preferences to the application's existing service owners.
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::ptr::NonNull;
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use accesskit::{ActionHandler, ActionRequest, ActivationHandler, TreeUpdate};
use accesskit_macos::SubclassingAdapter;
use block2::RcBlock;
use objc2::{
    rc::Retained, runtime::AnyObject, sel, DefinedClass, MainThreadMarker, MainThreadOnly,
};
use objc2_app_kit::{NSAlert, NSEvent, NSEventMask, NSEventType, NSView};
use objc2_foundation::NSString;
use snaplingo_accessibility::{window_project_region, WindowProjection};
use snaplingo_ui::{Action, Event, Page, Snapshot};

use super::shared_view_macos::SharedFeatureView;
use crate::application::hotkeys::HotkeyRecording;
use crate::application::{HotkeyRuntime, SettingsApplication, SettingsConfiguration};
use crate::domain::hotkey_config::{
    DEFAULT_HOTKEYS, OCR_CATEGORY, SCREENSHOT_CATEGORY, TRANSLATION_CATEGORY,
};
use crate::infrastructure::desktop_events::{DesktopEvent, DesktopSubscription};
use crate::infrastructure::{runtime, ui_thread};
use std::collections::VecDeque;

static NEXT: AtomicU64 = AtomicU64::new(1);
thread_local! {static PAGES:RefCell<BTreeMap<u64,Weak<SharedFeaturePage>>>=const{RefCell::new(BTreeMap::new())};}
fn lookup(id: u64) -> Option<Rc<SharedFeaturePage>> {
    PAGES.with_borrow(|pages| pages.get(&id).and_then(Weak::upgrade))
}
fn refresh_later(id: u64) {
    ui_thread::dispatch(move || {
        if let Some(page) = lookup(id) {
            if let Err(error) = page.refresh() {
                log::warn!("Failed to refresh shared General preferences: {error}");
            }
        }
    });
}
struct Initial(Arc<Mutex<Option<WindowProjection>>>);
impl ActivationHandler for Initial {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        self.0.lock().unwrap().as_ref().map(|p| p.update(None))
    }
}
struct Actions(u64);
impl ActionHandler for Actions {
    fn do_action(&mut self, request: ActionRequest) {
        let id = self.0;
        ui_thread::dispatch(move || {
            if let Some(page) = lookup(id) {
                let events = page
                    .projection
                    .lock()
                    .unwrap()
                    .as_ref()
                    .map(|p| p.actions(&request))
                    .unwrap_or_default();
                for event in events {
                    page.view.window_input(event);
                }
            }
        });
    }
}

pub(crate) struct SharedFeaturePage {
    id: u64,
    view: Retained<SharedFeatureView>,
    configuration: Arc<SettingsConfiguration>,
    settings: Arc<SettingsApplication>,
    hotkeys: Arc<HotkeyRuntime>,
    general: Option<Arc<super::GeneralController>>,
    library: Option<Arc<crate::infrastructure::settings_canvas::library::LibrarySession>>,
    services: Option<Arc<crate::infrastructure::settings_canvas::services::ServicesSession>>,
    recording: HotkeyRecording,
    recording_id: RefCell<Option<String>>,
    sequence: Cell<u64>,
    subscription: RefCell<Option<DesktopSubscription>>,
    usage: Arc<crate::infrastructure::settings_canvas::capacity_usage::CapacityUsage>,
    usage_subscription: RefCell<Option<DesktopSubscription>>,
    monitor: RefCell<Option<Retained<AnyObject>>>,
    accessibility: RefCell<Option<SubclassingAdapter>>,
    projection: Arc<Mutex<Option<WindowProjection>>>,
    sent: RefCell<Option<WindowProjection>>,
    saving: Cell<bool>,
    disposed: Cell<bool>,
    error: RefCell<Option<String>>,
    hotkey_queue: RefCell<VecDeque<(Page, Action)>>,
    host_actions: RefCell<Option<Rc<dyn Fn(Action)>>>,
}
impl SharedFeaturePage {
    pub(crate) fn perform_window(&self, page: Page, actions: Vec<snaplingo_ui::WindowIntent>) {
        for action in actions {
            match action {
                snaplingo_ui::WindowIntent::Settings(action) => self.perform(page, vec![action]),
                snaplingo_ui::WindowIntent::Library(intent) => self.library_intent(intent, false),
                snaplingo_ui::WindowIntent::Services(intent) => self.service_intent(intent),
            }
        }
    }
    pub(crate) fn from_state(
        mtm: MainThreadMarker,
        kind: Page,
        state: &crate::AppState,
        recording: HotkeyRecording,
    ) -> Result<Rc<Self>, String> {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let (usage, usage_subscription) =
            crate::infrastructure::settings_canvas::capacity_usage::CapacityUsage::subscribe(
                state.library_index.clone(),
                &state.desktop_events,
                move || refresh_later(id),
            );
        let services = (kind == Page::Services).then(|| {
            Arc::new(
                crate::infrastructure::settings_canvas::services::ServicesSession::from_state(
                    state,
                    Default::default(),
                ),
            )
        });
        let library = match kind {
            Page::History => Some(snaplingo_ui::library::LibraryKind::History),
            Page::Favorites => Some(snaplingo_ui::library::LibraryKind::Favorites),
            _ => None,
        }
        .map(|kind| {
            Arc::new(
                crate::infrastructure::settings_canvas::library::LibrarySession::from_state(
                    state, kind,
                ),
            )
        });
        let general = (kind == Page::General).then(|| {
            super::shared_general_macos::from_state(
                state,
                env!("CARGO_PKG_VERSION").into(),
                Arc::new(move || {
                    ui_thread::dispatch(move || {
                        if let Some(page) = lookup(id) {
                            if let Some(window) = page.view.window() {
                                if let Ok(saved) = page.configuration.snapshot() {
                                    super::notices_macos::show(&window, &saved.general);
                                }
                            }
                        }
                    });
                    Ok(())
                }),
            )
        });
        let mut snapshot = projection(&state.settings.configuration, &state.settings.hotkeys)?;
        if let Some(controller) = &general {
            super::shared_general_macos::refresh(controller, &mut snapshot)?;
        }
        let view = SharedFeatureView::new(mtm, kind, snapshot);
        if let Some(services) = &services {
            view.refresh_window(snaplingo_ui::WindowSnapshot::Services(services.snapshot(
                projection(&state.settings.configuration, &state.settings.hotkeys)?,
            )?));
        }
        if let Some(library) = &library {
            view.refresh_window(snaplingo_ui::WindowSnapshot::Library(library.snapshot(
                projection(&state.settings.configuration, &state.settings.hotkeys)?,
            )));
        }
        let page = Rc::new(Self {
            id,
            view,
            configuration: state.settings.configuration.clone(),
            settings: state.settings.application.clone(),
            hotkeys: state.settings.hotkeys.clone(),
            general,
            library,
            services,
            recording,
            recording_id: RefCell::new(None),
            sequence: Cell::new(0),
            subscription: RefCell::new(None),
            usage,
            usage_subscription: RefCell::new(Some(usage_subscription)),
            monitor: RefCell::new(None),
            accessibility: RefCell::new(None),
            projection: Arc::new(Mutex::new(None)),
            sent: RefCell::new(None),
            saving: Cell::new(false),
            disposed: Cell::new(false),
            error: RefCell::new(None),
            hotkey_queue: RefCell::new(VecDeque::new()),
            host_actions: RefCell::new(None),
        });
        PAGES.with_borrow_mut(|pages| {
            pages.insert(id, Rc::downgrade(&page));
        });
        page.view.bind(Rc::downgrade(&page));
        *page.accessibility.borrow_mut() = Some(unsafe {
            SubclassingAdapter::new(
                Retained::as_ptr(&page.view).cast_mut().cast(),
                Initial(page.projection.clone()),
                Actions(id),
            )
        });
        let subscription = state.desktop_events.subscribe(move |event| {
            if matches!(
                event,
                DesktopEvent::SettingsChanged
                    | DesktopEvent::HotkeysChanged
                    | DesktopEvent::HotkeyRecorded { .. }
                    | DesktopEvent::HistoryChanged
                    | DesktopEvent::FavoritesChanged
                    | DesktopEvent::ScreenshotFavoritesChanged
                    | DesktopEvent::ProvidersChanged
            ) {
                let event = event.clone();
                ui_thread::dispatch(move || {
                    if let Some(page) = lookup(id) {
                        match event {
                            DesktopEvent::HistoryChanged
                            | DesktopEvent::FavoritesChanged
                            | DesktopEvent::ScreenshotFavoritesChanged
                                if page.library.is_some() =>
                            {
                                page.view.window_input(snaplingo_ui::WindowInput::Library(
                                    snaplingo_ui::library::LibraryInput::Refresh,
                                ));
                            }
                            DesktopEvent::HotkeyRecorded {
                                window_label,
                                recording_id,
                                hotkey,
                            } => {
                                if window_label == "settings"
                                    && page.recording_id.borrow().as_ref() == Some(&recording_id)
                                    && page.recording.session().is_some_and(|session| {
                                        session.id == recording_id
                                            && session.window_label == "settings"
                                    })
                                {
                                    page.view.input(Event::RecordedShortcut(hotkey));
                                }
                            }
                            _ => {
                                if let Err(error) = page.refresh() {
                                    log::warn!("Failed to refresh shared preferences: {error}");
                                }
                            }
                        }
                    }
                });
            }
        });
        *page.subscription.borrow_mut() = Some(subscription);
        let monitor = RcBlock::new(move |event: NonNull<NSEvent>| -> *mut NSEvent {
            if lookup(id).is_some_and(|page| page.receive(unsafe { event.as_ref() })) {
                std::ptr::null_mut()
            } else {
                event.as_ptr()
            }
        });
        *page.monitor.borrow_mut() = unsafe {
            NSEvent::addLocalMonitorForEventsMatchingMask_handler(
                NSEventMask::KeyDown
                    | NSEventMask::KeyUp
                    | NSEventMask::LeftMouseDown
                    | NSEventMask::RightMouseDown,
                &monitor,
            )
        };
        if let Some(controller) = page.general.clone() {
            runtime::spawn(async move {
                let _ = controller.ensure_checked(move |_| refresh_later(id)).await;
            });
        }
        if page.library.is_some() {
            page.view.window_input(snaplingo_ui::WindowInput::Library(
                snaplingo_ui::library::LibraryInput::Refresh,
            ));
        }
        if page.services.is_some() {
            page.service_intent(snaplingo_ui::services::ServicesIntent::Refresh);
        }
        Ok(page)
    }
    pub(crate) fn view(&self) -> &SharedFeatureView {
        &self.view
    }
    pub(crate) fn set_host_actions(&self, handle: Rc<dyn Fn(Action)>) {
        *self.host_actions.borrow_mut() = Some(handle);
    }
    pub(crate) fn refresh(&self) -> Result<(), String> {
        if !self.disposed.get() {
            let mut next = projection(&self.configuration, &self.hotkeys)?;
            self.usage.apply(&mut next);
            next.error = self.error.borrow().clone();
            if let Some(controller) = &self.general {
                super::shared_general_macos::refresh(controller, &mut next)?;
            }
            if let Some(services) = &self.services {
                self.view
                    .refresh_window(snaplingo_ui::WindowSnapshot::Services(
                        services.snapshot(next)?,
                    ));
            } else if let Some(library) = &self.library {
                self.view
                    .refresh_window(snaplingo_ui::WindowSnapshot::Library(
                        library.snapshot(next),
                    ));
            } else {
                self.view.refresh(next);
            }
        }
        Ok(())
    }
    fn service_intent(&self, intent: snaplingo_ui::services::ServicesIntent) {
        if self.disposed.get() {
            return;
        }
        use crate::infrastructure::settings_canvas::services::ServicesHostEffect as Effect;
        use snaplingo_ui::services::ServicesIntent as Intent;
        let effect = match &intent {
            Intent::Navigate(page) => Some(Effect::Navigate(*page)),
            Intent::ConfigureCapacity => Some(Effect::ConfigureCapacity),
            Intent::CopyText(text) => Some(Effect::CopyText(text.clone())),
            Intent::ReadClipboard => Some(Effect::ReadClipboard),
            Intent::OpenUrl(url) => Some(Effect::OpenUrl(url.clone())),
            Intent::ConfirmRemove(id) => Some(Effect::ConfirmRemove(id.clone())),
            _ => None,
        };
        if let Some(effect) = effect {
            self.service_effect(effect);
            return;
        }
        let Some(services) = self.services.clone() else {
            return;
        };
        let route = matches!(&intent, snaplingo_ui::services::ServicesIntent::Route(_));
        match services.edit(&intent) {
            Ok(true) => {
                let _ = self.refresh();
                if route {
                    self.service_intent(snaplingo_ui::services::ServicesIntent::Refresh);
                }
                return;
            }
            Err(error) => {
                self.message(&error);
                return;
            }
            Ok(false) => {}
        }
        let id = self.id;
        runtime::spawn(async move {
            let result = services.dispatch(intent, move || refresh_later(id)).await;
            ui_thread::dispatch(move || {
                if let Some(page) = lookup(id) {
                    match result {
                        Ok(effects) => {
                            for effect in effects {
                                page.service_effect(effect);
                            }
                        }
                        Err(error) => page.message(&error),
                    }
                    let _ = page.refresh();
                }
            });
        });
    }
    fn service_effect(
        &self,
        effect: crate::infrastructure::settings_canvas::services::ServicesHostEffect,
    ) {
        use crate::infrastructure::settings_canvas::services::ServicesHostEffect as Effect;
        match effect {
            Effect::Navigate(page) => self.perform(page, vec![Action::Navigate(page)]),
            Effect::ConfigureCapacity => {
                self.perform(self.view.page(), vec![Action::ConfigureCapacity])
            }
            Effect::CopyText(text) => {
                if let Err(error) = crate::infrastructure::system::clipboard_text::copy_text(text) {
                    self.message(&error);
                }
            }
            Effect::ReadClipboard => match arboard::Clipboard::new().and_then(|mut c| c.get_text())
            {
                Ok(text) => self.view.window_input(snaplingo_ui::WindowInput::Services(
                    snaplingo_ui::services::ServicesInput::ClipboardText(text),
                )),
                Err(error) => self.message(&error.to_string()),
            },
            Effect::OpenUrl(url) => {
                if let Err(error) = crate::infrastructure::system::external_open::open_url(&url) {
                    self.message(&error.to_string());
                }
            }
            Effect::ConfirmRemove(provider) => {
                if let Some(window) = self.view.window() {
                    let alert = NSAlert::new(self.view.mtm());
                    alert.setMessageText(&NSString::from_str("确定要删除这个自定义翻译服务吗？"));
                    alert.addButtonWithTitle(&NSString::from_str("确认"));
                    alert.addButtonWithTitle(&NSString::from_str("取消"));
                    let id = self.id;
                    let callback = RcBlock::new(move |response| {
                        if response == 1000 {
                            if let Some(services) =
                                lookup(id).and_then(|page| page.services.clone())
                            {
                                let provider = provider.clone();
                                runtime::spawn(async move {
                                    if let Err(error) = services
                                        .remove_confirmed(provider, move || refresh_later(id))
                                        .await
                                    {
                                        ui_thread::dispatch(move || {
                                            if let Some(page) = lookup(id) {
                                                page.message(&error);
                                            }
                                        });
                                    }
                                });
                            }
                        }
                    });
                    alert.beginSheetModalForWindow_completionHandler(&window, Some(&callback));
                }
            }
        }
    }
    fn library_intent(&self, intent: snaplingo_ui::library::LibraryIntent, confirmed: bool) {
        use snaplingo_ui::library::{LibraryInput, LibraryIntent};
        if self.disposed.get() {
            return;
        }
        if matches!(intent, LibraryIntent::ReadClipboard) {
            match arboard::Clipboard::new().and_then(|mut clipboard| clipboard.get_text()) {
                Ok(text) => self.view.window_input(snaplingo_ui::WindowInput::Library(
                    LibraryInput::ClipboardText(text),
                )),
                Err(error) => self.message(&error.to_string()),
            }
            return;
        }
        if !confirmed
            && matches!(
                intent,
                LibraryIntent::Delete(_) | LibraryIntent::ConfirmClearHistory(_)
            )
        {
            if let Some(window) = self.view.window() {
                let alert = NSAlert::new(self.view.mtm());
                alert.setMessageText(&NSString::from_str(
                    if matches!(intent, LibraryIntent::Delete(_)) {
                        "确定要删除这条记录吗？"
                    } else {
                        "确定要清空当前类型的全部历史记录吗？独立收藏不会被删除。"
                    },
                ));
                alert.addButtonWithTitle(&NSString::from_str("确认"));
                alert.addButtonWithTitle(&NSString::from_str("取消"));
                let id = self.id;
                let callback = RcBlock::new(move |response| {
                    if response == 1000 {
                        if let Some(page) = lookup(id) {
                            page.library_intent(intent.clone(), true);
                        }
                    }
                });
                alert.beginSheetModalForWindow_completionHandler(&window, Some(&callback));
            }
            return;
        }
        if let Some(library) = self.library.clone() {
            let id = self.id;
            let task = library.dispatch(intent, move || refresh_later(id));
            runtime::spawn(async move {
                let _ = task.await;
            });
        }
    }
    pub(crate) fn jump(&self, section: &str) -> Result<(), String> {
        if self.services.is_some() {
            let route = match section {
                "ocr" => snaplingo_ui::services::ServicesRoute::Ocr,
                "translation" => snaplingo_ui::services::ServicesRoute::Translation,
                "tts" => snaplingo_ui::services::ServicesRoute::Tts,
                _ => return Err("Unknown Services route".into()),
            };
            self.service_intent(snaplingo_ui::services::ServicesIntent::Route(route));
            return Ok(());
        }
        self.view.jump(section)
    }
    pub(crate) fn suspend(&self) {
        self.view.cancel_composition();
        self.view.input(Event::FocusLost);
        self.finish_recording();
    }
    pub(crate) fn focus_changed(&self) {
        let focused = self
            .view
            .window()
            .is_some_and(|window| window.isKeyWindow());
        if !focused {
            self.suspend();
        }
        if let Some(adapter) = self.accessibility.borrow_mut().as_mut() {
            if let Some(events) = adapter.update_view_focus_state(focused) {
                events.raise();
            }
        }
    }
    fn finish_recording(&self) {
        if let Some(id) = self.recording_id.borrow_mut().take() {
            self.recording.end("settings", &id);
        }
    }
    pub(crate) fn perform(&self, page: Page, actions: Vec<Action>) {
        if self.disposed.get() {
            return;
        }
        for action in actions {
            if !matches!(action, Action::EndHotkeyRecording) {
                self.error.borrow_mut().take();
            }
            let result = match action {
                Action::EditOcr { field, value } => self
                    .settings
                    .edit_ocr(
                        snaplingo_settings::ocr_field(field),
                        snaplingo_settings::general_value(value),
                    )
                    .map(|_| ())
                    .map_err(|e| e.to_string()),
                Action::EditTranslation { field, value } => self
                    .settings
                    .edit_translation(
                        snaplingo_settings::translation_field(field),
                        snaplingo_settings::general_value(value),
                    )
                    .map(|_| ())
                    .map_err(|e| e.to_string()),
                Action::EditGeneral { field, value } => {
                    let field = snaplingo_settings::general_field(field);
                    let value = snaplingo_settings::general_value(value);
                    if let Some(controller) = &self.general {
                        controller.edit(field, value).map(|_| ())
                    } else {
                        self.settings
                            .edit_general(field, value)
                            .map(|_| ())
                            .map_err(|e| e.to_string())
                    }
                }
                Action::GeneralCommand(command) => {
                    self.general_command(command, false);
                    Ok(())
                }
                Action::EditScreenshot { field, value } => self
                    .configuration
                    .edit_screenshot(
                        snaplingo_settings::screenshot_field(field),
                        snaplingo_settings::general_value(value),
                    )
                    .map(|_| ())
                    .map_err(|e| e.to_string()),
                Action::BeginHotkeyRecording { .. }
                | Action::BeginLocalShortcutRecording { .. } => {
                    self.view.cancel_composition();
                    self.finish_recording();
                    self.sequence.set(self.sequence.get() + 1);
                    let id = format!("shared:{}:{}", self.id, self.sequence.get());
                    self.recording.begin("settings", id.clone());
                    *self.recording_id.borrow_mut() = Some(id);
                    Ok(())
                }
                Action::EndHotkeyRecording => {
                    self.finish_recording();
                    Ok(())
                }
                Action::SaveHotkey { .. } | Action::ResetHotkey { .. } | Action::ResetHotkeys => {
                    if matches!(action, Action::ResetHotkeys) {
                        self.confirm_reset(page);
                    } else {
                        self.mutate(page, action);
                    }
                    Ok(())
                }
                Action::CheckHotkeyConflicts => {
                    let result = self
                        .hotkeys
                        .snapshot()
                        .map_err(|e| e.to_string())
                        .and_then(|s| snaplingo_settings::validate_hotkey_conflicts(&s));
                    self.message(
                        result
                            .as_ref()
                            .err()
                            .map(String::as_str)
                            .unwrap_or("未发现冲突的快捷键"),
                    );
                    Ok(())
                }
                Action::Navigate(_) | Action::ConfigureCapacity => {
                    if let Some(handle) = self.host_actions.borrow().as_ref() {
                        handle(action);
                    }
                    Ok(())
                }
                Action::SelectScreenshotDirectory => {
                    self.select_directory();
                    Ok(())
                }
                Action::SelectScreenshotColor { field, rgba } => {
                    match super::shared_dialogs_macos::choose_color(rgba) {
                        Ok(Some(color)) => {
                            self.view.input(Event::SetValue(
                                snaplingo_ui::Target::Screenshot(field),
                                snaplingo_ui::Value::Number(u32::from_be_bytes(color)),
                            ));
                            Ok(())
                        }
                        Ok(None) => Ok(()),
                        Err(error) => Err(error),
                    }
                }
                Action::CopyToClipboard(text) => {
                    crate::infrastructure::system::clipboard_text::copy_text(text)
                }
                Action::ReadClipboard => {
                    match arboard::Clipboard::new().and_then(|mut clipboard| clipboard.get_text()) {
                        Ok(text) => {
                            self.view.input(Event::ClipboardText(text));
                            Ok(())
                        }
                        Err(error) => Err(error.to_string()),
                    }
                }
            };
            if let Err(error) = result {
                *self.error.borrow_mut() = Some(error.clone());
                self.message(&error);
            }
        }
        if let Err(error) = self.refresh() {
            log::warn!("Failed to refresh shared preferences after edit: {error}");
        }
    }
    fn general_command(&self, command: snaplingo_ui::GeneralCommand, confirmed: bool) {
        let Some(controller) = self.general.clone() else {
            return;
        };
        if !confirmed {
            if let Some(message) = super::shared_general_macos::confirmation(command) {
                if let Some(window) = self.view.window() {
                    let alert = NSAlert::new(self.view.mtm());
                    alert.setMessageText(&NSString::from_str(message));
                    alert.addButtonWithTitle(&NSString::from_str("确认"));
                    alert.addButtonWithTitle(&NSString::from_str("取消"));
                    let id = self.id;
                    let callback = RcBlock::new(move |response| {
                        if response == 1000 {
                            if let Some(page) = lookup(id) {
                                page.general_command(command, true);
                            }
                        }
                    });
                    alert.beginSheetModalForWindow_completionHandler(&window, Some(&callback));
                }
                return;
            }
        }
        let id = self.id;
        runtime::spawn(async move {
            let _ = controller
                .perform(super::shared_general_macos::action(command), move |_| {
                    refresh_later(id)
                })
                .await;
        });
    }
    fn select_directory(&self) {
        let initial = self
            .configuration
            .snapshot()
            .ok()
            .map(|s| std::path::PathBuf::from(s.screenshot.save_path))
            .filter(|p| !p.as_os_str().is_empty());
        let id = self.id;
        runtime::spawn(async move {
            let selected = super::shared_dialogs_macos::select_directory(initial).await;
            ui_thread::dispatch(move || {
                if let Some(page) = lookup(id).filter(|page| !page.disposed.get()) {
                    match selected {
                        Ok(Some(path)) => page.view.input(Event::SetValue(
                            snaplingo_ui::Target::Screenshot(
                                snaplingo_ui::ScreenshotField::SavePath,
                            ),
                            snaplingo_ui::Value::Text(path.to_string_lossy().into_owned()),
                        )),
                        Ok(None) => {}
                        Err(error) => page.message(&error),
                    }
                }
            });
        });
    }
    fn mutate(&self, page: Page, action: Action) {
        if self.saving.replace(true) {
            self.hotkey_queue.borrow_mut().push_back((page, action));
            return;
        }
        self.finish_recording();
        self.view.cancel_composition();
        let runtime = self.hotkeys.clone();
        let id = self.id;
        runtime::spawn_blocking(move || {
            let category = match page {
                Page::Ocr => OCR_CATEGORY,
                Page::Screenshot => SCREENSHOT_CATEGORY,
                _ => TRANSLATION_CATEGORY,
            };
            let target = |index| {
                DEFAULT_HOTKEYS
                    .iter()
                    .filter(|binding| binding.category == category)
                    .nth(index)
                    .map(|binding| binding.action)
                    .ok_or_else(|| "Unknown Settings shortcut".to_string())
            };
            let result = match action {
                Action::SaveHotkey { index, shortcut } => target(index).and_then(|action| {
                    runtime
                        .update_hotkey(category.into(), action.into(), shortcut)
                        .map(|_| ())
                        .map_err(|e| e.to_string())
                }),
                Action::ResetHotkey { index } => target(index).and_then(|action| {
                    runtime
                        .reset_hotkey(category.into(), action.into())
                        .map(|_| ())
                        .map_err(|e| e.to_string())
                }),
                Action::ResetHotkeys => runtime
                    .reset_category(category.into())
                    .map(|_| ())
                    .map_err(|e| e.to_string()),
                _ => Ok(()),
            };
            ui_thread::dispatch(move || {
                if let Some(page) = lookup(id) {
                    page.saving.set(false);
                    if let Err(error) = result {
                        *page.error.borrow_mut() = Some(error.clone());
                        page.message(&error);
                    } else {
                        page.error.borrow_mut().take();
                    }
                    let _ = page.refresh();
                    let next = page.hotkey_queue.borrow_mut().pop_front();
                    if let Some((kind, action)) = next {
                        page.mutate(kind, action);
                    }
                }
            });
        });
    }
    fn confirm_reset(&self, page: Page) {
        if let Some(window) = self.view.window() {
            let alert = NSAlert::new(self.view.mtm());
            alert.setMessageText(&NSString::from_str("确定要恢复这一组快捷键到默认值吗？"));
            alert.addButtonWithTitle(&NSString::from_str("确认"));
            alert.addButtonWithTitle(&NSString::from_str("取消"));
            let id = self.id;
            let callback = RcBlock::new(move |response| {
                if response == 1000 {
                    if let Some(owner) = lookup(id) {
                        owner.mutate(page, Action::ResetHotkeys);
                    }
                }
            });
            alert.beginSheetModalForWindow_completionHandler(&window, Some(&callback));
        }
    }
    fn message(&self, message: &str) {
        if let Some(window) = self.view.window() {
            let alert = NSAlert::new(self.view.mtm());
            alert.setMessageText(&NSString::from_str(message));
            alert.addButtonWithTitle(&NSString::from_str("确认"));
            alert.beginSheetModalForWindow_completionHandler(&window, None);
        }
    }
    fn receive(&self, event: &NSEvent) -> bool {
        if self.disposed.get() || self.recording_id.borrow().is_none() {
            return false;
        }
        if !self.recording.session().is_some_and(|session| {
            self.recording_id.borrow().as_ref() == Some(&session.id)
                && session.window_label == "settings"
        }) {
            self.suspend();
            return false;
        }
        let focused = self.view.window().is_some_and(|window| {
            window.isKeyWindow() && window.windowNumber() == event.windowNumber()
        });
        if !focused {
            self.suspend();
            return false;
        }
        if matches!(
            event.r#type(),
            NSEventType::LeftMouseDown | NSEventType::RightMouseDown
        ) {
            let point = self
                .view
                .convertPoint_fromView(event.locationInWindow(), None);
            let inside = point.x >= 0.
                && point.y >= 0.
                && point.x < self.view.bounds().size.width
                && point.y < self.view.bounds().size.height;
            if !inside || event.r#type() == NSEventType::RightMouseDown {
                self.suspend();
            }
            return false;
        }
        if self.saving.get() {
            return true;
        }
        if event.r#type() == NSEventType::KeyDown {
            self.view.key(event, true);
            return true;
        }
        if event.r#type() == NSEventType::KeyUp {
            self.view.key(event, false);
            return true;
        }
        false
    }
    pub(crate) fn accessibility_changed(&self) {
        let Some(next) = self.view.with_window(|frame, ui, scale| {
            window_project_region(
                frame,
                ui,
                scale,
                self.view.presentation_region(&frame.scene),
            )
        }) else {
            return;
        };
        *self.projection.lock().unwrap() = Some(next.clone());
        if self.sent.borrow().as_ref() == Some(&next) {
            return;
        }
        if let Some(adapter) = self.accessibility.borrow_mut().as_mut() {
            let mut delivered = false;
            let previous = self.sent.borrow();
            let events = adapter.update_if_active(|| {
                delivered = true;
                next.update(previous.as_ref())
            });
            drop(previous);
            if let Some(events) = events {
                events.raise();
            }
            if delivered {
                *self.sent.borrow_mut() = Some(next);
            }
        }
    }
    pub(crate) fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        self.finish_recording();
        self.view.cancel_composition();
        if let Some(controller) = &self.general {
            controller.invalidate_pending_updates();
        }
        if let Some(library) = &self.library {
            library.dispose();
        }
        if let Some(services) = &self.services {
            services.dispose();
        }
        if let Some(window) = self.view.window() {
            super::notices_macos::close_for(&window);
        }
        self.subscription.borrow_mut().take();
        self.usage_subscription.borrow_mut().take();
        self.usage.dispose();
        self.hotkey_queue.borrow_mut().clear();
        if let Some(monitor) = self.monitor.borrow_mut().take() {
            unsafe {
                NSEvent::removeMonitor(&monitor);
            }
        }
        self.accessibility.borrow_mut().take();
        PAGES.with_borrow_mut(|pages| {
            pages.remove(&self.id);
        });
    }
    #[cfg(test)]
    pub(crate) fn test_recording_id(&self) -> Option<String> {
        self.recording_id.borrow().clone()
    }
}
impl Drop for SharedFeaturePage {
    fn drop(&mut self) {
        self.dispose();
    }
}
fn projection(
    configuration: &SettingsConfiguration,
    hotkeys: &HotkeyRuntime,
) -> Result<Snapshot, String> {
    let saved = configuration.snapshot().map_err(|e| e.to_string())?;
    let mut snapshot = snaplingo_settings::project_snapshot(
        &saved,
        &hotkeys.snapshot().map_err(|e| e.to_string())?,
        &hotkeys.default_snapshot(),
    );
    snapshot.dark = super::super::native_theme::is_dark(&saved.general.theme);
    Ok(snapshot)
}

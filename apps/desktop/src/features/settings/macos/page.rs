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
use snaplingo_ui::accessibility::{window_project_region, WindowProjection};
use snaplingo_ui::{Action, Event, Page};

use super::view::SharedFeatureView;
use crate::features::settings::presenter::window::SettingsWindowSession;
use crate::integrations::desktop_events::DesktopEvent;
use crate::integrations::desktop_events::DesktopSubscription;
use crate::integrations::runtime;
use crate::integrations::ui_thread;
use snaplingo_core::hotkeys::HotkeyRecording;
use snaplingo_core::settings::SettingsConfiguration;

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
    session: RefCell<Option<SettingsWindowSession>>,
    recording: HotkeyRecording,
    recording_id: RefCell<Option<String>>,
    subscription: RefCell<Option<DesktopSubscription>>,
    hotkey_subscription: RefCell<Option<DesktopSubscription>>,
    monitor: RefCell<Option<Retained<AnyObject>>>,
    accessibility: RefCell<Option<SubclassingAdapter>>,
    projection: Arc<Mutex<Option<WindowProjection>>>,
    sent: RefCell<Option<WindowProjection>>,
    disposed: Cell<bool>,
    error: RefCell<Option<String>>,
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
    pub(crate) fn new(
        mtm: MainThreadMarker,
        kind: Page,
        state: &crate::features::settings::Services,
        recording: HotkeyRecording,
    ) -> Result<Rc<Self>, String> {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let (mut session, subscription) = (state.session)(
            recording.clone(),
            Arc::new(move || {
                ui_thread::dispatch(move || {
                    if let Some(page) = lookup(id) {
                        if let Some(window) = page.view.window() {
                            if let Ok(saved) = page.configuration.snapshot() {
                                super::notices::show(&window, &saved.general);
                            }
                        }
                    }
                });
                Ok(())
            }),
            Arc::new(move || {
                ui_thread::dispatch(move || {
                    if let Some(page) = lookup(id) {
                        if let Some(handle) = page.host_actions.borrow().as_ref() {
                            handle(Action::ConfigureCapacity);
                        }
                    }
                });
                Ok(())
            }),
        );
        session.set_notifier(Some(Arc::new(move || refresh_later(id))));
        session.set_system_dark(crate::integrations::system::native_theme::is_dark(
            &state
                .configuration
                .snapshot()
                .map_err(|e| e.to_string())?
                .general
                .theme,
        ));
        session.navigate(kind)?;
        let snapshot = session.snapshot()?;
        let shell = match &snapshot {
            snaplingo_ui::WindowSnapshot::Settings(shell) => shell.clone(),
            snaplingo_ui::WindowSnapshot::Services(snapshot) => snapshot.shell.clone(),
            snaplingo_ui::WindowSnapshot::Library(snapshot) => snapshot.shell.clone(),
        };
        let view = SharedFeatureView::new(mtm, kind, shell);
        view.refresh_window(snapshot);
        let page = Rc::new(Self {
            id,
            view,
            configuration: state.configuration.clone(),
            session: RefCell::new(Some(session)),
            recording,
            recording_id: RefCell::new(None),
            subscription: RefCell::new(Some(subscription)),
            hotkey_subscription: RefCell::new(None),
            monitor: RefCell::new(None),
            accessibility: RefCell::new(None),
            projection: Arc::new(Mutex::new(None)),
            sent: RefCell::new(None),
            disposed: Cell::new(false),
            error: RefCell::new(None),
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
        let hotkey_subscription = state.events.subscribe(move |event| {
            if let DesktopEvent::HotkeyRecorded {
                window_label,
                recording_id,
                hotkey,
            } = event
            {
                if window_label != "settings" {
                    return;
                }
                let recording_id = recording_id.clone();
                let hotkey = hotkey.clone();
                ui_thread::dispatch(move || {
                    if let Some(page) = lookup(id) {
                        if page.recording_id.borrow().as_ref() == Some(&recording_id)
                            && page.recording.session().is_some_and(|session| {
                                session.id == recording_id && session.window_label == "settings"
                            })
                        {
                            page.view.input(Event::RecordedShortcut(hotkey));
                        }
                    }
                });
            }
        });
        *page.hotkey_subscription.borrow_mut() = Some(hotkey_subscription);
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
        Ok(page)
    }
    pub(crate) fn view(&self) -> &SharedFeatureView {
        &self.view
    }
    pub(crate) fn set_host_actions(&self, handle: Rc<dyn Fn(Action)>) {
        *self.host_actions.borrow_mut() = Some(handle);
    }
    pub(crate) fn refresh(&self) -> Result<(), String> {
        if self.disposed.get() {
            return Ok(());
        }
        let snapshot = {
            let mut session = self.session.borrow_mut();
            let session = session.as_mut().ok_or("Settings session is closed")?;
            session.set_system_dark(crate::integrations::system::native_theme::is_dark(
                &self
                    .configuration
                    .snapshot()
                    .map_err(|e| e.to_string())?
                    .general
                    .theme,
            ));
            session.snapshot()?
        };
        self.view.refresh_window(snapshot);
        Ok(())
    }
    fn dispatch(&self, intent: snaplingo_ui::WindowIntent) -> Result<(), String> {
        self.session
            .borrow_mut()
            .as_mut()
            .ok_or("Settings session is closed")?
            .dispatch(self.view.page(), intent)
    }
    fn service_intent(&self, intent: snaplingo_ui::services::ServicesIntent) {
        if self.disposed.get() {
            return;
        }
        use crate::features::settings::presenter::services::ServicesHostEffect as Effect;
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
        if let Err(error) = self.dispatch(snaplingo_ui::WindowIntent::Services(intent)) {
            self.message(&error);
        }
        let _ = self.refresh();
    }
    fn service_effect(
        &self,
        effect: crate::features::settings::presenter::services::ServicesHostEffect,
    ) {
        use crate::features::settings::presenter::services::ServicesHostEffect as Effect;
        match effect {
            Effect::Navigate(page) => self.perform(page, vec![Action::Navigate(page)]),
            Effect::ConfigureCapacity => {
                self.perform(self.view.page(), vec![Action::ConfigureCapacity])
            }
            Effect::CopyText(text) => {
                if let Err(error) = snaplingo_platform::clipboard_text::copy_text(text) {
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
                if let Err(error) = crate::integrations::system::external_open::open_url(&url) {
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
                            if let Some(page) = lookup(id) {
                                if let Err(error) =
                                    page.dispatch(snaplingo_ui::WindowIntent::Services(
                                        snaplingo_ui::services::ServicesIntent::ConfirmRemove(
                                            provider.clone(),
                                        ),
                                    ))
                                {
                                    page.message(&error);
                                }
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
        if let Err(error) = self.dispatch(snaplingo_ui::WindowIntent::Library(intent)) {
            self.message(&error);
        }
    }
    pub(crate) fn jump(&self, section: &str) -> Result<(), String> {
        if self.view.page() == Page::Services {
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
    pub(crate) fn perform(&self, _page: Page, actions: Vec<Action>) {
        if self.disposed.get() {
            return;
        }
        for action in actions {
            if !matches!(action, Action::EndHotkeyRecording) {
                self.error.borrow_mut().take();
            }
            let result = match action {
                action @ (Action::EditOcr { .. }
                | Action::EditTranslation { .. }
                | Action::EditGeneral { .. }
                | Action::EditScreenshot { .. }) => {
                    self.dispatch(snaplingo_ui::WindowIntent::Settings(action))
                }
                Action::GeneralCommand(command) => {
                    self.general_command(command, false);
                    Ok(())
                }
                Action::BeginHotkeyRecording { .. }
                | Action::BeginLocalShortcutRecording { .. } => {
                    self.view.cancel_composition();
                    self.finish_recording();
                    let result = self.dispatch(snaplingo_ui::WindowIntent::Settings(action));
                    *self.recording_id.borrow_mut() = self
                        .session
                        .borrow()
                        .as_ref()
                        .and_then(|session| session.recording_id());
                    result
                }
                Action::EndHotkeyRecording => {
                    self.finish_recording();
                    Ok(())
                }
                Action::SaveHotkey { .. } | Action::ResetHotkey { .. } | Action::ResetHotkeys => {
                    if matches!(action, Action::ResetHotkeys) {
                        self.confirm_reset();
                    } else {
                        self.mutate(action);
                    }
                    Ok(())
                }
                Action::CheckHotkeyConflicts => {
                    let result = self.dispatch(snaplingo_ui::WindowIntent::Settings(action));
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
                    match super::dialogs::choose_color(rgba) {
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
                    snaplingo_platform::clipboard_text::copy_text(text)
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
        if !confirmed {
            if let Some(message) =
                crate::features::settings::presenter::general::confirmation(command)
            {
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
        if let Err(error) = self.dispatch(snaplingo_ui::WindowIntent::Settings(
            Action::GeneralCommand(command),
        )) {
            self.message(&error);
        }
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
            let selected = super::dialogs::select_directory(initial).await;
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
    fn mutate(&self, action: Action) {
        self.finish_recording();
        self.view.cancel_composition();
        if let Err(error) = self.dispatch(snaplingo_ui::WindowIntent::Settings(action)) {
            self.message(&error);
        }
    }
    fn confirm_reset(&self) {
        if let Some(window) = self.view.window() {
            let alert = NSAlert::new(self.view.mtm());
            alert.setMessageText(&NSString::from_str("确定要恢复这一组快捷键到默认值吗？"));
            alert.addButtonWithTitle(&NSString::from_str("确认"));
            alert.addButtonWithTitle(&NSString::from_str("取消"));
            let id = self.id;
            let callback = RcBlock::new(move |response| {
                if response == 1000 {
                    if let Some(owner) = lookup(id) {
                        owner.mutate(Action::ResetHotkeys);
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
        if self
            .session
            .borrow()
            .as_ref()
            .is_some_and(|session| session.saving_hotkeys())
        {
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
        self.session.borrow_mut().take();
        if let Some(window) = self.view.window() {
            super::notices::close_for(&window);
        }
        self.subscription.borrow_mut().take();
        self.hotkey_subscription.borrow_mut().take();
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

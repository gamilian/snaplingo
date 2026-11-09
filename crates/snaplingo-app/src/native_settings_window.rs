//! AppKit ownership for the standalone Rust Settings canvas.
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::sync::Arc;

use objc2::{
    define_class, msg_send, rc::Retained, runtime::ProtocolObject, DefinedClass, MainThreadMarker,
    MainThreadOnly,
};
use objc2_app_kit::{
    NSApplication, NSBackingStoreType, NSScreen, NSUserInterfaceItemIdentification, NSView,
    NSWindow, NSWindowDelegate, NSWindowStyleMask,
};
use objc2_foundation::{NSNotification, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString};
use snaplingo_ui::{Action, Page};

use crate::application::hotkeys::HotkeyRecording;
use crate::infrastructure::system::settings_window::SharedFeaturePage;
use crate::AppState;

struct RootState {
    owner: RefCell<Weak<NativeSettingsWindow>>,
}
define_class!(
    #[unsafe(super(NSView))]
    #[name = "SnapLingoNativeSettingsRoot"]
    #[thread_kind = MainThreadOnly]
    #[ivars = RootState]
    struct SettingsRoot;
    unsafe impl NSObjectProtocol for SettingsRoot {}
    unsafe impl NSWindowDelegate for SettingsRoot {
        #[unsafe(method(windowShouldClose:))]
        fn should_close(&self, _: &NSWindow) -> bool {
            if let Some(owner) = self.ivars().owner.borrow().upgrade() {
                if let Err(error) = owner.hide() { log::warn!("Failed to hide native Settings: {error}"); }
            }
            false
        }
        #[unsafe(method(windowDidResize:))]
        fn resized(&self, _: &NSNotification) {
            if let Some(owner) = self.ivars().owner.borrow().upgrade() { owner.layout(); }
        }
        #[unsafe(method(windowDidChangeScreen:))]
        fn changed_screen(&self, _: &NSNotification) {
            if let Some(owner) = self.ivars().owner.borrow().upgrade() { owner.layout(); }
        }
        #[unsafe(method(windowDidBecomeKey:))]
        fn became_key(&self, _: &NSNotification) {
            if let Some(owner) = self.ivars().owner.borrow().upgrade() { owner.refresh_permissions(); }
        }
    }
    impl SettingsRoot {
        #[unsafe(method(isFlipped))]
        fn flipped(&self) -> bool { true }
    }
);

pub(crate) struct NativeSettingsWindow {
    window: Retained<NSWindow>,
    root: Retained<SettingsRoot>,
    page: RefCell<Option<Rc<SharedFeaturePage>>>,
    state: Arc<AppState>,
    recording: HotkeyRecording,
    permission_prompt:
        Arc<crate::infrastructure::settings_canvas::permission_prompt::PermissionPromptSession>,
}

impl NativeSettingsWindow {
    pub(crate) fn new(
        mtm: MainThreadMarker,
        state: Arc<AppState>,
        recording: HotkeyRecording,
    ) -> Result<Rc<Self>, String> {
        let settings = state
            .settings
            .configuration
            .snapshot()
            .map_err(|error| error.to_string())?
            .general;
        let width = f64::from(settings.settings_window_width.unwrap_or(900).max(700));
        let height = f64::from(settings.settings_window_height.unwrap_or(650).max(500));
        let rect = NSRect::new(NSPoint::new(0., 0.), NSSize::new(width, height));
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                mtm.alloc(),
                rect,
                NSWindowStyleMask::Titled
                    | NSWindowStyleMask::Closable
                    | NSWindowStyleMask::Miniaturizable
                    | NSWindowStyleMask::Resizable,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        unsafe {
            window.setReleasedWhenClosed(false);
        }
        window.setTitle(&NSString::from_str("SnapLingo"));
        window.setIdentifier(Some(&NSString::from_str("settings")));
        window.setHidesOnDeactivate(false);
        window.setContentMinSize(NSSize::new(700., 500.));
        if let (Some(x), Some(y)) = (settings.settings_window_x, settings.settings_window_y) {
            let frame = window.frame();
            window.setFrameOrigin(NSPoint::new(
                f64::from(x),
                desktop_top(mtm) - f64::from(y) - frame.size.height,
            ));
        } else {
            window.center();
        }
        let root = SettingsRoot::alloc(mtm).set_ivars(RootState {
            owner: RefCell::new(Weak::new()),
        });
        let root: Retained<SettingsRoot> = unsafe { msg_send![super(root), initWithFrame: rect] };
        window.setContentView(Some(&root));
        window.setDelegate(Some(ProtocolObject::from_ref(&*root)));
        let permission_prompt = Arc::new(
            crate::infrastructure::settings_canvas::permission_prompt::PermissionPromptSession::new(
                state.permissions.clone(),
            ),
        );
        let owner = Rc::new(Self {
            window,
            root,
            page: RefCell::new(None),
            state,
            recording,
            permission_prompt,
        });
        *owner.root.ivars().owner.borrow_mut() = Rc::downgrade(&owner);
        Ok(owner)
    }

    pub(crate) fn show_page(&self, page: Page, section: Option<&str>) -> Result<(), String> {
        self.permission_prompt.set_visible(true);
        let replace = self
            .page
            .borrow()
            .as_ref()
            .is_none_or(|current| current.view().page() != page);
        if replace {
            self.dispose_page();
            let next = SharedFeaturePage::from_state(
                self.root.mtm(),
                page,
                &self.state,
                self.recording.clone(),
            )?;
            next.view().set_full_window(true);
            let window = self.window.clone();
            let state = self.state.clone();
            next.set_host_actions(Rc::new(move |action| {
                let result = match action {
                    Action::Navigate(page) => crate::native_app::show_page(page, None),
                    Action::ConfigureCapacity => {
                        crate::native_capacity::show(&window, state.clone())
                    }
                    _ => Ok(()),
                };
                if let Err(error) = result {
                    log::warn!("Failed to handle native Settings navigation: {error}");
                }
            }));
            self.root.addSubview(next.view());
            *self.page.borrow_mut() = Some(next);
        }
        self.layout();
        if let Some(current) = self.page.borrow().as_ref() {
            current.refresh()?;
            if let Some(section) = section {
                current.jump(section)?;
            }
        }
        self.window.makeKeyAndOrderFront(None);
        NSApplication::sharedApplication(self.root.mtm()).activate();
        crate::native_permission_prompt::refresh(
            &self.window,
            self.state.clone(),
            self.permission_prompt.clone(),
        )?;
        Ok(())
    }

    pub(crate) fn refresh_permissions(&self) {
        if !self.window.isVisible() {
            return;
        }
        if let Some(page) = self.page.borrow().as_ref() {
            if let Err(error) = page.refresh() {
                log::warn!("Failed to refresh Settings after activation: {error}");
            }
        }
        if let Err(error) = crate::native_permission_prompt::refresh(
            &self.window,
            self.state.clone(),
            self.permission_prompt.clone(),
        ) {
            log::warn!("Failed to refresh permission guide: {error}");
        }
    }

    pub(crate) fn layout(&self) {
        if let Some(page) = self.page.borrow().as_ref() {
            let viewport = self.root.frame().size;
            let display = self.window.screen().map(|screen| screen.frame());
            let saved = self.state.settings.configuration.snapshot().ok();
            let scale = crate::application::capture::interface_scale(
                saved
                    .as_ref()
                    .map(|saved| saved.screenshot.capture_ui_scale.as_str())
                    .unwrap_or("auto"),
                display
                    .as_ref()
                    .map(|frame| crate::domain::capture::LogicalRect {
                        x: frame.origin.x,
                        y: frame.origin.y,
                        width: frame.size.width,
                        height: frame.size.height,
                    })
                    .as_ref(),
            );
            let size = NSSize::new(viewport.width / scale, viewport.height / scale);
            self.root.setBoundsSize(size);
            page.view().set_layout(size.width, viewport.width, size);
        }
    }

    pub(crate) fn hide(&self) -> Result<(), String> {
        self.permission_prompt.set_visible(false);
        crate::native_permission_prompt::close_for(&self.window);
        crate::native_capacity::close_for(&self.window);
        if let Some(page) = self.page.borrow().as_ref() {
            page.suspend();
        }
        self.remember_geometry()?;
        self.window.orderOut(None);
        Ok(())
    }

    pub(crate) fn remember_geometry(&self) -> Result<(), String> {
        let frame = self.window.frame();
        let content = self.window.contentRectForFrameRect(frame);
        let mut general = self
            .state
            .settings
            .configuration
            .snapshot()
            .map_err(|error| error.to_string())?
            .general;
        general.settings_window_x = Some(frame.origin.x.round() as i32);
        general.settings_window_y = Some(
            (desktop_top(self.root.mtm()) - frame.origin.y - frame.size.height).round() as i32,
        );
        general.settings_window_width = Some(content.size.width.round() as u32);
        general.settings_window_height = Some(content.size.height.round() as u32);
        self.state
            .settings
            .configuration
            .update_general(general)
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub(crate) fn page(&self) -> Option<Page> {
        self.page.borrow().as_ref().map(|page| page.view().page())
    }

    fn dispose_page(&self) {
        let page = self.page.borrow_mut().take();
        if let Some(page) = page {
            page.suspend();
            page.dispose();
            page.view().removeFromSuperview();
        }
    }

    pub(crate) fn dispose(&self) {
        self.permission_prompt.set_visible(false);
        crate::native_permission_prompt::close_for(&self.window);
        crate::native_capacity::close_for(&self.window);
        self.dispose_page();
        self.window.setDelegate(None);
        self.window.setContentView(None);
        self.window.close();
    }

    #[cfg(test)]
    pub(crate) fn test_window(&self) -> &NSWindow {
        &self.window
    }
    #[cfg(test)]
    pub(crate) fn test_page(&self) -> Option<Page> {
        self.page.borrow().as_ref().map(|page| page.view().page())
    }

    #[cfg(test)]
    pub(crate) fn test_navigate(&self, page: Page) {
        self.test_activate(snaplingo_ui::Target::Navigate(page));
    }

    #[cfg(test)]
    pub(crate) fn test_activate(&self, target: snaplingo_ui::Target) {
        let current = self.page.borrow().as_ref().unwrap().clone();
        let point = current
            .view()
            .with_window(|frame, _, _| {
                let hit = frame
                    .scene
                    .hits
                    .iter()
                    .find(|hit| hit.target == target)
                    .unwrap_or_else(|| {
                        panic!(
                            "Native {:?} canvas has no {target:?} hit; scene size={:?}, hits={:?}",
                            current.view().page(),
                            frame.scene.size,
                            frame
                                .scene
                                .hits
                                .iter()
                                .map(|hit| hit.target)
                                .collect::<Vec<_>>()
                        )
                    });
                snaplingo_ui::Point {
                    x: hit.rect.x + hit.rect.width * 0.5,
                    y: hit.rect.y + hit.rect.height * 0.5,
                }
            })
            .unwrap();
        current
            .view()
            .input(snaplingo_ui::Event::PointerDown(point));
    }

    #[cfg(test)]
    pub(crate) fn test_geometry(&self) -> (NSSize, f64) {
        let page = self.page.borrow().as_ref().unwrap().clone();
        (
            page.view().bounds().size,
            self.root.frame().size.width / self.root.bounds().size.width,
        )
    }
}

fn desktop_top(mtm: MainThreadMarker) -> f64 {
    NSScreen::screens(mtm)
        .iter()
        .map(|screen| screen.frame().origin.y + screen.frame().size.height)
        .fold(0., f64::max)
}

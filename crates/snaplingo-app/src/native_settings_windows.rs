//! Resident Windows Settings owner; drawing and input remain in the shared host.
use crate::application::hotkeys::HotkeyRecording;
use crate::infrastructure::settings_canvas::window::SettingsWindowSession;
use snaplingo_desktop::{HostEvent, WindowBackend, WindowHost};
use snaplingo_ui::{Page, Size, WindowIntent, WindowSnapshot};
use std::{cell::RefCell, rc::Rc, sync::Arc};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalPosition, LogicalSize},
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoopProxy},
    window::{Theme, Window, WindowId},
};

#[derive(Clone)]
struct Backend(Rc<RefCell<SettingsWindowSession>>);
impl WindowBackend for Backend {
    fn snapshot(&self) -> Result<WindowSnapshot, String> {
        self.0.borrow().snapshot()
    }
    fn dispatch(&mut self, page: Page, intent: WindowIntent) -> Result<(), String> {
        self.0.borrow_mut().dispatch(page, intent)
    }
    fn set_notifier(&mut self, callback: Option<Arc<dyn Fn() + Send + Sync>>) {
        self.0.borrow().set_notifier(callback);
    }
    fn interface_scale(&self, display: Size) -> f32 {
        self.0.borrow().interface_scale(display)
    }
}
pub(crate) struct NativeSettingsWindow {
    host: WindowHost,
    backend: Backend,
    state: Arc<crate::AppState>,
    appearance: Option<String>,
}
impl NativeSettingsWindow {
    pub(crate) fn new(
        event_loop: &ActiveEventLoop,
        proxy: EventLoopProxy<HostEvent>,
        state: Arc<crate::AppState>,
        recording: HotkeyRecording,
        notices: Arc<dyn Fn() -> crate::Result<()> + Send + Sync>,
        capacity: Arc<dyn Fn() -> crate::Result<()> + Send + Sync>,
    ) -> Result<Self, String> {
        let general = state
            .settings
            .configuration
            .snapshot()
            .map_err(|e| e.to_string())?
            .general;
        let mut attributes = Window::default_attributes()
            .with_title("SnapLingo")
            .with_inner_size(LogicalSize::new(
                general.settings_window_width.unwrap_or(900).max(700),
                general.settings_window_height.unwrap_or(650).max(500),
            ))
            .with_min_inner_size(LogicalSize::new(700., 500.));
        if let (Some(x), Some(y)) = (general.settings_window_x, general.settings_window_y) {
            attributes = attributes.with_position(LogicalPosition::new(x, y));
        }
        let backend = Backend(Rc::new(RefCell::new(SettingsWindowSession::new(
            state.clone(),
            recording,
            notices,
            capacity,
        ))));
        let mut host = WindowHost::new(backend.clone(), proxy, attributes, true)?;
        host.resumed(event_loop);
        if let Some(error) = host.take_failure() {
            return Err(error);
        }
        let mut owner = Self {
            host,
            backend,
            state,
            appearance: None,
        };
        owner.sync_theme();
        Ok(owner)
    }
    pub(crate) fn window_id(&self) -> WindowId {
        self.host.window().unwrap().id()
    }
    pub(crate) fn window(&self) -> &Window {
        self.host.window().unwrap()
    }
    pub(crate) fn window_rc(&self) -> Rc<Window> {
        self.host.window_rc().unwrap()
    }
    pub(crate) fn show(&mut self, page: Page) -> Result<(), String> {
        self.host.navigate(page)?;
        self.sync_theme();
        self.host.show();
        Ok(())
    }
    pub(crate) fn section(&mut self, section: snaplingo_ui::Section) {
        self.host.activate_section(section);
    }
    pub(crate) fn recorded_shortcut(&mut self, shortcut: String) {
        self.host.recorded_shortcut(shortcut);
    }
    pub(crate) fn hide(&mut self) -> Result<(), String> {
        let saved = self.remember_geometry();
        crate::native_capacity_windows::close_for(self.window_id());
        crate::native_notices_windows::close_for(self.window_id());
        self.host.hide();
        saved
    }
    pub(crate) fn remember_geometry(&self) -> Result<(), String> {
        let window = self.window();
        let scale = window.scale_factor();
        let size = window.inner_size().to_logical::<f64>(scale);
        let position = window
            .outer_position()
            .map_err(|e| e.to_string())?
            .to_logical::<f64>(scale);
        let mut general = self
            .state
            .settings
            .configuration
            .snapshot()
            .map_err(|e| e.to_string())?
            .general;
        general.settings_window_x = Some(position.x.round() as i32);
        general.settings_window_y = Some(position.y.round() as i32);
        general.settings_window_width = Some(size.width.round() as u32);
        general.settings_window_height = Some(size.height.round() as u32);
        self.state
            .settings
            .configuration
            .update_general(general)
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    fn sync_theme(&mut self) {
        let mode = self
            .state
            .settings
            .configuration
            .snapshot()
            .map(|s| s.general.theme)
            .unwrap_or_else(|_| "system".into());
        if self.appearance.as_ref() != Some(&mode) {
            self.window().set_theme(match mode.as_str() {
                "dark" => Some(Theme::Dark),
                "light" => Some(Theme::Light),
                _ => None,
            });
            self.appearance = Some(mode);
        }
        let dark = self.window().theme() == Some(Theme::Dark);
        self.backend.0.borrow_mut().set_system_dark(dark);
    }
    pub(crate) fn user_event(&mut self, event_loop: &ActiveEventLoop, event: HostEvent) {
        self.sync_theme();
        self.host.user_event(event_loop, event);
    }
    pub(crate) fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        id: WindowId,
        event: WindowEvent,
    ) {
        if id != self.window_id() {
            return;
        }
        if matches!(event, WindowEvent::CloseRequested) {
            if let Err(error) = self.remember_geometry() {
                log::warn!("Failed to save Settings geometry: {error}");
            }
        }
        if let WindowEvent::ThemeChanged(theme) = &event {
            self.backend
                .0
                .borrow_mut()
                .set_system_dark(*theme == Theme::Dark);
            self.host.refresh();
        }
        self.host.window_event(event_loop, id, event);
    }
    pub(crate) fn take_failure(&mut self) -> Option<String> {
        self.host.take_failure()
    }
}

#[cfg(test)]
impl NativeSettingsWindow {
    pub(crate) fn test_page(&self) -> Page {
        self.host.page()
    }
    pub(crate) fn test_input(&mut self, input: snaplingo_ui::Event) {
        self.host.input(input);
    }
    pub(crate) fn test_navigate(&mut self, page: Page) -> Result<(), String> {
        self.host.input(snaplingo_ui::Event::Activate(
            snaplingo_ui::Target::Navigate(page),
        ));
        Ok(())
    }
    pub(crate) fn test_interface_scale(&self) -> f32 {
        let window = self.window();
        let display = window
            .current_monitor()
            .map(|monitor| monitor.size().to_logical::<f32>(monitor.scale_factor()))
            .unwrap_or_else(|| window.inner_size().to_logical::<f32>(window.scale_factor()));
        self.backend.interface_scale(Size {
            width: display.width,
            height: display.height,
        })
    }
}

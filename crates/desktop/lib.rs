//! Native window/event adapter for shared preferences, Services and Library.
//! Application backends supply snapshots and execute the resulting intents.
use std::{num::NonZeroU32, rc::Rc, sync::Arc};
mod accessibility;
mod dialogs;
pub use dialogs::choose_color;
pub mod settings_backend;
use accessibility::Accessibility;
use accesskit_winit::{Event as AccessibilityEvent, WindowEvent as AccessibilityWindowEvent};

use snaplingo_render::{Renderer, RendererTextMetrics};
use snaplingo_ui::{
    library::LibraryIntent, services::ServicesIntent, Action, Event, GeneralCommand, Key, KeyStyle,
    Modifiers, Page, Point, Size, Snapshot, WindowInput, WindowIntent, WindowSnapshot, WindowUi,
};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalPosition, LogicalSize},
    event::{ElementState, Ime, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    keyboard::{Key as NativeKey, KeyCode, NamedKey, PhysicalKey},
    window::{Window, WindowAttributes, WindowId},
};

pub use settings_backend::WindowBackend;

/// Events shared with a resident application that embeds the settings window.
pub enum HostEvent {
    Accessibility(AccessibilityEvent),
    Refresh,
    /// Wake the application's queue of native UI work.
    Wake,
}
impl From<AccessibilityEvent> for HostEvent {
    fn from(event: AccessibilityEvent) -> Self {
        Self::Accessibility(event)
    }
}

/// Owns one event loop. It sleeps between OS/service events; pointer, keyboard,
/// focus, wheel and DPI coordinates cross the same logical-DIP UI interface.
pub fn run_window_backend(backend: impl WindowBackend + 'static) -> Result<(), String> {
    let event_loop = EventLoop::<HostEvent>::with_user_event()
        .build()
        .map_err(|error| error.to_string())?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let proxy = event_loop.create_proxy();
    let mut host = WindowHost::new(
        backend,
        proxy,
        Window::default_attributes()
            .with_title("SnapLingo")
            .with_inner_size(LogicalSize::new(900.0, 650.0))
            .with_min_inner_size(LogicalSize::new(700.0, 500.0)),
        false,
    )?;
    event_loop
        .run_app(&mut host)
        .map_err(|error| error.to_string())?;
    host.failure.take().map_or(Ok(()), Err)
}

/// Shared canvas window embedded in an application-owned winit event loop.
/// Its ApplicationHandler methods may be forwarded by the application alongside
/// capture, result and pinned windows. It never creates a second event loop.
pub struct WindowHost {
    ui: WindowUi,
    backend: Box<dyn WindowBackend>,
    renderer: Renderer,
    metrics: RendererTextMetrics,
    // Native accessibility/window subclasses and the presenter are released
    // before the Window's final owner.
    accessibility: Option<Accessibility>,
    surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
    window: Option<Rc<Window>>,
    cursor: Point,
    modifiers: Modifiers,
    failure: Option<String>,
    ime_enabled: bool,
    ime_composing: bool,
    proxy: EventLoopProxy<HostEvent>,
    attributes: WindowAttributes,
    close_to_hide: bool,
}

impl WindowHost {
    pub fn new(
        mut backend: impl WindowBackend + 'static,
        proxy: EventLoopProxy<HostEvent>,
        attributes: WindowAttributes,
        close_to_hide: bool,
    ) -> Result<Self, String> {
        let initial = platform_window_snapshot(backend.snapshot()?);
        let refresh = proxy.clone();
        backend.set_notifier(Some(Arc::new(move || {
            let _ = refresh.send_event(HostEvent::Refresh);
        })));
        Ok(Self {
            ui: WindowUi::new(initial),
            backend: Box::new(backend),
            renderer: Renderer::default(),
            metrics: RendererTextMetrics::default(),
            window: None,
            surface: None,
            cursor: Point::default(),
            modifiers: Modifiers::default(),
            failure: None,
            ime_enabled: false,
            ime_composing: false,
            accessibility: None,
            proxy,
            attributes,
            close_to_hide,
        })
    }

    pub fn window(&self) -> Option<&Window> {
        self.window.as_deref()
    }
    pub fn window_rc(&self) -> Option<Rc<Window>> {
        self.window.clone()
    }

    pub fn show(&mut self) {
        self.refresh();
        if let Some(window) = &self.window {
            window.set_visible(true);
            window.focus_window();
        }
    }

    pub fn hide(&mut self) {
        self.input(Event::FocusLost);
        if let Some(window) = &self.window {
            window.set_visible(false);
        }
    }

    pub fn refresh(&mut self) {
        self.ime_composing = refresh_ui(&mut self.ui, self.backend.as_ref(), None, None);
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    /// Apply an application route, such as a tray or global-shortcut action.
    pub fn navigate(&mut self, page: Page) -> Result<(), String> {
        self.backend.dispatch(
            self.ui.page(),
            WindowIntent::Settings(Action::Navigate(page)),
        )?;
        self.ime_composing = refresh_ui(&mut self.ui, self.backend.as_ref(), None, Some(page));
        if let Some(window) = &self.window {
            window.request_redraw();
        }
        Ok(())
    }

    pub fn activate_section(&mut self, section: snaplingo_ui::Section) {
        self.input(Event::Activate(snaplingo_ui::Target::Section(section)));
    }

    pub fn recorded_shortcut(&mut self, shortcut: String) {
        self.input(Event::RecordedShortcut(shortcut));
    }

    pub fn take_failure(&mut self) -> Option<String> {
        self.failure.take()
    }

    /// Normalized application input uses the same path as native/AX events.
    pub fn input(&mut self, event: Event) {
        self.window_input(WindowInput::Event(event));
    }

    pub fn page(&self) -> Page {
        self.ui.page()
    }

    fn window_input(&mut self, input: WindowInput) {
        let page = self.ui.page();
        let actions = self.ui.input(input);
        if !actions.is_empty() {
            let mut error = None;
            let mut refresh = false;
            let mut navigation = None;
            for intent in actions {
                match &intent {
                    WindowIntent::Settings(Action::CopyToClipboard(text))
                    | WindowIntent::Services(ServicesIntent::CopyText(text))
                    | WindowIntent::Library(LibraryIntent::CopyText(text)) => {
                        if let Err(message) = arboard::Clipboard::new()
                            .and_then(|mut clipboard| clipboard.set_text(text))
                        {
                            error = Some(message.to_string());
                        }
                        continue;
                    }
                    WindowIntent::Settings(Action::ReadClipboard)
                    | WindowIntent::Services(ServicesIntent::ReadClipboard)
                    | WindowIntent::Library(LibraryIntent::ReadClipboard) => {
                        match arboard::Clipboard::new()
                            .and_then(|mut clipboard| clipboard.get_text())
                        {
                            Ok(text) => self.input(Event::ClipboardText(text)),
                            Err(message) => error = Some(message.to_string()),
                        }
                        continue;
                    }
                    WindowIntent::Settings(Action::SelectScreenshotDirectory) => {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_title("选择截图保存位置")
                            .pick_folder()
                        {
                            self.input(Event::SetValue(
                                snaplingo_ui::Target::Screenshot(
                                    snaplingo_ui::ScreenshotField::SavePath,
                                ),
                                snaplingo_ui::Value::Text(path.to_string_lossy().into_owned()),
                            ));
                        }
                        continue;
                    }
                    WindowIntent::Settings(Action::SelectScreenshotColor { field, rgba }) => {
                        match dialogs::choose_color(*rgba) {
                            Ok(Some(color)) => self.input(Event::SetValue(
                                snaplingo_ui::Target::Screenshot(*field),
                                snaplingo_ui::Value::Number(u32::from_be_bytes(color)),
                            )),
                            Ok(None) => {}
                            Err(message) => error = Some(message),
                        }
                        continue;
                    }
                    _ => {}
                }
                if let Some(message) = confirmation(&intent) {
                    if rfd::MessageDialog::new()
                        .set_title("确认操作")
                        .set_description(message)
                        .set_buttons(rfd::MessageButtons::OkCancel)
                        .show()
                        != rfd::MessageDialogResult::Ok
                    {
                        continue;
                    }
                }
                let destination = navigation_page(&intent);
                if let Err(message) = self.backend.dispatch(page, intent) {
                    error = Some(message);
                } else if destination.is_some() {
                    navigation = destination;
                }
                refresh = true;
            }
            if refresh || error.is_some() {
                refresh_ui(&mut self.ui, self.backend.as_ref(), error, navigation);
            }
        }
        self.ime_composing = self.ui.text_edit().is_some_and(|edit| edit.is_composing());
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn draw(&mut self) -> Result<(), String> {
        let Some(window) = &self.window else {
            return Ok(());
        };
        let physical = window.inner_size();
        let (Some(width), Some(height)) = (
            NonZeroU32::new(physical.width),
            NonZeroU32::new(physical.height),
        ) else {
            return Ok(());
        };
        let device_scale = window.scale_factor();
        let logical = physical.to_logical::<f32>(device_scale);
        let display = window
            .current_monitor()
            .map(|monitor| monitor.size().to_logical::<f32>(monitor.scale_factor()));
        let display = display.unwrap_or(logical);
        let frame = self.ui.frame(
            Size {
                width: logical.width,
                height: logical.height,
            },
            self.backend.interface_scale(Size {
                width: display.width,
                height: display.height,
            }),
            &mut self.metrics,
        );
        let scene = &frame.scene;
        if self.ime_enabled != scene.ime_cursor.is_some() {
            self.ime_enabled = scene.ime_cursor.is_some();
            window.set_ime_allowed(self.ime_enabled);
        }
        if let Some(caret) = scene.ime_cursor {
            window.set_ime_cursor_area(
                LogicalPosition::new(caret.x, caret.y),
                LogicalSize::new(caret.width, caret.height),
            );
        }
        let pixels = self.renderer.render(scene, device_scale as f32)?;
        if let Some(accessibility) = &mut self.accessibility {
            accessibility.update(&frame, &self.ui, device_scale);
        }
        let surface = self
            .surface
            .as_mut()
            .ok_or("Missing native pixel presenter")?;
        surface
            .resize(width, height)
            .map_err(|error| error.to_string())?;
        let mut buffer = surface.buffer_mut().map_err(|error| error.to_string())?;
        pixels.write_opaque_rgb(&mut buffer)?;
        buffer.present().map_err(|error| error.to_string())
    }
}

impl ApplicationHandler<HostEvent> for WindowHost {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let result = (|| {
            let window = Rc::new(
                event_loop
                    .create_window(self.attributes.clone().with_visible(false))
                    .map_err(|error| error.to_string())?,
            );
            let context =
                softbuffer::Context::new(window.clone()).map_err(|error| error.to_string())?;
            self.surface = Some(
                softbuffer::Surface::new(&context, window.clone())
                    .map_err(|error| error.to_string())?,
            );
            let physical = window.inner_size();
            let logical = physical.to_logical::<f32>(window.scale_factor());
            let display = window
                .current_monitor()
                .map(|monitor| monitor.size().to_logical::<f32>(monitor.scale_factor()))
                .unwrap_or(logical);
            let frame = self.ui.frame(
                Size {
                    width: logical.width,
                    height: logical.height,
                },
                self.backend.interface_scale(Size {
                    width: display.width,
                    height: display.height,
                }),
                &mut self.metrics,
            );
            self.accessibility = Some(Accessibility::new(
                event_loop,
                &window,
                self.proxy.clone(),
                &frame,
                &self.ui,
                window.scale_factor(),
            ));
            window.set_visible(true);
            window.request_redraw();
            self.window = Some(window);
            Ok::<_, String>(())
        })();
        if let Err(error) = result {
            self.failure = Some(error);
            event_loop.exit();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let Some(window) = &self.window else {
            return;
        };
        if window.id() != id {
            return;
        }
        if let Some(accessibility) = &mut self.accessibility {
            accessibility.process_event(window, &event);
        }
        match event {
            WindowEvent::CloseRequested => {
                if self.close_to_hide {
                    self.hide();
                } else {
                    self.input(Event::FocusLost);
                    event_loop.exit();
                }
            }
            WindowEvent::RedrawRequested => {
                if let Err(error) = self.draw() {
                    self.failure = Some(error);
                    event_loop.exit();
                }
            }
            WindowEvent::Resized(size) => {
                let size = size.to_logical::<f32>(window.scale_factor());
                self.input(Event::Resize(Size {
                    width: size.width,
                    height: size.height,
                }));
            }
            WindowEvent::ScaleFactorChanged { .. } => window.request_redraw(),
            WindowEvent::Focused(false) => self.input(Event::FocusLost),
            WindowEvent::CursorMoved { position, .. } => {
                let position = position.to_logical::<f32>(window.scale_factor());
                self.cursor = Point {
                    x: position.x,
                    y: position.y,
                };
                self.input(Event::PointerMove(self.cursor));
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => self.input(Event::PointerDown(self.cursor)),
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => self.input(Event::PointerUp(self.cursor)),
            WindowEvent::MouseWheel { delta, .. } => {
                let delta_y = match delta {
                    MouseScrollDelta::LineDelta(_, y) => -y * 32.0,
                    MouseScrollDelta::PixelDelta(p) => -(p.y / window.scale_factor()) as f32,
                };
                self.input(Event::Scroll {
                    position: self.cursor,
                    delta_y,
                });
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                let m = modifiers.state();
                self.modifiers = Modifiers {
                    shift: m.shift_key(),
                    alt: m.alt_key(),
                    control: m.control_key(),
                    meta: m.super_key(),
                };
            }
            WindowEvent::Ime(ime) => {
                let input = match ime {
                    Ime::Preedit(text, selection) => {
                        self.ime_composing = !text.is_empty();
                        Some(Event::ImePreedit {
                            text,
                            selection: selection.map(|(start, end)| start..end),
                        })
                    }
                    Ime::Commit(text) => {
                        self.ime_composing = false;
                        Some(Event::ImeCommit(text))
                    }
                    Ime::Disabled => {
                        self.ime_composing = false;
                        Some(Event::ImeDisabled)
                    }
                    Ime::Enabled => None,
                };
                if let Some(input) = input {
                    self.input(input);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if self.ime_composing {
                    return;
                }
                if self.ime_enabled
                    && event.state.is_pressed()
                    && !self.modifiers.meta
                    && !self.modifiers.control
                {
                    if let Some(text) = event
                        .text
                        .as_ref()
                        .filter(|text| text.chars().all(|c| !c.is_control()))
                    {
                        self.input(Event::TextInput(text.to_string()));
                        return;
                    }
                }
                if let Some(key) = key(&event.physical_key, &event.logical_key) {
                    self.input(if event.state.is_pressed() {
                        Event::KeyDown {
                            key,
                            modifiers: self.modifiers,
                            repeat: event.repeat,
                        }
                    } else {
                        Event::KeyUp { key }
                    });
                }
            }
            _ => {}
        }
    }
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: HostEvent) {
        if matches!(event, HostEvent::Wake) {
            return;
        }
        let HostEvent::Accessibility(event) = event else {
            self.refresh();
            return;
        };
        if self
            .window
            .as_ref()
            .is_none_or(|window| window.id() != event.window_id)
        {
            return;
        }
        match event.window_event {
            AccessibilityWindowEvent::ActionRequested(request) => {
                let events = self
                    .accessibility
                    .as_ref()
                    .map(|accessibility| accessibility.action(&request))
                    .unwrap_or_default();
                for event in events {
                    self.window_input(event);
                }
            }
            AccessibilityWindowEvent::InitialTreeRequested => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            AccessibilityWindowEvent::AccessibilityDeactivated => {
                if let Some(accessibility) = &mut self.accessibility {
                    accessibility.deactivated();
                }
            }
        }
    }
}

impl Drop for WindowHost {
    fn drop(&mut self) {
        self.backend.set_notifier(None);
    }
}

fn platform_snapshot(mut snapshot: Snapshot) -> Snapshot {
    snapshot.key_style = if cfg!(target_os = "windows") {
        KeyStyle::Windows
    } else {
        KeyStyle::Mac
    };
    snapshot
}

fn platform_window_snapshot(snapshot: WindowSnapshot) -> WindowSnapshot {
    match snapshot {
        WindowSnapshot::Settings(snapshot) => WindowSnapshot::Settings(platform_snapshot(snapshot)),
        WindowSnapshot::Services(mut snapshot) => {
            snapshot.shell = platform_snapshot(snapshot.shell);
            WindowSnapshot::Services(snapshot)
        }
        WindowSnapshot::Library(mut snapshot) => {
            snapshot.shell = platform_snapshot(snapshot.shell);
            WindowSnapshot::Library(snapshot)
        }
    }
}

fn current_snapshot(ui: &WindowUi) -> WindowSnapshot {
    match ui {
        WindowUi::Settings(ui) => WindowSnapshot::Settings(ui.snapshot().clone()),
        WindowUi::Services(ui) => WindowSnapshot::Services(ui.snapshot().clone()),
        WindowUi::Library(ui) => WindowSnapshot::Library(ui.snapshot().clone()),
    }
}

fn set_error(snapshot: &mut WindowSnapshot, error: Option<String>) {
    if let Some(error) = error {
        match snapshot {
            WindowSnapshot::Settings(snapshot) => snapshot.error = Some(error),
            WindowSnapshot::Services(snapshot) => snapshot.error = Some(error),
            WindowSnapshot::Library(snapshot) => snapshot.error = Some(error),
        }
    }
}

fn refresh_ui(
    ui: &mut WindowUi,
    backend: &dyn WindowBackend,
    error: Option<String>,
    navigation: Option<Page>,
) -> bool {
    match backend.snapshot() {
        Ok(snapshot) => {
            let mut snapshot = platform_window_snapshot(snapshot);
            set_error(&mut snapshot, error);
            ui.snapshot(snapshot);
            if let Some(page) = navigation {
                if ui.preferences().is_some() {
                    ui.update(Event::Activate(snaplingo_ui::Target::Navigate(page)));
                }
            }
        }
        Err(message) => {
            let mut snapshot = current_snapshot(ui);
            set_error(&mut snapshot, Some(message));
            ui.snapshot(snapshot);
        }
    }
    ui.text_edit().is_some_and(|edit| edit.is_composing())
}

fn navigation_page(intent: &WindowIntent) -> Option<Page> {
    match intent {
        WindowIntent::Settings(Action::Navigate(page))
        | WindowIntent::Services(ServicesIntent::Navigate(page)) => Some(*page),
        _ => None,
    }
}

fn confirmation(intent: &WindowIntent) -> Option<&'static str> {
    match intent {
        WindowIntent::Settings(Action::ResetHotkeys) => Some("确定要恢复当前分类的默认快捷键吗？"),
        WindowIntent::Settings(Action::GeneralCommand(command)) => match command {
            GeneralCommand::ClearLogs => Some("确定要清空所有应用日志吗？"),
            GeneralCommand::ClearHistory => Some("确定要清空全部历史记录吗？独立收藏不会被删除。"),
            GeneralCommand::ResetScreenRecording => Some("确定要重置 SnapLingo 的屏幕录制权限吗？"),
            GeneralCommand::ResetAccessibility => Some("确定要重置 SnapLingo 的辅助功能权限吗？"),
            _ => None,
        },
        WindowIntent::Services(ServicesIntent::ConfirmRemove(_)) => {
            Some("确定要删除这个翻译服务吗？")
        }
        WindowIntent::Library(LibraryIntent::ConfirmClearHistory(_)) => {
            Some("确定要清空当前分类的历史记录吗？独立收藏不会被删除。")
        }
        _ => None,
    }
}

fn key(physical: &PhysicalKey, key: &NativeKey) -> Option<Key> {
    if let PhysicalKey::Code(code) = physical {
        let letter = match code {
            KeyCode::KeyA => 'A',
            KeyCode::KeyB => 'B',
            KeyCode::KeyC => 'C',
            KeyCode::KeyD => 'D',
            KeyCode::KeyE => 'E',
            KeyCode::KeyF => 'F',
            KeyCode::KeyG => 'G',
            KeyCode::KeyH => 'H',
            KeyCode::KeyI => 'I',
            KeyCode::KeyJ => 'J',
            KeyCode::KeyK => 'K',
            KeyCode::KeyL => 'L',
            KeyCode::KeyM => 'M',
            KeyCode::KeyN => 'N',
            KeyCode::KeyO => 'O',
            KeyCode::KeyP => 'P',
            KeyCode::KeyQ => 'Q',
            KeyCode::KeyR => 'R',
            KeyCode::KeyS => 'S',
            KeyCode::KeyT => 'T',
            KeyCode::KeyU => 'U',
            KeyCode::KeyV => 'V',
            KeyCode::KeyW => 'W',
            KeyCode::KeyX => 'X',
            KeyCode::KeyY => 'Y',
            KeyCode::KeyZ => 'Z',
            KeyCode::Digit0 => '0',
            KeyCode::Digit1 => '1',
            KeyCode::Digit2 => '2',
            KeyCode::Digit3 => '3',
            KeyCode::Digit4 => '4',
            KeyCode::Digit5 => '5',
            KeyCode::Digit6 => '6',
            KeyCode::Digit7 => '7',
            KeyCode::Digit8 => '8',
            KeyCode::Digit9 => '9',
            _ => '\0',
        };
        if letter != '\0' {
            return Some(Key::Character(letter));
        }
    }
    Some(match key {
        NativeKey::Character(text) => Key::Character(text.chars().next()?.to_ascii_uppercase()),
        NativeKey::Named(key) => match key {
            NamedKey::Tab => Key::Tab,
            NamedKey::Enter => Key::Enter,
            NamedKey::Space => Key::Space,
            NamedKey::Escape => Key::Escape,
            NamedKey::ArrowUp => Key::ArrowUp,
            NamedKey::ArrowDown => Key::ArrowDown,
            NamedKey::ArrowLeft => Key::ArrowLeft,
            NamedKey::ArrowRight => Key::ArrowRight,
            NamedKey::Home => Key::Home,
            NamedKey::End => Key::End,
            NamedKey::PageUp => Key::PageUp,
            NamedKey::PageDown => Key::PageDown,
            NamedKey::Backspace => Key::Backspace,
            NamedKey::Delete => Key::Delete,
            NamedKey::F1 => Key::Function(1),
            NamedKey::F2 => Key::Function(2),
            NamedKey::F3 => Key::Function(3),
            NamedKey::F4 => Key::Function(4),
            NamedKey::F5 => Key::Function(5),
            NamedKey::F6 => Key::Function(6),
            NamedKey::F7 => Key::Function(7),
            NamedKey::F8 => Key::Function(8),
            NamedKey::F9 => Key::Function(9),
            NamedKey::F10 => Key::Function(10),
            NamedKey::F11 => Key::Function(11),
            NamedKey::F12 => Key::Function(12),
            NamedKey::F13 => Key::Function(13),
            NamedKey::F14 => Key::Function(14),
            NamedKey::F15 => Key::Function(15),
            NamedKey::F16 => Key::Function(16),
            NamedKey::F17 => Key::Function(17),
            NamedKey::F18 => Key::Function(18),
            NamedKey::F19 => Key::Function(19),
            NamedKey::F20 => Key::Function(20),
            NamedKey::F21 => Key::Function(21),
            NamedKey::F22 => Key::Function(22),
            NamedKey::F23 => Key::Function(23),
            NamedKey::F24 => Key::Function(24),
            _ => return None,
        },
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn async_backend_notification_refreshes_the_active_dynamic_page_and_can_be_released() {
        use snaplingo_ui::services::{ServicesRoute, ServicesSnapshot};
        use std::sync::{mpsc, Mutex};
        use std::time::Duration;

        struct Backend {
            snapshot: Arc<Mutex<ServicesSnapshot>>,
            notifier: Arc<Mutex<Option<Arc<dyn Fn() + Send + Sync>>>>,
        }
        impl WindowBackend for Backend {
            fn snapshot(&self) -> Result<WindowSnapshot, String> {
                Ok(WindowSnapshot::Services(
                    self.snapshot.lock().unwrap().clone(),
                ))
            }
            fn dispatch(&mut self, _: Page, _: WindowIntent) -> Result<(), String> {
                Ok(())
            }
            fn set_notifier(&mut self, notifier: Option<Arc<dyn Fn() + Send + Sync>>) {
                *self.notifier.lock().unwrap() = notifier;
            }
        }
        let snapshot = Arc::new(Mutex::new(ServicesSnapshot {
            route: ServicesRoute::Tts,
            rate: 180,
            ..Default::default()
        }));
        let notifier = Arc::new(Mutex::new(None));
        let mut backend = Backend {
            snapshot: snapshot.clone(),
            notifier: notifier.clone(),
        };
        let mut ui = WindowUi::new(backend.snapshot().unwrap());
        let (sender, receiver) = mpsc::channel();
        backend.set_notifier(Some(Arc::new(move || {
            sender.send(()).unwrap();
        })));
        std::thread::spawn(move || {
            let mut snapshot = snapshot.lock().unwrap();
            snapshot.rate = 230;
            snapshot.revision += 1;
            snapshot.status = Some("服务更新完成".into());
            drop(snapshot);
            notifier.lock().unwrap().as_ref().unwrap()();
        })
        .join()
        .unwrap();
        receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("An async result must wake the idle host");
        refresh_ui(&mut ui, &backend, None, None);
        let WindowUi::Services(ui) = &ui else {
            panic!()
        };
        assert_eq!(ui.snapshot().rate, 230);
        assert_eq!(ui.snapshot().status.as_deref(), Some("服务更新完成"));
        let callback = Arc::new(());
        let lifetime = Arc::downgrade(&callback);
        backend.set_notifier(Some(Arc::new(move || {
            let _ = &callback;
        })));
        backend.set_notifier(None);
        assert!(
            lifetime.upgrade().is_none(),
            "Closing the native host must release its notification callback"
        );
    }

    #[test]
    fn async_route_refresh_clears_the_native_composition_gate() {
        use snaplingo_ui::services::*;
        struct Backend;
        impl WindowBackend for Backend {
            fn snapshot(&self) -> Result<WindowSnapshot, String> {
                Ok(WindowSnapshot::Services(ServicesSnapshot {
                    route: ServicesRoute::Tts,
                    revision: 2,
                    form_revision: 2,
                    ..Default::default()
                }))
            }
            fn dispatch(&mut self, _: Page, _: WindowIntent) -> Result<(), String> {
                Ok(())
            }
        }
        let mut ui = WindowUi::new(WindowSnapshot::Services(ServicesSnapshot {
            route: ServicesRoute::Translation,
            revision: 1,
            form_revision: 1,
            form: Some(ServiceForm::Custom {
                draft: ServiceCustomDraft::default(),
                prompts: ServicePromptWorkspace::default(),
            }),
            ..Default::default()
        }));
        let frame = ui.frame(
            Size {
                width: 900.,
                height: 650.,
            },
            1.,
            &mut RendererTextMetrics::default(),
        );
        let snaplingo_ui::WindowControls::Services(controls) = frame.controls else {
            panic!()
        };
        let field = controls
            .iter()
            .find(|control| {
                control.target.control
                    == ServiceControl::Field(ServiceField::Custom(ServiceCustomField::Name))
            })
            .unwrap()
            .target
            .clone();
        ui.services_input(ServicesInput::Focus(field));
        ui.services_input(ServicesInput::ImePreedit {
            text: "拼音".into(),
            selection: None,
        });
        assert!(ui.text_edit().unwrap().is_composing());
        assert!(
            !refresh_ui(&mut ui, &Backend, None, None),
            "A disposed form must release the host's keyboard composition gate"
        );
        assert!(ui.text_edit().is_none());
    }

    #[test]
    fn shortcut_identity_survives_modifier_release_and_shifted_digits() {
        let physical = PhysicalKey::Code(KeyCode::KeyA);
        assert_eq!(
            key(&physical, &NativeKey::Character("A".into())),
            key(&physical, &NativeKey::Character("a".into()))
        );
        assert_eq!(
            key(
                &PhysicalKey::Code(KeyCode::Digit1),
                &NativeKey::Character("!".into())
            ),
            Some(Key::Character('1'))
        );
        assert_eq!(
            key(
                &PhysicalKey::Code(KeyCode::F18),
                &NativeKey::Named(NamedKey::F18)
            ),
            Some(Key::Function(18))
        );
    }
}

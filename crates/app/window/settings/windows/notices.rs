//! Native Windows inventory window using shared search and license drawing.
use crate::infrastructure::ui_thread;
use accesskit::{ActivationHandler, TreeUpdate};
use accesskit_winit::{Adapter, WindowEvent as AccessibilityEvent};
use serde::Deserialize;
use snaplingo_accessibility::{notices_project, NoticesProjection};
use snaplingo_render::{Renderer, RendererTextMetrics};
use snaplingo_ui::{notices::*, Key, KeyStyle, Modifiers, Point, Size};
use std::{
    cell::RefCell,
    collections::HashMap,
    num::NonZeroU32,
    rc::Rc,
    sync::{Arc, Mutex},
};
use winit::{
    dpi::{LogicalPosition, LogicalSize},
    event::{ElementState, Ime, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{Key as NativeKey, NamedKey},
    platform::windows::WindowAttributesExtWindows,
    raw_window_handle::{HasWindowHandle, RawWindowHandle},
    window::{Window, WindowId},
};
#[derive(Deserialize)]
struct Component {
    name: String,
    version: String,
    license: String,
    source: String,
    notices: Vec<String>,
}
#[derive(Deserialize)]
struct Inventory {
    components: Vec<Component>,
    texts: HashMap<String, String>,
}
fn inventory() -> Vec<NoticeComponent> {
    let inventory: Inventory = serde_json::from_str(include_str!(
        "../../../../../assets/licenses/thirdPartyNotices.json"
    ))
    .expect("Bundled third-party notices are valid");
    inventory
        .components
        .into_iter()
        .map(|component| {
            let text = if component.notices.is_empty() {
                format!(
                    "该依赖的许可标识为 {}，完整许可请查看来源仓库。",
                    component.license
                )
            } else {
                component
                    .notices
                    .iter()
                    .map(|id| inventory.texts[id].as_str())
                    .collect::<Vec<_>>()
                    .join("\n\n")
            };
            NoticeComponent {
                name: component.name,
                version: component.version,
                license: component.license,
                source: component.source,
                text,
            }
        })
        .collect()
}
thread_local! {static VIEW:RefCell<Option<NoticesView>>=const {RefCell::new(None)};}
struct CachedTree(Arc<Mutex<NoticesProjection>>);
impl ActivationHandler for CachedTree {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        Some(self.0.lock().unwrap().update(None))
    }
}
struct NoticesView {
    parent: WindowId,
    surface: softbuffer::Surface<Rc<Window>, Rc<Window>>,
    ui: NoticesUi,
    renderer: Renderer,
    metrics: RendererTextMetrics,
    scale: f32,
    adapter: Adapter,
    projection: Arc<Mutex<NoticesProjection>>,
    sent: Option<NoticesProjection>,
    cursor: Point,
    modifiers: Modifiers,
    ime_enabled: bool,
    window: Rc<Window>,
}
pub(crate) fn show(
    event_loop: &ActiveEventLoop,
    parent: Rc<Window>,
    state: Arc<crate::AppState>,
) -> Result<(), String> {
    close_for(parent.id());
    let saved = state
        .settings
        .configuration
        .snapshot()
        .map_err(|e| e.to_string())?;
    let mut shell = snaplingo_settings::project_snapshot(
        &saved,
        &state
            .settings
            .hotkeys
            .snapshot()
            .map_err(|e| e.to_string())?,
        &state.settings.hotkeys.default_snapshot(),
    );
    shell.key_style = KeyStyle::Windows;
    shell.dark = saved.general.theme == "dark"
        || saved.general.theme == "system" && parent.theme() == Some(winit::window::Theme::Dark);
    let scale =
        crate::application::capture::interface_scale(&saved.screenshot.capture_ui_scale, None)
            as f32;
    let handle = parent.window_handle().map_err(|e| e.to_string())?;
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return Err("Notices owner requires a Win32 window".into());
    };
    let mut attributes = Window::default_attributes()
        .with_title("第三方组件与许可")
        .with_owner_window(handle.hwnd.get())
        .with_visible(false)
        .with_inner_size(LogicalSize::new(720. * scale, 650. * scale))
        .with_min_inner_size(LogicalSize::new(600., 420.));
    if let Ok(position) = parent.outer_position() {
        let position = position.to_logical::<f64>(parent.scale_factor());
        attributes =
            attributes.with_position(LogicalPosition::new(position.x + 30., position.y + 30.));
    }
    let window = Rc::new(
        event_loop
            .create_window(attributes)
            .map_err(|e| e.to_string())?,
    );
    let context = softbuffer::Context::new(window.clone()).map_err(|e| e.to_string())?;
    let surface = softbuffer::Surface::new(&context, window.clone()).map_err(|e| e.to_string())?;
    let mut ui = NoticesUi::new(shell, inventory());
    let mut metrics = RendererTextMetrics::default();
    let frame = ui.frame(
        Size {
            width: 720. * scale,
            height: 650. * scale,
        },
        scale,
        &mut metrics,
    );
    let projection = Arc::new(Mutex::new(notices_project(
        &frame,
        &ui,
        window.scale_factor(),
        1,
    )));
    let adapter = Adapter::with_mixed_handlers(
        event_loop,
        &window,
        CachedTree(projection.clone()),
        ui_thread::event_proxy()?,
    );
    window.set_visible(true);
    window.focus_window();
    window.request_redraw();
    VIEW.with_borrow_mut(|slot| {
        *slot = Some(NoticesView {
            parent: parent.id(),
            window,
            surface,
            ui,
            renderer: Renderer::default(),
            metrics,
            scale,
            adapter,
            projection,
            sent: None,
            cursor: Point::default(),
            modifiers: Modifiers::default(),
            ime_enabled: false,
        })
    });
    Ok(())
}
pub(crate) fn close_for(parent: WindowId) {
    let view = VIEW.with_borrow_mut(|slot| {
        if slot.as_ref().is_some_and(|view| view.parent == parent) {
            slot.take()
        } else {
            None
        }
    });
    drop(view);
}
impl NoticesView {
    fn input(&mut self, input: NoticesInput) -> bool {
        for intent in self.ui.update(input) {
            match intent {
                NoticesIntent::Close => return true,
                NoticesIntent::OpenSource(url) => {
                    if let Err(error) = open::that_detached(&url) {
                        log::warn!("Failed to open component source: {error}");
                    }
                }
                NoticesIntent::Copy(text) => {
                    if let Err(error) =
                        crate::infrastructure::system::clipboard_text::copy_text(text)
                    {
                        log::warn!("Failed to copy license text: {error}");
                    }
                }
                NoticesIntent::ReadClipboard => {
                    if let Ok(text) = arboard::Clipboard::new().and_then(|mut c| c.get_text()) {
                        self.input(NoticesInput::Text(text));
                    }
                }
            }
        }
        self.window.request_redraw();
        false
    }
    fn draw(&mut self) -> Result<(), String> {
        let physical = self.window.inner_size();
        let (Some(width), Some(height)) = (
            NonZeroU32::new(physical.width),
            NonZeroU32::new(physical.height),
        ) else {
            return Ok(());
        };
        let size = physical.to_logical::<f32>(self.window.scale_factor());
        let frame = self.ui.frame(
            Size {
                width: size.width,
                height: size.height,
            },
            self.scale,
            &mut self.metrics,
        );
        let next = notices_project(&frame, &self.ui, self.window.scale_factor(), 1);
        *self.projection.lock().unwrap() = next.clone();
        if self.sent.as_ref() != Some(&next) {
            let mut sent = false;
            self.adapter.update_if_active(|| {
                sent = true;
                next.update(self.sent.as_ref())
            });
            if sent {
                self.sent = Some(next);
            }
        }
        let enable = frame.scene.ime_cursor.is_some();
        if self.ime_enabled != enable {
            self.ime_enabled = enable;
            self.window.set_ime_allowed(enable);
        }
        if let Some(caret) = frame.scene.ime_cursor {
            self.window.set_ime_cursor_area(
                LogicalPosition::new(caret.x, caret.y),
                LogicalSize::new(caret.width, caret.height),
            );
        }
        let pixels = self
            .renderer
            .render(&frame.scene, self.window.scale_factor() as f32)?;
        self.surface
            .resize(width, height)
            .map_err(|e| e.to_string())?;
        let mut buffer = self.surface.buffer_mut().map_err(|e| e.to_string())?;
        pixels.write_opaque_rgb(&mut buffer)?;
        buffer.present().map_err(|e| e.to_string())
    }
}
pub(crate) fn window_event(_event_loop: &ActiveEventLoop, id: WindowId, event: &WindowEvent) {
    let close = VIEW.with_borrow_mut(|slot| {
        let Some(view) = slot.as_mut().filter(|view| view.window.id() == id) else {
            return false;
        };
        view.adapter.process_event(&view.window, event);
        match event {
            WindowEvent::CloseRequested => true,
            WindowEvent::RedrawRequested => {
                if let Err(error) = view.draw() {
                    log::warn!("Failed to draw notices: {error}");
                }
                false
            }
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                view.window.request_redraw();
                false
            }
            WindowEvent::CursorMoved { position, .. } => {
                let p = position.to_logical::<f32>(view.window.scale_factor());
                view.cursor = Point { x: p.x, y: p.y };
                view.input(NoticesInput::PointerMove(view.cursor))
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => view.input(NoticesInput::PointerDown(view.cursor)),
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => view.input(NoticesInput::PointerUp),
            WindowEvent::MouseWheel { delta, .. } => {
                view.input(NoticesInput::Scroll(match delta {
                    MouseScrollDelta::LineDelta(_, y) => -y * 32.,
                    MouseScrollDelta::PixelDelta(p) => -(p.y / view.window.scale_factor()) as f32,
                }))
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                let m = modifiers.state();
                view.modifiers = Modifiers {
                    shift: m.shift_key(),
                    alt: m.alt_key(),
                    control: m.control_key(),
                    meta: m.super_key(),
                };
                false
            }
            WindowEvent::Ime(ime) => match ime {
                Ime::Preedit(text, selection) => view.input(NoticesInput::Preedit(
                    text.clone(),
                    selection.map(|(start, end)| start..end),
                )),
                Ime::Commit(text) => view.input(NoticesInput::Commit(text.clone())),
                Ime::Disabled => view.input(NoticesInput::ImeDisabled),
                _ => false,
            },
            WindowEvent::KeyboardInput { event, .. } if event.state.is_pressed() => {
                if view.ui.is_composing() {
                    return false;
                }
                if view.ui.focused() == Some(&NoticesTarget::Search)
                    && !view.modifiers.control
                    && !view.modifiers.meta
                    && !view.modifiers.alt
                {
                    if let Some(text) = event
                        .text
                        .as_ref()
                        .filter(|text| text.chars().all(|c| !c.is_control()))
                    {
                        return view.input(NoticesInput::Text(text.to_string()));
                    }
                }
                if let Some(key) = key(&event.logical_key) {
                    return view.input(NoticesInput::Key(key, view.modifiers));
                }
                false
            }
            _ => false,
        }
    });
    if close {
        VIEW.with_borrow_mut(Option::take);
    }
}
pub(crate) fn accessibility(event: &accesskit_winit::Event) {
    let close = VIEW.with_borrow_mut(|slot| {
        let Some(view) = slot
            .as_mut()
            .filter(|view| view.window.id() == event.window_id)
        else {
            return false;
        };
        match &event.window_event {
            AccessibilityEvent::ActionRequested(request) => {
                let inputs = view.projection.lock().unwrap().actions(request);
                for input in inputs {
                    if view.input(input) {
                        return true;
                    }
                }
            }
            AccessibilityEvent::InitialTreeRequested => view.window.request_redraw(),
            AccessibilityEvent::AccessibilityDeactivated => view.sent = None,
        }
        false
    });
    if close {
        VIEW.with_borrow_mut(Option::take);
    }
}
fn key(key: &NativeKey) -> Option<Key> {
    Some(match key {
        NativeKey::Character(text) => Key::Character(text.chars().next()?.to_ascii_uppercase()),
        NativeKey::Named(key) => match key {
            NamedKey::Escape => Key::Escape,
            NamedKey::Tab => Key::Tab,
            NamedKey::Enter => Key::Enter,
            NamedKey::Space => Key::Space,
            NamedKey::Backspace => Key::Backspace,
            NamedKey::Delete => Key::Delete,
            NamedKey::ArrowLeft => Key::ArrowLeft,
            NamedKey::ArrowRight => Key::ArrowRight,
            NamedKey::Home => Key::Home,
            NamedKey::End => Key::End,
            NamedKey::PageUp => Key::PageUp,
            NamedKey::PageDown => Key::PageDown,
            _ => return None,
        },
        _ => return None,
    })
}

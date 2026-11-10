//! Windows modal host for the shared, explicitly saved capacity draft.
use crate::features::settings::presenter::capacity::CapacitySession;
use crate::integrations::ui_thread;
use accesskit::{ActivationHandler, TreeUpdate};
use accesskit_winit::{Adapter, WindowEvent as AccessibilityEvent};
use snaplingo_ui::accessibility::{capacity_project, CapacityProjection};
use snaplingo_ui::render::{Renderer, RendererTextMetrics};
use snaplingo_ui::{capacity::*, Key, KeyStyle, Modifiers, Point};
use std::{
    cell::{Cell, RefCell},
    num::NonZeroU32,
    rc::Rc,
    sync::{Arc, Mutex},
};
use winit::{
    dpi::{LogicalPosition, LogicalSize},
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{Key as NativeKey, NamedKey},
    platform::windows::{WindowAttributesExtWindows, WindowExtWindows},
    raw_window_handle::{HasWindowHandle, RawWindowHandle},
    window::{Window, WindowId},
};

thread_local! {
    static SHEET:RefCell<Option<Sheet>>=const {RefCell::new(None)};
    static EPOCH:Cell<u64>=const {Cell::new(0)};
}
struct CachedTree(Arc<Mutex<CapacityProjection>>);
impl ActivationHandler for CachedTree {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        Some(self.0.lock().unwrap().update(None))
    }
}
struct Sheet {
    parent: Rc<Window>,
    surface: softbuffer::Surface<Rc<Window>, Rc<Window>>,
    session: CapacitySession,
    renderer: Renderer,
    metrics: RendererTextMetrics,
    scale: f32,
    adapter: Adapter,
    projection: Arc<Mutex<CapacityProjection>>,
    sent: Option<CapacityProjection>,
    epoch: u64,
    cursor: Point,
    modifiers: Modifiers,
    window: Rc<Window>,
}
impl Drop for Sheet {
    fn drop(&mut self) {
        self.parent.set_enable(true);
        self.window.set_visible(false);
        self.parent.focus_window();
    }
}
/// The owner is retained so closing or hiding Settings can cancel this draft.
pub(crate) fn show(
    event_loop: &ActiveEventLoop,
    parent: Rc<Window>,
    state: crate::features::settings::Services,
) -> Result<(), String> {
    close_for(parent.id());
    let saved = state.configuration.snapshot().map_err(|e| e.to_string())?;
    let appearance = snaplingo_ui::Appearance {
        theme: match saved.general.interface_theme.as_str() {
            "studio" => snaplingo_ui::Theme::Studio,
            "paper" => snaplingo_ui::Theme::Paper,
            _ => snaplingo_ui::Theme::Native,
        },
        dark: saved.general.theme == "dark"
            || saved.general.theme == "system"
                && parent.theme() == Some(winit::window::Theme::Dark),
    };
    let scale =
        snaplingo_core::capture::interface_scale(&saved.screenshot.capture_ui_scale, None) as f32;
    let handle = parent.window_handle().map_err(|e| e.to_string())?;
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return Err("Capacity owner requires a Win32 window".into());
    };
    let mut attributes = Window::default_attributes()
        .with_title("历史与收藏")
        .with_owner_window(handle.hwnd.get())
        .with_visible(false)
        .with_resizable(false)
        .with_inner_size(LogicalSize::new(440. * scale, 510. * scale));
    if let Ok(position) = parent.outer_position() {
        let position = position.to_logical::<f64>(parent.scale_factor());
        let size = parent.inner_size().to_logical::<f64>(parent.scale_factor());
        attributes = attributes.with_position(LogicalPosition::new(
            position.x + (size.width - 440. * f64::from(scale)) / 2.,
            position.y + (size.height - 510. * f64::from(scale)) / 2.,
        ));
    }
    let window = Rc::new(
        event_loop
            .create_window(attributes)
            .map_err(|e| e.to_string())?,
    );
    let context = softbuffer::Context::new(window.clone()).map_err(|e| e.to_string())?;
    let surface = softbuffer::Surface::new(&context, window.clone()).map_err(|e| e.to_string())?;
    let mut session =
        CapacitySession::new(state.configuration.clone(), appearance, KeyStyle::Windows)?;
    let mut metrics = RendererTextMetrics::default();
    let frame = session.ui.frame(scale, &mut metrics);
    let epoch = EPOCH.with(|epoch| {
        let next = epoch.get().wrapping_add(1);
        epoch.set(next);
        next
    });
    let projection = Arc::new(Mutex::new(capacity_project(
        &frame,
        &session.ui,
        window.scale_factor(),
        epoch,
    )));
    let adapter = Adapter::with_mixed_handlers(
        event_loop,
        &window,
        CachedTree(projection.clone()),
        ui_thread::event_proxy()?,
    );
    parent.set_enable(false);
    window.set_visible(true);
    window.focus_window();
    window.request_redraw();
    SHEET.with_borrow_mut(|slot| {
        *slot = Some(Sheet {
            parent,
            window,
            surface,
            session,
            renderer: Renderer::default(),
            metrics,
            scale,
            adapter,
            projection,
            sent: None,
            epoch,
            cursor: Point::default(),
            modifiers: Modifiers::default(),
        })
    });
    Ok(())
}
pub(crate) fn close_for(parent: WindowId) {
    let sheet = SHEET.with_borrow_mut(|slot| {
        if slot
            .as_ref()
            .is_some_and(|sheet| sheet.parent.id() == parent)
        {
            slot.take()
        } else {
            None
        }
    });
    drop(sheet);
}
impl Sheet {
    fn input(&mut self, input: CapacityInput) -> bool {
        let close = self.session.input(input);
        if !close {
            self.window.request_redraw();
        }
        close
    }
    fn draw(&mut self) -> Result<(), String> {
        let size = self.window.inner_size();
        let (Some(width), Some(height)) =
            (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
        else {
            return Ok(());
        };
        let frame = self.session.ui.frame(self.scale, &mut self.metrics);
        let next = capacity_project(
            &frame,
            &self.session.ui,
            self.window.scale_factor(),
            self.epoch,
        );
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
    let close = SHEET.with_borrow_mut(|slot| {
        let Some(sheet) = slot.as_mut().filter(|sheet| sheet.window.id() == id) else {
            return false;
        };
        sheet.adapter.process_event(&sheet.window, event);
        match event {
            WindowEvent::CloseRequested => true,
            WindowEvent::RedrawRequested => {
                if let Err(error) = sheet.draw() {
                    log::error!("Failed to draw capacity dialog: {error}");
                }
                false
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                sheet.window.request_redraw();
                false
            }
            WindowEvent::CursorMoved { position, .. } => {
                let p = position.to_logical::<f32>(sheet.window.scale_factor());
                sheet.cursor = Point { x: p.x, y: p.y };
                false
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => sheet.input(CapacityInput::Pointer(Point {
                x: sheet.cursor.x / sheet.scale,
                y: sheet.cursor.y / sheet.scale,
            })),
            WindowEvent::ModifiersChanged(modifiers) => {
                let m = modifiers.state();
                sheet.modifiers = Modifiers {
                    shift: m.shift_key(),
                    alt: m.alt_key(),
                    control: m.control_key(),
                    meta: m.super_key(),
                };
                false
            }
            WindowEvent::KeyboardInput { event, .. } if event.state.is_pressed() => {
                if let Some(key) = key(&event.logical_key, sheet.modifiers) {
                    return sheet.input(CapacityInput::Key(key, sheet.modifiers));
                }
                if !sheet.modifiers.control && !sheet.modifiers.meta {
                    if let Some(text) = &event.text {
                        return sheet.input(CapacityInput::Text(text.to_string()));
                    }
                }
                false
            }
            _ => false,
        }
    });
    if close {
        SHEET.with_borrow_mut(Option::take);
    }
}
pub(crate) fn accessibility(event: &accesskit_winit::Event) {
    let close = SHEET.with_borrow_mut(|slot| {
        let Some(sheet) = slot
            .as_mut()
            .filter(|sheet| sheet.window.id() == event.window_id)
        else {
            return false;
        };
        match &event.window_event {
            AccessibilityEvent::ActionRequested(request) => {
                let inputs = sheet.projection.lock().unwrap().actions(request);
                for input in inputs {
                    if sheet.input(input) {
                        return true;
                    }
                }
            }
            AccessibilityEvent::InitialTreeRequested => sheet.window.request_redraw(),
            AccessibilityEvent::AccessibilityDeactivated => sheet.sent = None,
        }
        false
    });
    if close {
        SHEET.with_borrow_mut(Option::take);
    }
}
fn key(key: &NativeKey, modifiers: Modifiers) -> Option<Key> {
    Some(match key {
        NativeKey::Character(text)
            if (modifiers.control || modifiers.meta) && text.eq_ignore_ascii_case("a") =>
        {
            Key::Character('A')
        }
        NativeKey::Named(key) => match key {
            NamedKey::Escape => Key::Escape,
            NamedKey::Tab => Key::Tab,
            NamedKey::Enter => Key::Enter,
            NamedKey::Space => Key::Space,
            NamedKey::Backspace => Key::Backspace,
            NamedKey::Delete => Key::Delete,
            NamedKey::ArrowLeft => Key::ArrowLeft,
            NamedKey::ArrowRight => Key::ArrowRight,
            NamedKey::ArrowUp => Key::ArrowUp,
            NamedKey::ArrowDown => Key::ArrowDown,
            NamedKey::Home => Key::Home,
            NamedKey::End => Key::End,
            _ => return None,
        },
        _ => return None,
    })
}

#[cfg(test)]
pub(crate) fn test_is_open() -> bool {
    SHEET.with_borrow(|sheet| sheet.is_some())
}

#[cfg(test)]
pub(crate) fn test_input(input: CapacityInput) {
    let close = SHEET.with_borrow_mut(|sheet| {
        sheet
            .as_mut()
            .expect("Capacity sheet must be open")
            .input(input)
    });
    if close {
        let sheet = SHEET.with_borrow_mut(Option::take);
        drop(sheet);
    }
}

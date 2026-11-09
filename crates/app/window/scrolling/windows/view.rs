//! Transparent scrolling controls, viewport outline, and mouse-through preview.
use super::{
    layered_windows::LayeredPresenter,
    scrolling_geometry::{outside_position, overlaps, preview_placement},
};
use accesskit::{Action, ActivationHandler, Node, NodeId, Role, TreeId, TreeInfo, TreeUpdate};
use accesskit_winit::{Adapter, WindowEvent as AccessibilityEvent};
use base64::Engine;
use snaplingo_core::application::capture::ScrollingCaptureGeometry;
use snaplingo_core::domain::capture::{LogicalRect, ScrollingCaptureStatus, ScrollingCaptureView};
use snaplingo_render::Renderer;
use snaplingo_ui::{
    scrolling::{
        outline_scene, preview_scene, ScrollingAction, ScrollingSnapshot, ScrollingUi,
        VIEWPORT_BORDER,
    },
    Key, KeyStyle, Modifiers, Point, RasterImage, Scene, Size, Snapshot, Theme,
};
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    rc::Rc,
    sync::{Arc, Mutex},
};
use winit::{
    dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{Key as NativeKey, NamedKey},
    platform::windows::WindowAttributesExtWindows,
    raw_window_handle::{HasWindowHandle, RawWindowHandle},
    window::{Window, WindowId, WindowLevel},
};
thread_local! {static VIEWS:RefCell<HashMap<String,ScrollingView>>=RefCell::new(HashMap::new());}
#[derive(Clone)]
struct Semantics {
    nodes: BTreeMap<NodeId, Node>,
    actions: BTreeMap<NodeId, ScrollingAction>,
}
impl Semantics {
    fn update(&self) -> TreeUpdate {
        TreeUpdate {
            nodes: self
                .nodes
                .iter()
                .map(|(id, node)| (*id, node.clone()))
                .collect(),
            tree: Some(TreeInfo::new(NodeId(1))),
            tree_id: TreeId::ROOT,
            focus: NodeId(1),
        }
    }
}
struct CachedTree(Arc<Mutex<Semantics>>);
impl ActivationHandler for CachedTree {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        Some(self.0.lock().unwrap().update())
    }
}
struct Surface {
    window: Rc<Window>,
    presenter: LayeredPresenter,
    renderer: Renderer,
}
impl Surface {
    fn new(
        event_loop: &ActiveEventLoop,
        rect: &LogicalRect,
        desktop_scale: Option<f64>,
        click_through: bool,
        title: &str,
    ) -> Result<Self, String> {
        let mut attributes = Window::default_attributes()
            .with_title(title)
            .with_visible(false)
            .with_decorations(false)
            .with_resizable(false)
            .with_transparent(true)
            .with_active(false)
            .with_skip_taskbar(true)
            .with_window_level(WindowLevel::AlwaysOnTop);
        attributes = if let Some(scale) = desktop_scale {
            attributes
                .with_position(PhysicalPosition::new(
                    (rect.x * scale).round() as i32,
                    (rect.y * scale).round() as i32,
                ))
                .with_inner_size(PhysicalSize::new(
                    (rect.width * scale).round().max(1.) as u32,
                    (rect.height * scale).round().max(1.) as u32,
                ))
        } else {
            attributes
                .with_position(LogicalPosition::new(rect.x, rect.y))
                .with_inner_size(LogicalSize::new(rect.width, rect.height))
        };
        let window = Rc::new(
            event_loop
                .create_window(attributes)
                .map_err(|e| e.to_string())?,
        );
        window
            .set_cursor_hittest(!click_through)
            .map_err(|e| e.to_string())?;
        exclude_capture(&window);
        let presenter = LayeredPresenter::new(&window)?;
        Ok(Self {
            window,
            presenter,
            renderer: Renderer::default(),
        })
    }
    fn present(&mut self, scene: &Scene) -> Result<(), String> {
        let pixels = self
            .renderer
            .render(scene, self.window.scale_factor() as f32)?;
        self.presenter.present(&self.window, &pixels)
    }
    fn bounds(&self, desktop_scale: Option<f64>) -> Option<LogicalRect> {
        let p = self.window.outer_position().ok()?;
        let size = self.window.inner_size();
        let scale = desktop_scale.unwrap_or(self.window.scale_factor());
        Some(LogicalRect {
            x: p.x as f64 / scale,
            y: p.y as f64 / scale,
            width: size.width as f64 / scale,
            height: size.height as f64 / scale,
        })
    }
}
struct ScrollingView {
    // AccessKit removes its native HWND subclass before the surfaces drop.
    adapter: Adapter,
    controls: Surface,
    outline: Surface,
    preview: Option<Surface>,
    view: ScrollingCaptureView,
    ui: ScrollingUi,
    general: snaplingo_core::domain::GeneralSettings,
    dispatch: Box<dyn Fn(ScrollingAction)>,
    stop: Box<dyn Fn()>,
    working: bool,
    error: Option<String>,
    desktop_scale: Option<f64>,
    preview_visible: bool,
    hidden: [bool; 3],
    sampling: u32,
    preview_data: String,
    preview_image: Option<RasterImage>,
    semantics: Arc<Mutex<Semantics>>,
    cursor: Point,
    modifiers: Modifiers,
}
impl Drop for ScrollingView {
    fn drop(&mut self) {
        (self.stop)();
    }
}
fn shell(general: &snaplingo_core::domain::GeneralSettings, window: &Window) -> Snapshot {
    Snapshot {
        theme: match general.interface_theme.as_str() {
            "studio" => Theme::Studio,
            "paper" => Theme::Paper,
            _ => Theme::Native,
        },
        dark: general.theme == "dark"
            || general.theme == "system" && window.theme() == Some(winit::window::Theme::Dark),
        key_style: KeyStyle::Windows,
        ..Snapshot::default()
    }
}
fn model(
    view: &ScrollingCaptureView,
    general: &snaplingo_core::domain::GeneralSettings,
    window: &Window,
    working: bool,
    error: Option<String>,
) -> ScrollingSnapshot {
    ScrollingSnapshot {
        shell: shell(general, window),
        width: view.width,
        height: view.height,
        can_undo: view.can_undo,
        preview_available: view.preview_available,
        preview_expanded: view.preview_expanded,
        working,
        hint: view.hint.clone(),
        error,
    }
}
pub(super) fn open(
    event_loop: &ActiveEventLoop,
    view: ScrollingCaptureView,
    geometry: ScrollingCaptureGeometry,
    general: snaplingo_core::domain::GeneralSettings,
    dispatch: impl Fn(ScrollingAction) + 'static,
    stop: impl Fn() + 'static,
) -> Result<bool, String> {
    let panel = LogicalRect {
        x: geometry.viewport.x,
        y: geometry.viewport.y,
        width: 408. * view.ui_scale,
        height: 132. * view.ui_scale,
    };
    let controls = Surface::new(event_loop, &panel, None, false, "SnapLingo 滚动截图")?;
    place(
        &controls.window,
        geometry.viewport.x,
        geometry.viewport.y,
        geometry.desktop_scale,
    );
    let actual = controls
        .bounds(geometry.desktop_scale)
        .ok_or("Scrolling panel position is unavailable")?;
    let (x, y) = outside_position(&geometry, actual.width, actual.height).unwrap_or_else(|| {
        let screen = geometry.displays.first().unwrap_or(&geometry.viewport);
        (screen.x + screen.width - actual.width - 12., screen.y + 12.)
    });
    place(&controls.window, x, y, geometry.desktop_scale);
    let padding = f64::from(VIEWPORT_BORDER) * view.ui_scale;
    let border = LogicalRect {
        x: geometry.viewport.x - padding,
        y: geometry.viewport.y - padding,
        width: geometry.viewport.width + padding * 2.,
        height: geometry.viewport.height + padding * 2.,
    };
    let outline = Surface::new(
        event_loop,
        &border,
        geometry.desktop_scale,
        true,
        "滚动截图选区",
    )?;
    let preview = preview_placement(
        &geometry,
        view.ui_scale,
        controls.bounds(geometry.desktop_scale).as_ref(),
    )
    .map(|rect| {
        Surface::new(
            event_loop,
            &rect,
            geometry.desktop_scale,
            true,
            "滚动截图预览",
        )
    })
    .transpose()?;
    let available = preview.is_some();
    let mut ui = ScrollingUi::new(model(&view, &general, &controls.window, false, None));
    let frame = ui.frame(view.ui_scale as f32);
    let semantics = Arc::new(Mutex::new(semantics(
        &frame,
        &ui,
        controls.window.scale_factor(),
    )));
    let adapter = Adapter::with_mixed_handlers(
        event_loop,
        &controls.window,
        CachedTree(semantics.clone()),
        crate::infrastructure::ui_thread::event_proxy()?,
    );
    let mut host = ScrollingView {
        controls,
        outline,
        preview,
        ui,
        general,
        view,
        dispatch: Box::new(dispatch),
        stop: Box::new(stop),
        working: false,
        error: None,
        desktop_scale: geometry.desktop_scale,
        preview_visible: available,
        hidden: [false; 3],
        sampling: 0,
        preview_data: String::new(),
        preview_image: None,
        adapter,
        semantics,
        cursor: Point::default(),
        modifiers: Modifiers::default(),
    };
    host.view.preview_available = available;
    host.view.preview_expanded = available;
    host.update_preview();
    host.controls.window.set_visible(true);
    host.outline.window.set_visible(true);
    if let Some(preview) = &host.preview {
        preview.window.set_visible(true);
    }
    host.controls.window.request_redraw();
    host.outline.window.request_redraw();
    if let Some(preview) = &host.preview {
        preview.window.request_redraw();
    }
    VIEWS.with_borrow_mut(|views| views.insert(host.view.id.clone(), host));
    Ok(available)
}
fn place(window: &Window, x: f64, y: f64, scale: Option<f64>) {
    if let Some(scale) = scale {
        window.set_outer_position(PhysicalPosition::new(
            (x * scale).round() as i32,
            (y * scale).round() as i32,
        ));
    } else {
        window.set_outer_position(LogicalPosition::new(x, y));
    }
}
impl ScrollingView {
    fn refresh(&mut self) {
        self.ui.replace(model(
            &self.view,
            &self.general,
            &self.controls.window,
            self.working,
            self.error.clone(),
        ));
        self.controls.window.request_redraw();
    }
    fn update_preview(&mut self) {
        if self.preview_data == self.view.preview_base64 {
            return;
        }
        self.preview_data = self.view.preview_base64.clone();
        self.preview_image = base64::engine::general_purpose::STANDARD
            .decode(
                self.preview_data
                    .split_once(',')
                    .map_or(self.preview_data.as_str(), |(_, data)| data),
            )
            .ok()
            .and_then(|data| image::load_from_memory(&data).ok())
            .and_then(|image| {
                let pixels = image.into_rgba8();
                RasterImage::from_rgba(pixels.width(), pixels.height(), pixels.into_raw())
            });
        if let Some(preview) = &self.preview {
            preview.window.request_redraw();
        }
    }
    fn draw_controls(&mut self) -> Result<(), String> {
        let frame = self.ui.frame(self.view.ui_scale as f32);
        let tree = semantics(&frame, &self.ui, self.controls.window.scale_factor());
        *self.semantics.lock().unwrap() = tree.clone();
        self.adapter.update_if_active(|| tree.update());
        self.controls.present(&frame.scene)
    }
    fn draw_outline(&mut self) -> Result<(), String> {
        let size = self
            .outline
            .window
            .inner_size()
            .to_logical::<f32>(self.outline.window.scale_factor());
        let ratio = self
            .desktop_scale
            .unwrap_or(self.outline.window.scale_factor())
            / self.outline.window.scale_factor();
        let scene = outline_scene(
            Size {
                width: size.width,
                height: size.height,
            },
            (self.view.ui_scale * ratio) as f32,
        );
        self.outline.present(&scene)
    }
    fn draw_preview(&mut self) -> Result<(), String> {
        let Some(preview) = &mut self.preview else {
            return Ok(());
        };
        let size = preview
            .window
            .inner_size()
            .to_logical::<f32>(preview.window.scale_factor());
        preview.present(&preview_scene(
            Size {
                width: size.width,
                height: size.height,
            },
            self.preview_image.clone(),
        ))
    }
}

pub(crate) fn update(next: ScrollingCaptureView) {
    VIEWS.with_borrow_mut(|views| {
        let Some(host) = views.get_mut(&next.id) else {
            return;
        };
        if next.revision < host.view.revision {
            return;
        }
        host.error = (next.status == ScrollingCaptureStatus::Error)
            .then(|| next.hint.clone())
            .flatten();
        host.view = next;
        host.update_preview();
        host.refresh();
    });
}
pub(crate) fn set_working(id: &str, working: bool, error: Option<String>) {
    VIEWS.with_borrow_mut(|views| {
        if let Some(host) = views.get_mut(id) {
            host.working = working;
            host.error = error.or_else(|| {
                (host.view.status == ScrollingCaptureStatus::Error)
                    .then(|| host.view.hint.clone())
                    .flatten()
            });
            host.refresh();
        }
    });
}
pub(super) fn set_preview_visible(id: &str, visible: bool) -> bool {
    VIEWS.with_borrow_mut(|views| {
        let Some(host) = views.get_mut(id) else {
            return false;
        };
        host.preview_visible = visible && host.preview.is_some();
        if let Some(preview) = &host.preview {
            preview
                .window
                .set_visible(host.preview_visible && host.sampling == 0);
        }
        host.preview_visible
    })
}
pub(super) fn begin_sampling(id: &str, viewport: &LogicalRect) -> bool {
    VIEWS.with_borrow_mut(|views| {
        let Some(host) = views.get_mut(id) else {
            return false;
        };
        host.sampling += 1;
        if host.sampling > 1 {
            return false;
        }
        let mut hidden = false;
        for (index, surface) in [
            Some(&host.controls),
            Some(&host.outline),
            host.preview.as_ref(),
        ]
        .into_iter()
        .enumerate()
        {
            if let Some(surface) = surface {
                let overlap = surface.window.is_visible() == Some(true)
                    && surface
                        .bounds(host.desktop_scale)
                        .is_some_and(|bounds| overlaps(&bounds, viewport));
                host.hidden[index] = overlap;
                if overlap {
                    surface.window.set_visible(false);
                    hidden = true;
                }
            }
        }
        hidden
    })
}
pub(super) fn end_sampling(id: &str) {
    VIEWS.with_borrow_mut(|views| {
        if let Some(host) = views.get_mut(id) {
            host.sampling = host.sampling.saturating_sub(1);
            if host.sampling > 0 {
                return;
            }
            for (index, surface) in [
                Some(&host.controls),
                Some(&host.outline),
                host.preview.as_ref(),
            ]
            .into_iter()
            .enumerate()
            {
                if host.hidden[index] && (index != 2 || host.preview_visible) {
                    if let Some(surface) = surface {
                        surface.window.set_visible(true);
                    }
                }
            }
            if let Some(preview) = &host.preview {
                preview.window.set_visible(host.preview_visible);
            }
            host.hidden = [false; 3];
        }
    });
}
pub(super) fn close(id: &str) {
    VIEWS.with_borrow_mut(|views| views.remove(id));
}
pub(crate) fn close_all() {
    VIEWS.with_borrow_mut(HashMap::clear);
}
pub(super) fn settings_changed(general: snaplingo_core::domain::GeneralSettings) {
    VIEWS.with_borrow_mut(|views| {
        for host in views.values_mut() {
            host.general = general.clone();
            host.refresh();
        }
    });
}
fn semantics(
    frame: &snaplingo_ui::scrolling::ScrollingFrame,
    ui: &ScrollingUi,
    scale: f64,
) -> Semantics {
    let mut nodes = BTreeMap::new();
    let mut actions = BTreeMap::new();
    let mut root = Node::new(Role::Window);
    root.set_label("滚动截图控制");
    let mut children = vec![];
    for control in &frame.controls {
        let id = NodeId(10 + control.action as u64);
        let mut node = Node::new(Role::Button);
        node.set_label(control.label.clone());
        let r = control.rect;
        node.set_bounds(accesskit::Rect::new(
            r.x as f64 * scale,
            r.y as f64 * scale,
            (r.x + r.width) as f64 * scale,
            (r.y + r.height) as f64 * scale,
        ));
        if control.enabled {
            node.add_action(Action::Click);
            node.add_action(Action::Focus);
        } else {
            node.set_disabled();
        }
        nodes.insert(id, node);
        actions.insert(id, control.action);
        children.push(id);
    }
    let mut status = Node::new(Role::Status);
    status.set_label(
        ui.snapshot()
            .error
            .as_deref()
            .or(ui.snapshot().hint.as_deref())
            .unwrap_or("手动滚动页面以追加截图"),
    );
    status.set_live(accesskit::Live::Polite);
    nodes.insert(NodeId(2), status);
    children.push(NodeId(2));
    root.set_children(children);
    let _ = ui.focused();
    nodes.insert(NodeId(1), root);
    Semantics { nodes, actions }
}
pub(crate) fn window_event(_event_loop: &ActiveEventLoop, id: WindowId, event: &WindowEvent) {
    let drag = VIEWS.with_borrow_mut(|views| {
        for host in views.values_mut() {
            if host.controls.window.id() == id {
                host.adapter.process_event(&host.controls.window, event);
                match event {
                    WindowEvent::RedrawRequested => {
                        if let Err(error) = host.draw_controls() {
                            log::warn!("Failed to draw scrolling controls: {error}");
                        }
                    }
                    WindowEvent::CloseRequested => (host.dispatch)(ScrollingAction::Cancel),
                    WindowEvent::ThemeChanged(_) => host.refresh(),
                    WindowEvent::ScaleFactorChanged { .. } => host.controls.window.request_redraw(),
                    WindowEvent::CursorMoved { position, .. } => {
                        let p = position.to_logical::<f32>(host.controls.window.scale_factor());
                        host.cursor = Point { x: p.x, y: p.y };
                        if host.ui.hover(Some(host.cursor)) {
                            host.controls.window.request_redraw();
                        }
                    }
                    WindowEvent::CursorLeft { .. } => {
                        if host.ui.hover(None) {
                            host.controls.window.request_redraw();
                        }
                    }
                    WindowEvent::MouseInput {
                        state: ElementState::Pressed,
                        button: MouseButton::Left,
                        ..
                    } => {
                        if let Some(action) = host.ui.pointer(host.cursor) {
                            (host.dispatch)(action);
                        } else if host.cursor.y < 32. * host.view.ui_scale as f32 {
                            return Some(host.controls.window.clone());
                        }
                    }
                    WindowEvent::ModifiersChanged(modifiers) => {
                        let m = modifiers.state();
                        host.modifiers = Modifiers {
                            shift: m.shift_key(),
                            alt: m.alt_key(),
                            control: m.control_key(),
                            meta: m.super_key(),
                        };
                    }
                    WindowEvent::KeyboardInput { event, .. } if event.state.is_pressed() => {
                        if let Some(key) = key(&event.logical_key) {
                            if let Some(action) = host.ui.key(key, host.modifiers) {
                                use snaplingo_core::domain::capture::ScrollingShortcutAction;
                                let registered = match key {
                                    Key::Enter => host
                                        .view
                                        .shortcut_actions
                                        .contains(&ScrollingShortcutAction::Complete),
                                    Key::Escape => host
                                        .view
                                        .shortcut_actions
                                        .contains(&ScrollingShortcutAction::Cancel),
                                    Key::Character('z' | 'Z')
                                        if host.modifiers.control || host.modifiers.meta =>
                                    {
                                        host.view
                                            .shortcut_actions
                                            .contains(&ScrollingShortcutAction::Undo)
                                    }
                                    _ => false,
                                };
                                if !registered {
                                    (host.dispatch)(action);
                                }
                            }
                            host.controls.window.request_redraw();
                        }
                    }
                    _ => {}
                }
                return None;
            } else if host.outline.window.id() == id
                && matches!(
                    event,
                    WindowEvent::RedrawRequested | WindowEvent::ScaleFactorChanged { .. }
                )
            {
                if let Err(error) = host.draw_outline() {
                    log::warn!("Failed to draw scrolling outline: {error}");
                }
                return None;
            } else if host
                .preview
                .as_ref()
                .is_some_and(|preview| preview.window.id() == id)
                && matches!(
                    event,
                    WindowEvent::RedrawRequested | WindowEvent::ScaleFactorChanged { .. }
                )
            {
                if let Err(error) = host.draw_preview() {
                    log::warn!("Failed to draw scrolling preview: {error}");
                }
                return None;
            }
        }
        None
    });
    if let Some(window) = drag {
        let _ = window.drag_window();
    }
}
pub(crate) fn accessibility_event(event: &accesskit_winit::Event) {
    VIEWS.with_borrow_mut(|views| {
        let Some(host) = views
            .values_mut()
            .find(|host| host.controls.window.id() == event.window_id)
        else {
            return;
        };
        match &event.window_event {
            AccessibilityEvent::ActionRequested(request) => {
                if request.target_tree != TreeId::ROOT {
                    return;
                }
                let action = host
                    .semantics
                    .lock()
                    .unwrap()
                    .actions
                    .get(&request.target_node)
                    .copied();
                if let Some(action) = action {
                    match request.action {
                        Action::Click => {
                            if let Some(action) = host.ui.activate(action) {
                                (host.dispatch)(action);
                            }
                        }
                        Action::Focus => {
                            host.ui.focus(action);
                            host.controls.window.request_redraw();
                        }
                        _ => {}
                    }
                }
            }
            AccessibilityEvent::InitialTreeRequested => host.controls.window.request_redraw(),
            AccessibilityEvent::AccessibilityDeactivated => {}
        }
    });
}
fn key(key: &NativeKey) -> Option<Key> {
    Some(match key {
        NativeKey::Character(text) => Key::Character(text.chars().next()?.to_ascii_lowercase()),
        NativeKey::Named(key) => match key {
            NamedKey::Enter => Key::Enter,
            NamedKey::Escape => Key::Escape,
            NamedKey::Tab => Key::Tab,
            NamedKey::Space => Key::Space,
            _ => return None,
        },
        _ => return None,
    })
}
fn exclude_capture(window: &Window) {
    use windows::Win32::{
        Foundation::HWND,
        UI::WindowsAndMessaging::{SetWindowDisplayAffinity, WDA_EXCLUDEFROMCAPTURE},
    };
    if let Ok(handle) = window.window_handle() {
        if let RawWindowHandle::Win32(handle) = handle.as_raw() {
            if let Err(error) =
                unsafe { SetWindowDisplayAffinity(HWND(handle.hwnd.get()), WDA_EXCLUDEFROMCAPTURE) }
            {
                log::debug!("Capture exclusion is unavailable; sampling will hide overlapping windows: {error}");
            }
        }
    }
}

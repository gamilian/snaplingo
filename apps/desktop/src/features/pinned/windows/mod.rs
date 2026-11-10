//! Transparent pin windows hosted by the application's single winit loop.
use super::scene::{Action, PinUi, Preferences};
use crate::features::pinned::Services;
use crate::integrations::system::layered_windows::LayeredPresenter;
use crate::integrations::ui_thread;
use accesskit::{
    Action as AxAction, ActivationHandler, Node, NodeId, Role, TreeId, TreeInfo, TreeUpdate,
};
use accesskit_winit::{Adapter, WindowEvent as AccessibilityEvent};
use base64::Engine;
use snaplingo_core::capture::model::LogicalRect;
use snaplingo_core::capture::model::MonitorSnapshot;
use snaplingo_core::capture::model::PhysicalRect;
use snaplingo_core::capture::model::PinnedImageExportSize;
use snaplingo_core::capture::model::PinnedImageView;
use snaplingo_core::pinned::PinnedImagePlacement;
use snaplingo_core::pinned::PinnedImageRuntimeHost;
use snaplingo_core::settings::SettingsConfiguration;
use snaplingo_ui::pinned::Presentation;
use snaplingo_ui::render::Renderer;
use snaplingo_ui::{Key, Modifiers, Point};
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    rc::Rc,
    sync::{Arc, Mutex, RwLock},
    time::{Duration, Instant},
};
use winit::{
    dpi::{LogicalPosition, LogicalSize, PhysicalPosition},
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{Key as NativeKey, NamedKey},
    platform::windows::{WindowAttributesExtWindows, WindowExtWindows},
    raw_window_handle::{HasWindowHandle, RawWindowHandle},
    window::{Window, WindowId, WindowLevel},
};

#[derive(Default)]
pub(crate) struct NativePinnedImageRuntimeHost {
    services: RwLock<Option<Services>>,
}
impl NativePinnedImageRuntimeHost {
    pub(crate) fn bind(&self, services: Services) {
        *self.services.write().unwrap() = Some(services);
    }

    fn services(&self) -> Result<Services, String> {
        self.services
            .read()
            .unwrap()
            .clone()
            .ok_or_else(|| "Native pinned image services are not initialized".into())
    }
    pub(crate) fn settings_changed(&self) {
        let Ok(services) = self.services() else {
            return;
        };
        ui_thread::dispatch(move || {
            if let Ok(preferences) = preferences(&services.settings) {
                WINDOWS.with_borrow_mut(|windows| {
                    for window in windows.values_mut() {
                        window.preferences = preferences.clone();
                        window.window.set_undecorated_shadow(preferences.shadow);
                        if let Some(ui) = &mut window.ui {
                            ui.preferences = preferences.clone();
                            let _ = ui.changed();
                        }
                        window.window.request_redraw();
                    }
                });
            }
        });
    }
}
#[async_trait::async_trait]
impl PinnedImageRuntimeHost for NativePinnedImageRuntimeHost {
    async fn open(
        &self,
        image: PinnedImageView,
        placement: PinnedImagePlacement,
    ) -> crate::Result<()> {
        let services = self.services()?;
        ui_thread::run_with_event_loop("open pinned image", move |event_loop| {
            open(event_loop, image, placement, services)
        })
        .await
    }
    async fn show_or_open(&self, image: PinnedImageView) -> crate::Result<()> {
        let services = self.services()?;
        ui_thread::run_with_event_loop("show pinned image", move |event_loop| {
            let found = WINDOWS.with_borrow_mut(|windows| {
                if let Some(window) = windows.get_mut(&image.id) {
                    window.show(true)?;
                    Ok::<_, String>(true)
                } else {
                    Ok(false)
                }
            })?;
            if !found {
                open(event_loop, image, PinnedImagePlacement::default(), services)?;
            }
            Ok(())
        })
        .await
    }
    async fn hide(&self, id: String) -> crate::Result<()> {
        ui_thread::run("hide pinned image", move || {
            WINDOWS.with_borrow_mut(|windows| {
                if let Some(window) = windows.get_mut(&id) {
                    window.hide();
                }
            });
            Ok(())
        })
        .await
    }
    async fn toggle_all(&self) -> crate::Result<Option<bool>> {
        ui_thread::run("toggle pinned images", || {
            WINDOWS.with_borrow_mut(|windows| {
                if windows.is_empty() {
                    return Ok(None);
                }
                let visible = windows
                    .values()
                    .any(|window| window.window.is_visible() != Some(true));
                for window in windows.values_mut() {
                    if visible {
                        window.show(false)?;
                    } else {
                        window.hide();
                    }
                }
                Ok(Some(visible))
            })
        })
        .await
    }
    async fn apply_group_switch(&self, hide: Vec<String>, show: Vec<String>) -> crate::Result<()> {
        ui_thread::run("switch pin group", move || {
            WINDOWS.with_borrow_mut(|windows| {
                for id in hide {
                    if let Some(window) = windows.get_mut(&id) {
                        window.hide();
                    }
                }
                for id in show {
                    if let Some(window) = windows.get_mut(&id) {
                        window.show(false)?;
                    }
                }
                Ok(())
            })
        })
        .await
    }
    async fn hide_group(&self, ids: Vec<String>) -> crate::Result<()> {
        ui_thread::run("hide pin group", move || {
            WINDOWS.with_borrow_mut(|windows| {
                for id in ids {
                    if let Some(window) = windows.get_mut(&id) {
                        window.hide();
                    }
                }
            });
            Ok(())
        })
        .await
    }
    async fn close_group(&self, ids: Vec<String>) -> crate::Result<()> {
        ui_thread::run("destroy pin group", move || {
            WINDOWS.with_borrow_mut(|windows| {
                for id in ids {
                    windows.remove(&id);
                }
            });
            Ok(())
        })
        .await
    }
}
fn preferences(settings: &SettingsConfiguration) -> Result<Preferences, String> {
    let saved = settings.snapshot().map_err(|e| e.to_string())?.screenshot;
    Ok(Preferences {
        opacity: f64::from(saved.pin_opacity) / 100.,
        shadow: saved.pin_shadow,
        border: saved.pin_border,
        border_shortcut: saved.pin_border_shortcut,
    })
}
thread_local! {static WINDOWS:RefCell<HashMap<String,PinWindow>>=RefCell::new(HashMap::new());}
#[derive(Clone)]
struct Semantics {
    nodes: BTreeMap<NodeId, Node>,
    actions: BTreeMap<NodeId, Action>,
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
struct PinWindow {
    presenter: Option<LayeredPresenter>,
    ui: Option<PinUi>,
    renderer: Renderer,
    image: PinnedImageView,
    presentation: Presentation,
    preferences: Preferences,
    services: Services,
    gate: Arc<tokio::sync::Mutex<()>>,
    adapter: Adapter,
    semantics: Arc<Mutex<Semantics>>,
    last_click: Option<(Instant, Point)>,
    // Native accessibility subclasses must detach before destroying the HWND.
    window: Rc<Window>,
}
fn open(
    event_loop: &ActiveEventLoop,
    image: PinnedImageView,
    placement: PinnedImagePlacement,
    services: Services,
) -> Result<(), String> {
    let preferences = preferences(&services.settings)?;
    let bytes = decode(&image)?;
    let ui = PinUi::new(&bytes, image.source_text.clone(), preferences.clone())?;
    let (width, height) = ui.dimensions();
    let attributes = Window::default_attributes()
        .with_title("SnapLingo 贴图")
        .with_visible(false)
        .with_decorations(false)
        .with_transparent(true)
        .with_resizable(false)
        .with_window_level(WindowLevel::AlwaysOnTop)
        .with_undecorated_shadow(preferences.shadow)
        .with_skip_taskbar(true)
        .with_inner_size(LogicalSize::new(width, height));
    let mut native_monitors: Vec<_> = event_loop.available_monitors().collect();
    if let Some(primary) = event_loop.primary_monitor() {
        if let Some(index) = native_monitors
            .iter()
            .position(|monitor| monitor == &primary)
        {
            native_monitors.swap(0, index);
        }
    }
    let monitors: Vec<_> = native_monitors
        .iter()
        .enumerate()
        .map(|(index, monitor)| {
            let p = monitor.position();
            let size = monitor.size();
            let scale = monitor.scale_factor();
            MonitorSnapshot {
                id: index.to_string(),
                logical_bounds: LogicalRect {
                    x: p.x as f64 / scale,
                    y: p.y as f64 / scale,
                    width: size.width as f64 / scale,
                    height: size.height as f64 / scale,
                },
                physical_bounds: PhysicalRect {
                    x: p.x,
                    y: p.y,
                    width: size.width,
                    height: size.height,
                },
                scale_factor: scale,
                png_data: Vec::new(),
            }
        })
        .collect();
    let window = Rc::new(
        event_loop
            .create_window(attributes)
            .map_err(|e| e.to_string())?,
    );
    let cascade = WINDOWS.with_borrow(HashMap::len);
    let place = || {
        let size = window.inner_size();
        super::geometry::position(&placement, &monitors, (size.width, size.height), cascade)
    };
    if let Some(point) = place() {
        window.set_outer_position(PhysicalPosition::new(point.x, point.y));
        // Moving to another DPI may resize and reposition the logical-DIP
        // window. Restore its chosen physical origin, or recenter its actual
        // resulting extent, before revealing it; rendering keeps its DPI.
        if let Some(adjusted) = place() {
            if window
                .outer_position()
                .is_ok_and(|actual| actual.x != adjusted.x || actual.y != adjusted.y)
            {
                window.set_outer_position(PhysicalPosition::new(adjusted.x, adjusted.y));
            }
        }
    }
    let presenter = LayeredPresenter::new(&window)?;
    let semantics = Arc::new(Mutex::new(semantics(&ui, window.scale_factor())));
    let adapter = Adapter::with_mixed_handlers(
        event_loop,
        &window,
        CachedTree(semantics.clone()),
        ui_thread::event_proxy()?,
    );
    window.set_visible(true);
    window.focus_window();
    window.request_redraw();
    let id = image.id.clone();
    let pin = PinWindow {
        window,
        presenter: Some(presenter),
        presentation: ui.presentation.clone(),
        ui: Some(ui),
        renderer: Renderer::default(),
        image,
        preferences,
        services,
        gate: Arc::new(tokio::sync::Mutex::new(())),
        adapter,
        semantics,
        last_click: None,
    };
    WINDOWS.with_borrow_mut(|windows| windows.insert(id, pin));
    Ok(())
}
fn decode(image: &PinnedImageView) -> Result<Vec<u8>, String> {
    base64::engine::general_purpose::STANDARD
        .decode(
            image
                .image_base64
                .split_once(',')
                .map_or(image.image_base64.as_str(), |(_, data)| data),
        )
        .map_err(|e| e.to_string())
}
impl PinWindow {
    fn show(&mut self, focus: bool) -> Result<(), String> {
        if self.ui.is_none() {
            let mut ui = PinUi::new(
                &decode(&self.image)?,
                self.image.source_text.clone(),
                self.preferences.clone(),
            )?;
            ui.presentation = self.presentation.clone();
            ui.changed()?;
            self.ui = Some(ui);
            self.presenter = Some(LayeredPresenter::new(&self.window)?);
        }
        self.window.set_visible(true);
        if focus {
            self.window.focus_window();
        }
        self.window.request_redraw();
        Ok(())
    }
    fn hide(&mut self) {
        self.window.set_visible(false);
        if let Some(ui) = self.ui.take() {
            self.presentation = ui.presentation.clone();
        }
        self.presenter = None;
    }
    fn change(&mut self) -> Result<(), String> {
        let Some(ui) = &mut self.ui else {
            return Ok(());
        };
        ui.changed()?;
        let (width, height) = ui.dimensions();
        let old = self
            .window
            .inner_size()
            .to_logical::<f64>(self.window.scale_factor());
        if (width - old.width).abs() > 0.5 || (height - old.height).abs() > 0.5 {
            if let Ok(position) = self.window.outer_position() {
                let p = position.to_logical::<f64>(self.window.scale_factor());
                self.window.set_outer_position(LogicalPosition::new(
                    p.x + (old.width - width) / 2.,
                    p.y + (old.height - height) / 2.,
                ));
            }
            let _ = self
                .window
                .request_inner_size(LogicalSize::new(width, height));
        }
        self.window.request_redraw();
        Ok(())
    }
    fn draw(&mut self) -> Result<(), String> {
        let Some(ui) = &self.ui else {
            return Ok(());
        };
        let scene = ui.scene();
        let next = semantics(ui, self.window.scale_factor());
        *self.semantics.lock().unwrap() = next.clone();
        self.adapter.update_if_active(|| next.update());
        let pixels = self
            .renderer
            .render(&scene, self.window.scale_factor() as f32)?;
        if let Some(presenter) = &mut self.presenter {
            presenter.present(&self.window, &pixels)?;
        }
        Ok(())
    }
    fn emit(&self, action: Action) {
        let size = self.ui.as_ref().map_or_else(
            || {
                self.presentation
                    .export_size(self.image.width, self.image.height)
            },
            |ui| {
                ui.presentation
                    .export_size(self.image.width, self.image.height)
            },
        );
        dispatch(
            self.services.clone(),
            self.image.id.clone(),
            self.gate.clone(),
            action,
            PinnedImageExportSize {
                width: size.width,
                height: size.height,
            },
        );
    }
    fn menu(&mut self, tag: u32) -> Result<(), String> {
        let Some(ui) = &mut self.ui else {
            return Ok(());
        };
        let action = ui.menu(tag);
        if let Some(action) = action {
            self.emit(action);
        }
        self.change()
    }
}
fn dispatch(
    services: Services,
    id: String,
    gate: Arc<tokio::sync::Mutex<()>>,
    action: Action,
    size: PinnedImageExportSize,
) {
    crate::integrations::runtime::spawn(async move {
        let _operation = gate.lock().await;
        if let Err(error) = run_action(&services, &id, action, size).await {
            log::warn!("Pinned image action failed: {error}");
            let message = error.to_string();
            ui_thread::dispatch(move || {
                rfd::MessageDialog::new()
                    .set_title("贴图操作失败")
                    .set_description(message)
                    .set_level(rfd::MessageLevel::Error)
                    .show();
            });
        }
    });
}
async fn run_action(
    services: &Services,
    id: &str,
    action: Action,
    size: PinnedImageExportSize,
) -> crate::Result<()> {
    let runtime = services
        .runtime
        .upgrade()
        .ok_or("Pinned image runtime has stopped")?;
    match action {
        Action::Copy => runtime.copy_at_size(id, Some(size)).await,
        Action::Save => {
            let preferences = services.settings.snapshot()?.screenshot;
            let format = preferences
                .last_save_format
                .as_deref()
                .unwrap_or(&preferences.format);
            let default = services.output.dialog_capture_save_path(
                preferences
                    .last_save_directory
                    .as_deref()
                    .or(Some(&preferences.save_path)),
                Some(&preferences.save_path),
                format,
                &preferences.naming_rule,
                &preferences.custom_file_name,
            );
            let name = default
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let directory = default.parent().map(|path| path.to_path_buf());
            let future = ui_thread::run("save pinned image", move || {
                let mut dialog = rfd::AsyncFileDialog::new()
                    .set_title("保存贴图")
                    .add_filter("图片", &["png", "jpg", "webp"])
                    .set_file_name(name);
                if let Some(directory) = directory {
                    dialog = dialog.set_directory(directory);
                }
                Ok(dialog.save_file())
            })
            .await?;
            if let Some(file) = future.await {
                runtime
                    .save_at_size(id, file.path(), Some(size), true)
                    .await?;
            }
            Ok(())
        }
        Action::QuickSave => {
            let preferences = services.settings.snapshot()?.screenshot;
            let path = services.output.quick_capture_save_path(
                Some(&preferences.save_path),
                "png",
                "timestamp",
                "SnapLingo",
            );
            runtime.save_at_size(id, &path, Some(size), false).await
        }
        Action::Close => runtime.close(id).await,
        Action::Destroy => {
            runtime.remove(id)?;
            let id = id.to_owned();
            ui_thread::run("destroy pinned image", move || {
                WINDOWS.with_borrow_mut(|windows| windows.remove(&id));
                Ok(())
            })
            .await
        }
        Action::HideGroup => runtime.hide_group(id).await.map(|_| ()),
        Action::DestroyGroup => runtime.destroy_group(id).await.map(|_| ()),
        Action::CloseAll => runtime.close_all().await.map(|_| ()),
        Action::MoveGroup => runtime.move_to_next_group(id).await.map(|_| ()),
        Action::Preferences => {
            let show = services.show_settings.clone();
            ui_thread::run("open pinned preferences", move || show()).await
        }
        Action::Replace => {
            let image = runtime.replace_from_clipboard(id)?;
            ui_thread::run("replace pinned image", move || {
                WINDOWS.with_borrow_mut(|windows| {
                    if let Some(window) = windows.get_mut(&image.id) {
                        let mut ui = PinUi::new(
                            &decode(&image)?,
                            image.source_text.clone(),
                            window.preferences.clone(),
                        )?;
                        if let Some(previous) = &window.ui {
                            ui.presentation.opacity = previous.presentation.opacity;
                            ui.presentation.grayscale = previous.presentation.grayscale;
                            ui.presentation.inverted = previous.presentation.inverted;
                            ui.changed()?;
                        }
                        window.ui = Some(ui);
                        window.image = image;
                        window.change()?;
                    }
                    Ok(())
                })
            })
            .await
        }
        Action::ToggleBorder => {
            let mut preferences = services.settings.snapshot()?.screenshot;
            preferences.pin_border = !preferences.pin_border;
            services.settings.update_screenshot(preferences)?;
            Ok(())
        }
        Action::CopyText(text) => {
            snaplingo_platform::clipboard_text::copy_text(text).map_err(Into::into)
        }
    }
}
fn semantics(ui: &PinUi, scale: f64) -> Semantics {
    let mut nodes = BTreeMap::new();
    let mut actions = BTreeMap::new();
    let mut root = Node::new(Role::Window);
    root.set_label("SnapLingo 贴图");
    let mut children = vec![];
    let mut image = Node::new(Role::Image);
    image.set_label(ui.source_text.as_deref().unwrap_or("固定在桌面的截图"));
    let (width, height) = ui.dimensions();
    image.set_bounds(accesskit::Rect::new(0., 0., width * scale, height * scale));
    nodes.insert(NodeId(2), image);
    children.push(NodeId(2));
    for (index, (action, rect, label)) in ui.toolbar().into_iter().enumerate() {
        let id = NodeId(10 + index as u64);
        let mut node = Node::new(Role::Button);
        node.set_label(label);
        node.set_bounds(accesskit::Rect::new(
            rect.x as f64 * scale,
            rect.y as f64 * scale,
            (rect.x + rect.width) as f64 * scale,
            (rect.y + rect.height) as f64 * scale,
        ));
        node.add_action(AxAction::Click);
        node.add_action(AxAction::Focus);
        nodes.insert(id, node);
        actions.insert(id, action);
        children.push(id);
    }
    root.set_children(children);
    nodes.insert(NodeId(1), root);
    Semantics { nodes, actions }
}
enum NativeOperation {
    Drag(Rc<Window>),
    Menu(Rc<Window>, String, bool),
}
pub(crate) fn window_event(_event_loop: &ActiveEventLoop, id: WindowId, event: &WindowEvent) {
    let operation = WINDOWS.with_borrow_mut(|windows| {
        let Some(window) = windows.values_mut().find(|window| window.window.id() == id) else {
            return None;
        };
        window.adapter.process_event(&window.window, event);
        match event {
            WindowEvent::CloseRequested => window.emit(Action::Close),
            WindowEvent::RedrawRequested => {
                if let Err(error) = window.draw() {
                    log::warn!("Failed to draw pin: {error}");
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => window.window.request_redraw(),
            WindowEvent::CursorEntered { .. } => {
                if let Some(ui) = &mut window.ui {
                    ui.hovered = true;
                }
                window.window.request_redraw();
            }
            WindowEvent::CursorLeft { .. } => {
                if let Some(ui) = &mut window.ui {
                    ui.hovered = false;
                }
                window.window.request_redraw();
            }
            WindowEvent::CursorMoved { position, .. } => {
                let p = position.to_logical::<f32>(window.window.scale_factor());
                if let Some(ui) = &mut window.ui {
                    ui.cursor = Point { x: p.x, y: p.y };
                    if ui.modifiers.alt {
                        window.window.request_redraw();
                    }
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                let m = modifiers.state();
                if let Some(ui) = &mut window.ui {
                    ui.modifiers(Modifiers {
                        shift: m.shift_key(),
                        alt: m.alt_key(),
                        control: m.control_key(),
                        meta: m.super_key(),
                    });
                }
                window.window.request_redraw();
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Right,
                ..
            } => {
                return Some(NativeOperation::Menu(
                    window.window.clone(),
                    window.image.id.clone(),
                    window.preferences.border,
                ))
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Middle,
                ..
            } => {
                if let Some(ui) = &mut window.ui {
                    ui.presentation.zoom = 1.;
                    ui.presentation.thumbnail = false;
                    ui.presentation.opacity = ui.preferences.opacity;
                }
                if let Err(error) = window.change() {
                    log::warn!("Failed to reset pin: {error}");
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                if let Some(ui) = &window.ui {
                    if let Some((action, _, _)) = ui
                        .toolbar()
                        .into_iter()
                        .find(|(_, rect, _)| rect.contains(ui.cursor))
                    {
                        window.emit(action);
                        return None;
                    }
                    let double = window.last_click.is_some_and(|(time, point)| {
                        time.elapsed() < Duration::from_millis(450)
                            && (point.x - ui.cursor.x).abs() < 4.
                            && (point.y - ui.cursor.y).abs() < 4.
                    });
                    window.last_click = Some((Instant::now(), ui.cursor));
                    if double {
                        if ui.modifiers.shift {
                            if let Some(ui) = &mut window.ui {
                                ui.presentation.thumbnail = !ui.presentation.thumbnail;
                            }
                            if let Err(error) = window.change() {
                                log::warn!("Failed to resize pin: {error}");
                            }
                        } else {
                            window.emit(Action::Close);
                        }
                        return None;
                    }
                }
                return Some(NativeOperation::Drag(window.window.clone()));
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if let Some(ui) = &mut window.ui {
                    let y = match delta {
                        MouseScrollDelta::LineDelta(_, y) => *y as f64,
                        MouseScrollDelta::PixelDelta(p) => p.y,
                    };
                    if !ui.scroll(y) {
                        return None;
                    }
                }
                if let Err(error) = window.change() {
                    log::warn!("Failed to resize pin: {error}");
                }
            }
            WindowEvent::KeyboardInput { event, .. } if event.state.is_pressed() => {
                let Some(key) = key(&event.logical_key) else {
                    return None;
                };
                if let Some(ui) = &mut window.ui {
                    match ui.key(key, event.repeat) {
                        Ok(Some(action)) => window.emit(action),
                        Ok(None) => {
                            if let Some((x, y)) = ui.movement(key) {
                                if let Ok(position) = window.window.outer_position() {
                                    let p =
                                        position.to_logical::<f64>(window.window.scale_factor());
                                    window
                                        .window
                                        .set_outer_position(LogicalPosition::new(p.x + x, p.y + y));
                                }
                            } else if let Err(error) = window.change() {
                                log::warn!("Failed to transform pin: {error}");
                            }
                        }
                        Err(error) => log::warn!("Failed to transform pin: {error}"),
                    }
                }
            }
            _ => {}
        }
        None
    });
    // Native dragging and menus run nested message loops, so release our window
    // storage borrow before entering either operating-system operation.
    match operation {
        Some(NativeOperation::Drag(window)) => {
            if let Err(error) = window.drag_window() {
                log::warn!("Failed to drag pin: {error}");
            }
        }
        Some(NativeOperation::Menu(window, image_id, border)) => {
            if let Ok(tag) = popup_menu(&window, border) {
                WINDOWS.with_borrow_mut(|windows| {
                    if let Some(window) = windows.get_mut(&image_id) {
                        if let Err(error) = window.menu(tag) {
                            log::warn!("Failed to update pin menu: {error}");
                        }
                    }
                });
            }
        }
        None => {}
    }
}
pub(crate) fn accessibility_event(event: &accesskit_winit::Event) {
    WINDOWS.with_borrow_mut(|windows| {
        let Some(window) = windows
            .values_mut()
            .find(|window| window.window.id() == event.window_id)
        else {
            return;
        };
        match &event.window_event {
            AccessibilityEvent::ActionRequested(request) => {
                if request.target_tree != TreeId::ROOT {
                    return;
                }
                let action = window
                    .semantics
                    .lock()
                    .unwrap()
                    .actions
                    .get(&request.target_node)
                    .cloned();
                if let Some(action) = action {
                    match request.action {
                        AxAction::Click => window.emit(action),
                        AxAction::Focus => window.window.focus_window(),
                        _ => {}
                    }
                }
            }
            AccessibilityEvent::InitialTreeRequested => window.window.request_redraw(),
            AccessibilityEvent::AccessibilityDeactivated => {}
        }
    });
}
pub(crate) fn close_all() {
    WINDOWS.with_borrow_mut(HashMap::clear);
}
fn key(key: &NativeKey) -> Option<Key> {
    Some(match key {
        NativeKey::Character(text) => Key::Character(text.chars().next()?.to_ascii_lowercase()),
        NativeKey::Named(key) => match key {
            NamedKey::Escape => Key::Escape,
            NamedKey::Space => Key::Space,
            NamedKey::Enter => Key::Enter,
            NamedKey::Tab => Key::Tab,
            NamedKey::Backspace => Key::Backspace,
            NamedKey::Delete => Key::Delete,
            NamedKey::ArrowLeft => Key::ArrowLeft,
            NamedKey::ArrowRight => Key::ArrowRight,
            NamedKey::ArrowUp => Key::ArrowUp,
            NamedKey::ArrowDown => Key::ArrowDown,
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
            _ => return None,
        },
        _ => return None,
    })
}
fn popup_menu(window: &Window, border: bool) -> Result<u32, String> {
    use windows::core::PCWSTR;
    use windows::Win32::{
        Foundation::{HWND, POINT},
        UI::WindowsAndMessaging::{
            AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, MF_STRING,
        },
    };
    #[link(name = "user32")]
    extern "system" {
        #[link_name = "TrackPopupMenu"]
        fn track_menu(
            menu: isize,
            flags: u32,
            x: i32,
            y: i32,
            reserved: i32,
            window: isize,
            rect: *const core::ffi::c_void,
        ) -> i32;
    }
    let handle = window.window_handle().map_err(|e| e.to_string())?;
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return Err("Pin menu requires a Win32 window".into());
    };
    let menu = unsafe { CreatePopupMenu() }.map_err(|e| e.to_string())?;
    let result = (|| {
        for (tag, label) in [
            (1, "重置大小"),
            (2, "透明度 100%"),
            (3, "透明度 75%"),
            (4, "透明度 50%"),
            (5, "移到另一组"),
            (
                6,
                if border {
                    "隐藏边框"
                } else {
                    "显示边框"
                },
            ),
            (7, "收起贴图"),
            (8, "隐藏当前组"),
            (9, "复制"),
            (10, "另存为"),
            (11, "快速保存 PNG"),
            (12, "关闭当前贴图"),
            (13, "关闭当前组"),
            (14, "关闭全部贴图"),
        ] {
            let wide: Vec<u16> = label.encode_utf16().chain(std::iter::once(0)).collect();
            unsafe { AppendMenuW(menu, MF_STRING, tag, PCWSTR(wide.as_ptr())) }
                .map_err(|e| e.to_string())?;
        }
        let mut point = POINT::default();
        unsafe { GetCursorPos(&mut point) }.map_err(|e| e.to_string())?;
        let hwnd = HWND(handle.hwnd.get());
        Ok(unsafe {
            track_menu(
                menu.0,
                0x100 | 0x2,
                point.x,
                point.y,
                0,
                hwnd.0,
                std::ptr::null(),
            )
        } as u32)
    })();
    let _ = unsafe { DestroyMenu(menu) };
    result
}

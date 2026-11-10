//! One capture overlay on the application's existing winit event loop.
use super::super::preview::{cursor_rect, local_rect, preview_annotations, PreviewRequests};
use super::{
    actions::{self, Action},
    Services,
};
use crate::integrations::system::layered_windows::LayeredPresenter;
use accesskit::{ActivationHandler, TreeUpdate};
use accesskit_winit::{Adapter, WindowEvent as AccessibilityEvent};
use snaplingo_core::capture::model::CaptureSessionId;
use snaplingo_core::capture::model::LogicalPoint;
use snaplingo_core::capture::model::LogicalRect;
use snaplingo_core::capture::AnnotationTool;
use snaplingo_core::capture::CaptureEditor;
use snaplingo_core::capture::CaptureEditorControlAction;
use snaplingo_core::capture::CaptureEditorStyleChange;
use snaplingo_core::capture::CaptureEditorStylePresentation;
use snaplingo_core::capture::CaptureSelectionMode;
use snaplingo_core::capture::CaptureSession;
use snaplingo_core::capture::CaptureWindowGeometry;
use snaplingo_core::capture::Selection;
use snaplingo_core::settings::model::SettingsSnapshot;
use snaplingo_ui::render::{Renderer, RendererTextMetrics};
use snaplingo_ui::{capture::*, Modifiers, Point, RasterImage, Rect, Size, Theme};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{Key, NamedKey},
    platform::windows::WindowAttributesExtWindows,
    window::{Window, WindowId, WindowLevel},
};

mod assets;
mod input;
mod semantics;
mod text;

static PRESENTATIONS: AtomicUsize = AtomicUsize::new(0);
thread_local! {static VIEW:RefCell<Option<View>>=const{RefCell::new(None)};}
struct CachedTree(Arc<Mutex<semantics::Semantics>>);
impl ActivationHandler for CachedTree {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        Some(self.0.lock().unwrap().update())
    }
}
struct View {
    // The adapter must drop before the HWND that it subclasses.
    adapter: Adapter,
    presenter: LayeredPresenter,
    window: Rc<Window>,
    services: Services,
    id: CaptureSessionId,
    mode: CaptureSelectionMode,
    geometry: CaptureWindowGeometry,
    preferences: SettingsSnapshot,
    selection: Selection,
    editor: Option<CaptureEditor>,
    ui: CaptureUi,
    images: Vec<CaptureImage>,
    cursor_image: Option<CaptureImage>,
    preview: Option<CaptureImage>,
    renderer: Renderer,
    metrics: RendererTextMetrics,
    semantics: Arc<Mutex<semantics::Semantics>>,
    frame: CaptureFrame,
    snapshot: CaptureSnapshot,
    cursor: Point,
    modifiers: Modifiers,
    pressed_ui: bool,
    include_cursor: bool,
    busy: bool,
    suspended: bool,
    focused: bool,
    rendering: PreviewRequests,
    error: Option<String>,
    history: Vec<LogicalRect>,
    text: Option<snaplingo_ui::TextEdit>,
    text_undo: Vec<snaplingo_ui::TextEdit>,
    text_redo: Vec<snaplingo_ui::TextEdit>,
    last_click: Option<(std::time::Instant, Point)>,
    text_dragging: bool,
    rgb: bool,
    magnifier: bool,
    fonts: snaplingo_platform::annotation_fonts::SystemAnnotationFonts,
    toolbar_focus: Option<CaptureTarget>,
}
pub(super) fn open(
    event_loop: &ActiveEventLoop,
    services: Services,
    session: CaptureSession,
    mode: CaptureSelectionMode,
    geometry: CaptureWindowGeometry,
    preferences: SettingsSnapshot,
    history: Vec<LogicalRect>,
) -> Result<(), String> {
    let scale = geometry.desktop_scale.unwrap_or(1.);
    let bounds = &geometry.bounds;
    let window = Rc::new(
        event_loop
            .create_window(
                Window::default_attributes()
                    .with_title("SnapLingo 截图")
                    .with_visible(false)
                    .with_decorations(false)
                    .with_resizable(false)
                    .with_transparent(true)
                    .with_skip_taskbar(true)
                    .with_window_level(WindowLevel::AlwaysOnTop)
                    .with_position(PhysicalPosition::new(
                        (bounds.x * scale).round() as i32,
                        (bounds.y * scale).round() as i32,
                    ))
                    .with_inner_size(PhysicalSize::new(
                        (bounds.width * scale).round().max(1.) as u32,
                        (bounds.height * scale).round().max(1.) as u32,
                    )),
            )
            .map_err(|e| e.to_string())?,
    );
    let images = session
        .snapshots
        .iter()
        .filter(|m| {
            session
                .selection_monitor_id
                .as_ref()
                .is_none_or(|id| id == &m.id)
        })
        .map(|m| {
            Ok(CaptureImage {
                rect: local_rect(&m.logical_bounds, bounds),
                image: decode(&m.png_data)?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    if images.is_empty() {
        return Err("Capture session has no frozen monitor images".into());
    }
    let cursor_image = session.captured_cursor.as_ref().and_then(|c| {
        let image = decode(&c.png_data).ok()?;
        Some(CaptureImage {
            rect: Rect {
                ..cursor_rect(c, bounds)
            },
            image,
        })
    });
    let mut selection = Selection::new(
        bounds.clone(),
        session
            .layout_snapshots
            .iter()
            .map(|m| m.logical_bounds.clone())
            .collect(),
        session.candidates,
    );
    let cursor = session
        .captured_cursor
        .as_ref()
        .map(|c| Point {
            x: (c.logical_position.x - bounds.x) as f32,
            y: (c.logical_position.y - bounds.y) as f32,
        })
        .unwrap_or(Point {
            x: bounds.width as f32 / 2.,
            y: bounds.height as f32 / 2.,
        });
    selection.move_to(
        LogicalPoint {
            x: bounds.x + cursor.x as f64,
            y: bounds.y + cursor.y as f64,
        },
        false,
    );
    let semantics = Arc::new(Mutex::new(semantics::Semantics::default()));
    let adapter = Adapter::with_mixed_handlers(
        event_loop,
        &window,
        CachedTree(semantics.clone()),
        crate::integrations::ui_thread::event_proxy()?,
    );
    let presenter = LayeredPresenter::new(&window)?;
    let mut ui = CaptureUi::default();
    let snapshot = CaptureSnapshot::default();
    let frame = ui.frame(&snapshot);
    let magnifier = preferences.screenshot.show_magnifier;
    let mut view = View {
        adapter,
        presenter,
        window,
        services,
        id: session.id,
        mode,
        geometry,
        preferences,
        selection,
        editor: None,
        ui,
        images,
        cursor_image,
        preview: None,
        renderer: Renderer::default(),
        metrics: RendererTextMetrics::default(),
        semantics,
        frame,
        snapshot,
        cursor,
        modifiers: Modifiers::default(),
        pressed_ui: false,
        include_cursor: false,
        busy: false,
        suspended: false,
        focused: false,
        rendering: PreviewRequests::default(),
        error: None,
        history,
        text: None,
        text_undo: vec![],
        text_redo: vec![],
        last_click: None,
        text_dragging: false,
        rgb: false,
        magnifier,
        fonts: Default::default(),
        toolbar_focus: None,
    };
    view.refresh();
    VIEW.with_borrow_mut(|slot| *slot = Some(view));
    Ok(())
}
fn decode(png: &[u8]) -> Result<RasterImage, String> {
    let image = image::load_from_memory(png)
        .map_err(|e| e.to_string())?
        .into_rgba8();
    RasterImage::from_rgba(image.width(), image.height(), image.into_raw())
        .ok_or_else(|| "Invalid capture raster".into())
}
impl View {
    fn scale(&self) -> f64 {
        self.geometry
            .desktop_scale
            .unwrap_or(self.window.scale_factor())
    }
    fn desktop_point(&self) -> LogicalPoint {
        LogicalPoint {
            x: self.geometry.bounds.x + self.cursor.x as f64,
            y: self.geometry.bounds.y + self.cursor.y as f64,
        }
    }
    fn editor_point(&self) -> LogicalPoint {
        let point = self.desktop_point();
        let selection = &self.editor.as_ref().unwrap().selection;
        LogicalPoint {
            x: point.x - selection.x,
            y: point.y - selection.y,
        }
    }
    fn dispatch(&self, action: Action) {
        actions::dispatch(self.services.clone(), self.id.clone(), self.mode, action);
    }
    fn refresh(&mut self) {
        if self.editor.as_ref().is_none_or(|e| e.text.is_none()) {
            self.text = None;
            self.text_dragging = false;
        }
        let bounds = &self.geometry.bounds;
        let s = &self.preferences.screenshot;
        let editor = self.editor.as_ref();
        let selection = editor
            .map(|e| &e.selection)
            .or_else(|| self.selection.active_rect());
        let style = editor.map(|e| &e.styling);
        let mut snapshot = CaptureSnapshot {
            size: Size {
                width: bounds.width as f32,
                height: bounds.height as f32,
            },
            theme: match self.preferences.general.interface_theme.as_str() {
                "studio" => Theme::Studio,
                "paper" => Theme::Paper,
                _ => Theme::Native,
            },
            dark: self.preferences.general.theme == "dark"
                || self.preferences.general.theme == "system"
                    && self.window.theme() == Some(winit::window::Theme::Dark),
            ui_scale: snaplingo_core::capture::interface_scale(&s.capture_ui_scale, Some(bounds))
                as f32,
            desktop_scale: self.scale() as f32,
            images: self.images.clone(),
            cursor: self
                .include_cursor
                .then(|| self.cursor_image.clone())
                .flatten(),
            preview: self.preview.clone(),
            selection: selection.map(|r| local_rect(r, bounds)),
            selection_tinted: editor.is_none()
                && (self.selection.anchor.is_some() || self.selection.rect.is_none()),
            point: Some(self.cursor),
            selection_size: selection
                .map(|r| {
                    format!(
                        "{} x {} px",
                        (r.width * self.scale()).ceil() as u32,
                        (r.height * self.scale()).ceil() as u32
                    )
                })
                .unwrap_or_default(),
            show_size: s.show_selection_size,
            border_color: s.selection_border_color,
            border_width: s.selection_border_width as f32,
            mask_color: s.selection_mask_color,
            editing: editor.is_some(),
            tool: editor
                .and_then(|e| e.tool)
                .and_then(|t| CaptureTool::ALL.into_iter().find(|c| c.name() == t.name())),
            can_undo: editor.is_some_and(CaptureEditor::can_undo),
            can_redo: editor.is_some_and(CaptureEditor::can_redo),
            busy: self.busy,
            hint: if editor.is_some() {
                String::new()
            } else {
                self.error.clone().unwrap_or_else(|| {
                    format!(
                        "拖动选区或点击窗口 · Enter {} · Tab 界面元素 · Alt 放大镜 · Esc 取消",
                        match self.mode {
                            CaptureSelectionMode::Edit | CaptureSelectionMode::Copy => "复制截图",
                            CaptureSelectionMode::Ocr => "识别文字",
                            CaptureSelectionMode::SilentOcr => "复制文字",
                            CaptureSelectionMode::Translate => "截图翻译",
                        }
                    )
                })
            },
            ..CaptureSnapshot::default()
        };
        if let Some(style) = style {
            snapshot.style = super::super::shared_presentation::style(style);
            snapshot.style.error = self.error.clone().or(snapshot.style.error);
        }
        if let Some(editor) = editor {
            snapshot.handles = editor
                .selected_handles()
                .into_iter()
                .map(|(_, p)| Point {
                    x: (p.x + editor.selection.x - bounds.x) as f32,
                    y: (p.y + editor.selection.y - bounds.y) as f32,
                })
                .collect();
        }
        snapshot.icons = if self.snapshot.theme == snapshot.theme
            && self.snapshot.dark == snapshot.dark
            && !self.snapshot.icons.is_empty()
        {
            self.snapshot.icons.clone()
        } else {
            assets::icons(snapshot.theme, snapshot.dark)
        };
        self.decorate_snapshot(&mut snapshot);
        self.snapshot = snapshot;
        self.frame = self.ui.frame_with_text(&self.snapshot, &mut self.metrics);
        self.decorate_text();
        if let Some(control) = self.toolbar_focus.and_then(|target| {
            self.frame
                .controls
                .iter()
                .find(|c| c.target == target && c.enabled)
        }) {
            self.frame
                .scene
                .commands
                .push(snaplingo_ui::DrawCommand::StrokeRect {
                    rect: control.rect,
                    radius: 4. * self.snapshot.ui_scale,
                    width: 1.5,
                    color: icon_colors(self.snapshot.theme, self.snapshot.dark)[1],
                });
        }
        self.window
            .set_cursor(if self.frame.hit_test(self.cursor).is_some() {
                winit::window::CursorIcon::Default
            } else if self.text.is_some() {
                winit::window::CursorIcon::Text
            } else if self.editor.as_ref().is_some_and(|e| e.tool.is_none()) {
                winit::window::CursorIcon::Default
            } else {
                winit::window::CursorIcon::Crosshair
            });
        *self.semantics.lock().unwrap() = semantics::project(
            &self.frame,
            &self.snapshot,
            self.scale(),
            &self.id.0,
            self.text.as_ref(),
            self.text_layout().map(|(r, _)| r),
            self.toolbar_focus,
        );
        let tree = self.semantics.lock().unwrap().update();
        self.adapter.update_if_active(|| tree);
        self.window.request_redraw();
        self.queue_render();
    }
    fn queue_render(&mut self) {
        let Some(editor) = &self.editor else {
            return;
        };
        // Draft gestures and text affect the preview before image_revision changes.
        let Some(revision) = self
            .rendering
            .request(editor.revision, self.selection.revision)
        else {
            return;
        };
        let annotations = preview_annotations(editor);
        self.dispatch(Action::Render {
            rect: editor.selection.clone(),
            annotations,
            include_cursor: self.include_cursor,
            revision,
        });
    }
    fn draw(&mut self) -> Result<(), String> {
        let pixels = self
            .renderer
            .render(&self.frame.scene, self.scale() as f32)?;
        self.presenter.present(&self.window, &pixels)?;
        Ok(())
    }
    fn inspect(&self) {
        if self.selection.control_detection && self.selection.anchor.is_none() {
            if let Some(point) = self.selection.point.clone() {
                self.dispatch(Action::Inspect {
                    point,
                    revision: self.selection.revision,
                });
            }
        }
    }
    fn enter_editor(&mut self, rect: LogicalRect) {
        self.selection.rect = Some(rect.clone());
        let s = &self.preferences.screenshot;
        let mut style = CaptureEditorStylePresentation::default();
        style.style.stroke_width = s.default_stroke_width as u32;
        style.font_size = s.default_font_size as u32;
        style.color_presets = s.annotation_colors.clone();
        style.font_families = self
            .services
            .runtime()
            .map(|r| r.annotation_font_families())
            .unwrap_or_default();
        let tool = if s.remember_last_tool {
            self.services
                .runtime()
                .ok()
                .and_then(|r| AnnotationTool::parse(&r.last_annotation_tool()))
        } else {
            Some(AnnotationTool::Rectangle)
        };
        self.editor = Some(CaptureEditor::new(rect, tool, style));
        self.preview = None;
        self.rendering.invalidate();
        self.refresh();
    }
    fn complete_selection(&mut self, rect: LogicalRect) {
        if self.mode == CaptureSelectionMode::Edit {
            self.enter_editor(rect);
        } else {
            self.selection.rect = Some(rect);
            self.command(CaptureCommand::Primary);
        }
    }
    fn command(&mut self, command: CaptureCommand) {
        if self.busy {
            return;
        }
        if matches!(command, CaptureCommand::Cancel | CaptureCommand::Refresh) {
            self.busy = true;
            self.dispatch(if command == CaptureCommand::Cancel {
                Action::Cancel
            } else {
                Action::Refresh
            });
            self.refresh();
            return;
        }
        if matches!(
            command,
            CaptureCommand::Select
                | CaptureCommand::Undo
                | CaptureCommand::Redo
                | CaptureCommand::Clear
        ) {
            if let Some(editor) = &mut self.editor {
                editor.control(&CaptureEditorControlAction::Command(command.name()));
                self.text = None;
                self.refresh();
            }
            return;
        }
        if let Some(editor) = &mut self.editor {
            if let Some(text) = &mut self.text {
                text.clear_preedit();
                editor.update_text(text.text().into());
            }
            editor.commit_text();
            self.text = None;
        }
        let rect = self
            .editor
            .as_ref()
            .map(|e| e.selection.clone())
            .or_else(|| self.selection.active_rect().cloned());
        let Some(rect) = rect else {
            return;
        };
        let annotations = self
            .editor
            .as_ref()
            .map(CaptureEditor::export_annotations)
            .unwrap_or_default();
        self.busy = true;
        self.dispatch(Action::Output {
            command,
            rect,
            annotations,
            include_cursor: self.include_cursor,
        });
        self.refresh();
    }
    fn intents(&mut self, intents: Vec<CaptureIntent>) {
        for intent in intents {
            match intent {
                CaptureIntent::Command(command) => {
                    if let Some(CaptureIntent::Command(command)) = presentation::toolbar_intent(
                        CaptureTarget::Command(command),
                        &self.snapshot,
                        false,
                        self.modifiers.shift,
                    ) {
                        self.command(command);
                    }
                }
                CaptureIntent::Tool(tool) => {
                    self.commit_composition();
                    if let Some(editor) = &mut self.editor {
                        editor.control(&CaptureEditorControlAction::Command(tool.name()));
                        self.text = None;
                    }
                    self.dispatch(Action::RememberTool(tool.name().into()));
                }
                CaptureIntent::Style { value, commit } => {
                    let change = style_change(value);
                    if let Some(editor) = &mut self.editor {
                        editor.control(&CaptureEditorControlAction::Style {
                            change: change.clone(),
                            commit,
                        });
                    }
                    if commit {
                        self.dispatch(Action::Style(change));
                    }
                }
                CaptureIntent::Presets(colors) => self.dispatch(Action::Presets(colors)),
                CaptureIntent::SystemColor(color) => {
                    self.busy = true;
                    self.dispatch(Action::SystemColor(color));
                }
            }
        }
        self.refresh();
    }
}
fn style_change(value: CaptureStyleValue) -> CaptureEditorStyleChange {
    match value {
        CaptureStyleValue::Color(v) => CaptureEditorStyleChange::Color(v),
        CaptureStyleValue::StrokeWidth(v) => CaptureEditorStyleChange::StrokeWidth(v),
        CaptureStyleValue::FontSize(v) => CaptureEditorStyleChange::FontSize(v),
        CaptureStyleValue::Filled(v) => CaptureEditorStyleChange::Filled(v),
        CaptureStyleValue::CornerRadius(v) => CaptureEditorStyleChange::CornerRadius(v),
        CaptureStyleValue::BlurRadius(v) => CaptureEditorStyleChange::BlurRadius(v),
        CaptureStyleValue::FontFamily(v) => CaptureEditorStyleChange::FontFamily(v),
        CaptureStyleValue::Bold(v) => CaptureEditorStyleChange::Bold(v),
        CaptureStyleValue::Italic(v) => CaptureEditorStyleChange::Italic(v),
    }
}

fn with_view<R>(id: &str, f: impl FnOnce(&mut View) -> R) -> Option<R> {
    VIEW.with_borrow_mut(|slot| slot.as_mut().filter(|v| v.id.0 == id).map(f))
}
pub(super) fn is_current(id: &str) -> bool {
    VIEW.with_borrow(|v| v.as_ref().is_some_and(|v| v.id.0 == id))
}
pub(super) fn is_busy(id: &str) -> bool {
    VIEW.with_borrow(|v| v.as_ref().is_some_and(|v| v.id.0 == id && v.busy))
}
pub(super) fn owns_focus(id: &str) -> bool {
    VIEW.with_borrow(|v| {
        v.as_ref()
            .is_some_and(|v| v.id.0 == id && (v.focused || v.suspended || v.busy))
    })
}
pub(super) fn is_active() -> bool {
    PRESENTATIONS.load(Ordering::Acquire) > 0
}
pub(super) fn set_active(active: bool) {
    if active {
        PRESENTATIONS.fetch_add(1, Ordering::AcqRel);
    } else {
        let _ = PRESENTATIONS.fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
            Some(n.saturating_sub(1))
        });
    }
}
pub(super) fn reveal() -> Result<(), String> {
    VIEW.with_borrow_mut(|slot| {
        let v = slot.as_mut().ok_or("No capture window")?;
        v.suspended = false;
        v.window.set_visible(true);
        v.window.focus_window();
        v.refresh();
        Ok(())
    })
}
pub(super) fn hide() {
    VIEW.with_borrow_mut(|slot| {
        if let Some(v) = slot {
            v.suspended = true;
            v.window.set_visible(false);
        }
    });
}
pub(super) fn suspend(id: &str) {
    with_view(id, |v| {
        v.suspended = true;
        v.window.set_visible(false);
    });
}
pub(super) fn passthrough(enabled: bool) -> Result<(), String> {
    VIEW.with_borrow(|slot| {
        if let Some(v) = slot {
            v.window
                .set_cursor_hittest(!enabled)
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    })
}
pub(super) fn close() -> Option<String> {
    VIEW.with_borrow_mut(|slot| slot.take().map(|v| v.id.0))
}
pub(super) fn close_session(id: &str) {
    VIEW.with_borrow_mut(|slot| {
        if slot.as_ref().is_some_and(|v| v.id.0 == id) {
            slot.take();
        }
    });
}
pub(crate) fn close_all() {
    close();
}
pub(super) fn request_pin(id: &str) -> bool {
    with_view(id, |v| {
        if v.busy || v.selection.active_rect().is_none() {
            false
        } else {
            v.command(CaptureCommand::Pin);
            true
        }
    })
    .unwrap_or(false)
}
pub(super) fn settings_changed(settings: SettingsSnapshot) {
    VIEW.with_borrow_mut(|slot| {
        if let Some(v) = slot {
            v.preferences = settings;
            v.refresh();
        }
    });
}
pub(super) fn action_finished(id: &str, error: Option<String>) {
    with_view(id, |v| {
        v.busy = false;
        v.error = error;
        if v.suspended {
            v.suspended = false;
            v.window.set_visible(true);
            v.window.focus_window();
        }
        v.refresh();
    });
}
pub(super) fn control_result(id: &str, revision: u64, result: Result<Option<LogicalRect>, String>) {
    with_view(id, |v| {
        match result {
            Ok(rect) => {
                v.selection.control_result(revision, rect);
            }
            Err(error) => {
                if v.selection.control_failed(revision) {
                    v.error = Some(error);
                }
            }
        }
        v.refresh();
    });
}
pub(super) fn rendered(
    id: &str,
    revision: u64,
    rect: LogicalRect,
    pixels: image::RgbaImage,
) -> Result<(), String> {
    with_view(id, |v| {
        if v.rendering.is_current(revision) && v.editor.is_some() {
            v.preview = RasterImage::from_rgba(pixels.width(), pixels.height(), pixels.into_raw())
                .map(|image| CaptureImage {
                    rect: local_rect(&rect, &v.geometry.bounds),
                    image,
                });
            v.refresh();
        }
    });
    Ok(())
}
pub(super) fn render_failed(id: &str, revision: u64, error: String) {
    with_view(id, |v| {
        if v.rendering.is_current(revision) && v.editor.is_some() {
            v.error = Some(error);
            v.refresh();
        }
    });
}
pub(super) fn presets(id: &str, colors: Vec<[u8; 4]>) {
    with_view(id, |v| {
        if let Some(e) = &mut v.editor {
            e.styling.color_presets = colors;
        }
        v.refresh();
    });
}
pub(super) fn apply_color(id: &str, color: [u8; 4]) {
    with_view(id, |v| {
        v.busy = false;
        if let Some(e) = &mut v.editor {
            e.control(&CaptureEditorControlAction::Style {
                change: CaptureEditorStyleChange::Color(color),
                commit: true,
            });
        }
        v.refresh();
    });
}
pub(super) fn choose_color(color: [u8; 4]) -> Result<Option<[u8; 4]>, String> {
    snaplingo_platform::color_dialog::choose_color(color)
}
pub(super) fn print_file(path: &std::path::Path) -> Result<bool, String> {
    let window = VIEW
        .with_borrow(|slot| slot.as_ref().map(|view| view.window.clone()))
        .ok_or("Capture session has stopped")?;
    crate::integrations::system::image_print_windows::print(path, &window)
}
pub(crate) fn window_event(_event_loop: &ActiveEventLoop, id: WindowId, event: &WindowEvent) {
    VIEW.with_borrow_mut(|slot| {
        let Some(v) = slot.as_mut().filter(|v| v.window.id() == id) else {
            return;
        };
        v.adapter.process_event(&v.window, event);
        if let Err(error) = v.event(event) {
            v.error = Some(error.to_string());
            v.busy = false;
            v.refresh();
        }
    });
}
pub(crate) fn accessibility_event(event: &accesskit_winit::Event) {
    VIEW.with_borrow_mut(|slot| {
        let Some(v) = slot.as_mut().filter(|v| v.window.id() == event.window_id) else {
            return;
        };
        match &event.window_event {
            AccessibilityEvent::ActionRequested(request) => v.accessibility(request),
            AccessibilityEvent::InitialTreeRequested => v.refresh(),
            AccessibilityEvent::AccessibilityDeactivated => {}
        }
    });
}

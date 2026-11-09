//! A result window on the application's shared winit loop.
use super::controller::{Action, ResultWindowController, ResultWindowEffects, Services};
use crate::application::result_window::{
    resolved_target, ResultWindowMode, ResultWindowNotifierPort, ResultWindowPresentationPort,
    ResultWindowRequestId, ResultWindowSnapshot, ResultWindowWindowPort, TranslationStatus,
};
use crate::infrastructure::system::layered_windows::LayeredPresenter;
use crate::infrastructure::ui_thread;
use accesskit::{ActivationHandler, TreeUpdate};
use accesskit_winit::{Adapter, WindowEvent as AccessibilityEvent};
use base64::Engine;
use snaplingo_accessibility::{result_project, ResultProjection};
use snaplingo_render::{Renderer, RendererTextMetrics};
use snaplingo_ui::result::{
    ResultFrame, ResultInput, ResultIntent, ResultMode, ResultRow, ResultSnapshot, ResultUi,
    RowStatus,
};
use snaplingo_ui::{Key, KeyStyle, Modifiers, Point, RasterImage, Size, Snapshot, Theme};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event::{ElementState, Ime, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key as NativeKey, NamedKey};
use winit::window::{Window, WindowId, WindowLevel};

struct NativeEffects;
#[async_trait::async_trait]
impl ResultWindowEffects for NativeEffects {
    async fn pick_image(&self) -> crate::Result<Option<PathBuf>> {
        let future = ui_thread::run("select OCR image", || {
            Ok(rfd::AsyncFileDialog::new()
                .set_title("选择图片进行 OCR")
                .add_filter(
                    "图片",
                    &["png", "jpg", "jpeg", "webp", "bmp", "gif", "tiff"],
                )
                .pick_file())
        })
        .await?;
        Ok(future.await.map(|file| file.path().to_path_buf()))
    }
    fn copy_text(&self, text: String) -> crate::Result<()> {
        crate::infrastructure::system::clipboard_text::copy_text(text).map_err(Into::into)
    }
}
#[derive(Default)]
pub(crate) struct NativeResultWindowHost {
    controller: RwLock<Option<ResultWindowController>>,
}
impl NativeResultWindowHost {
    pub(crate) fn bind(&self, state: &crate::AppState) {
        *self.controller.write().unwrap() = Some(ResultWindowController::new(Services {
            window: Arc::downgrade(&state.result_window),
            session: state.result_session.clone(),
            settings: state.settings.configuration.clone(),
            ocr: state.providers.ocr.clone(),
            file_ocr: state.providers.file_ocr.clone(),
            favorites: state.favorites.favorites.clone(),
            tts: state.tts.clone(),
            effects: Arc::new(NativeEffects),
        }));
    }
    fn controller(&self) -> Result<ResultWindowController, String> {
        self.controller
            .read()
            .unwrap()
            .clone()
            .ok_or_else(|| "Native result window services are not initialized".into())
    }
    pub(crate) fn settings_changed(&self) {
        if let Ok(controller) = self.controller() {
            controller.settings_changed();
        }
    }
}
#[async_trait::async_trait]
impl ResultWindowWindowPort for NativeResultWindowHost {
    async fn show_or_create(&self) -> crate::Result<()> {
        let controller = self.controller()?;
        ui_thread::run_with_event_loop("show result window", move |event_loop| {
            VIEW.with_borrow_mut(|slot| {
                if let Some(view) = slot {
                    view.controller = controller;
                    view.show();
                } else {
                    *slot = Some(ResultView::new(event_loop, controller)?);
                }
                Ok(())
            })
        })
        .await
    }
}
#[async_trait::async_trait]
impl ResultWindowNotifierPort for NativeResultWindowHost {
    async fn notify_payload_ready(&self, id: ResultWindowRequestId) -> crate::Result<()> {
        let controller = self.controller()?;
        ui_thread::run("receive result payload", move || {
            controller.receive_payload(id)
        })
        .await
    }
}
pub(crate) struct NativeResultWindowPresentation;
impl ResultWindowPresentationPort for NativeResultWindowPresentation {
    fn publish(&self, snapshot: ResultWindowSnapshot) {
        ui_thread::dispatch(move || {
            VIEW.with_borrow_mut(|view| {
                if let Some(view) = view {
                    view.update(snapshot);
                }
            })
        });
    }
}
thread_local! { static VIEW:RefCell<Option<ResultView>>=const { RefCell::new(None) }; }
struct CachedTree(Arc<Mutex<ResultProjection>>);
impl ActivationHandler for CachedTree {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        Some(self.0.lock().unwrap().update(None))
    }
}
struct ResultView {
    presenter: LayeredPresenter,
    controller: ResultWindowController,
    snapshot: ResultWindowSnapshot,
    ui: ResultUi,
    renderer: Renderer,
    metrics: RendererTextMetrics,
    accessibility: Adapter,
    projection: Arc<Mutex<ResultProjection>>,
    sent: Option<ResultProjection>,
    cursor: Point,
    modifiers: Modifiers,
    opened_at: Instant,
    image_source: Option<Arc<str>>,
    image: Option<RasterImage>,
    ime_enabled: bool,
    max_height: f64,
    window: Rc<Window>,
}
impl ResultView {
    fn new(
        event_loop: &ActiveEventLoop,
        controller: ResultWindowController,
    ) -> Result<Self, String> {
        let snapshot = controller.snapshot();
        let width = f64::from(snapshot.preferences.translation.window_width).clamp(300., 1000.);
        let monitor = event_loop
            .primary_monitor()
            .or_else(|| event_loop.available_monitors().next());
        let display = monitor.as_ref().map(|monitor| {
            let size = monitor.size().to_logical::<f64>(monitor.scale_factor());
            crate::domain::capture::LogicalRect {
                x: 0.,
                y: 0.,
                width: size.width,
                height: size.height,
            }
        });
        let ui_scale = crate::application::capture::interface_scale(
            &snapshot.preferences.screenshot.capture_ui_scale,
            display.as_ref(),
        );
        let width = (width * ui_scale).min(
            display
                .as_ref()
                .map_or(1200., |display| (display.width - 32.).max(300.)),
        );
        let height = monitor
            .as_ref()
            .map(|m| {
                m.size().to_logical::<f64>(m.scale_factor()).height
                    * f64::from(snapshot.preferences.translation.max_window_height_ratio)
                    / 100.
            })
            .unwrap_or(650.)
            .clamp(250., 850.);
        let window = Rc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("SnapLingo")
                        .with_inner_size(LogicalSize::new(width, height))
                        .with_min_inner_size(LogicalSize::new(300., 150.))
                        .with_visible(false)
                        .with_decorations(false)
                        .with_transparent(true)
                        .with_window_level(
                            if snapshot.preferences.translation.window_always_on_top {
                                WindowLevel::AlwaysOnTop
                            } else {
                                WindowLevel::Normal
                            },
                        ),
                )
                .map_err(|e| e.to_string())?,
        );
        if let Some(monitor) = monitor {
            let pos = monitor.position().to_logical::<f64>(monitor.scale_factor());
            let size = monitor.size().to_logical::<f64>(monitor.scale_factor());
            window.set_outer_position(LogicalPosition::new(
                pos.x + (size.width - width) / 2.,
                pos.y + (size.height - height) / 2.,
            ));
        }
        let presenter = LayeredPresenter::new(&window)?;
        let mut ui = ResultUi::new(project(&snapshot, None, window.theme()));
        let mut metrics = RendererTextMetrics::default();
        let frame = ui.frame(
            Size {
                width: width as f32,
                height: height as f32,
            },
            ui_scale as f32,
            &mut metrics,
        );
        if snapshot.mode == ResultWindowMode::Translation {
            ui.update(ResultInput::Focus(
                snaplingo_ui::result::ResultTarget::Source,
            ));
        }
        let projection = Arc::new(Mutex::new(result_project(
            &frame,
            &ui,
            window.scale_factor(),
        )));
        let accessibility = Adapter::with_mixed_handlers(
            event_loop,
            &window,
            CachedTree(projection.clone()),
            ui_thread::event_proxy()?,
        );
        let mut view = Self {
            window,
            presenter,
            controller,
            snapshot,
            ui,
            renderer: Renderer::default(),
            metrics,
            accessibility,
            projection,
            sent: None,
            cursor: Point::default(),
            modifiers: Modifiers::default(),
            opened_at: Instant::now(),
            image_source: None,
            image: None,
            ime_enabled: false,
            max_height: height,
        };
        view.show();
        Ok(view)
    }
    fn show(&mut self) {
        self.opened_at = Instant::now();
        self.window.set_visible(true);
        self.window.focus_window();
        self.window.request_redraw();
    }
    fn update(&mut self, snapshot: ResultWindowSnapshot) {
        if snapshot.revision < self.snapshot.revision {
            return;
        }
        if self.image_source != snapshot.ocr_image {
            self.image_source = snapshot.ocr_image.clone();
            self.image = self
                .image_source
                .as_deref()
                .and_then(|data| {
                    base64::engine::general_purpose::STANDARD
                        .decode(data.split_once(',').map_or(data, |(_, data)| data))
                        .ok()
                })
                .and_then(|bytes| image::load_from_memory(&bytes).ok())
                .and_then(|image| {
                    let rgba = image.into_rgba8();
                    RasterImage::from_rgba(rgba.width(), rgba.height(), rgba.into_raw())
                });
        }
        self.ui
            .replace(project(&snapshot, self.image.clone(), self.window.theme()));
        if snapshot.presentation != self.snapshot.presentation
            && snapshot.mode == ResultWindowMode::Translation
        {
            let _ = self.frame();
            self.ui.update(ResultInput::Focus(
                snaplingo_ui::result::ResultTarget::Source,
            ));
        }
        self.window
            .set_window_level(if snapshot.preferences.translation.window_always_on_top {
                WindowLevel::AlwaysOnTop
            } else {
                WindowLevel::Normal
            });
        if !snapshot.visible {
            self.window.set_visible(false);
        }
        self.snapshot = snapshot;
        self.window.request_redraw();
    }
    fn input(&mut self, input: ResultInput) {
        for intent in self.ui.update(input) {
            let action = match intent {
                ResultIntent::ReadClipboard => {
                    match arboard::Clipboard::new().and_then(|mut clipboard| clipboard.get_text()) {
                        Ok(text) => self.input(ResultInput::Text(text)),
                        Err(error) => log::warn!("Failed to paste result source: {error}"),
                    }
                    continue;
                }
                ResultIntent::Edit(text, composing) => Action::Edit(text, composing),
                ResultIntent::SourceLanguage(code) => Action::SourceLanguage(code),
                ResultIntent::TargetLanguage(code) => Action::TargetLanguage(code),
                ResultIntent::Swap => Action::Swap,
                ResultIntent::Translate => Action::Translate,
                ResultIntent::Retry(id) => Action::Retry(id),
                ResultIntent::FileOcr => Action::FileOcr,
                ResultIntent::ClearOcr => Action::ClearOcr,
                ResultIntent::Copy(text) => Action::Copy(text),
                ResultIntent::Speak(text, language) => Action::Speak(
                    text,
                    language.map(|language| {
                        if language == self.snapshot.target_lang {
                            resolved_target(
                                &self.snapshot.source_text,
                                &self.snapshot.source_lang,
                                &self.snapshot.target_lang,
                            )
                        } else {
                            language
                        }
                    }),
                ),
                ResultIntent::Favorite(ids) => Action::Favorite(self.snapshot.clone(), ids),
                ResultIntent::Drag => {
                    ui_thread::dispatch(|| {
                        let window =
                            VIEW.with_borrow(|view| view.as_ref().map(|view| view.window.clone()));
                        if let Some(window) = window {
                            let _ = window.drag_window();
                        }
                    });
                    continue;
                }
                ResultIntent::Close => {
                    self.window.set_visible(false);
                    Action::Close
                }
            };
            self.controller.dispatch(action);
            self.update(self.controller.snapshot());
        }
        self.window.request_redraw();
    }
    fn frame(&mut self) -> ResultFrame {
        let size = self
            .window
            .inner_size()
            .to_logical::<f32>(self.window.scale_factor());
        let display = self.window.current_monitor().map(|monitor| {
            let size = monitor.size().to_logical::<f64>(monitor.scale_factor());
            crate::domain::capture::LogicalRect {
                x: 0.,
                y: 0.,
                width: size.width,
                height: size.height,
            }
        });
        let scale = crate::application::capture::interface_scale(
            &self.snapshot.preferences.screenshot.capture_ui_scale,
            display.as_ref(),
        );
        self.ui.frame(
            Size {
                width: size.width,
                height: size.height,
            },
            scale as f32,
            &mut self.metrics,
        )
    }
    fn draw(&mut self) -> Result<(), String> {
        let size = self.window.inner_size();
        if size.width == 0 || size.height == 0 {
            return Ok(());
        }
        let frame = self.frame();
        let current_size = size.to_logical::<f64>(self.window.scale_factor());
        let height = f64::from(frame.scene.content_height + frame.panel.y)
            .min(self.max_height)
            .max(150.);
        if (current_size.height - height).abs() > 1. {
            let _ = self
                .window
                .request_inner_size(LogicalSize::new(current_size.width, height));
        }
        let next = result_project(&frame, &self.ui, self.window.scale_factor());
        *self.projection.lock().unwrap() = next.clone();
        if self.sent.as_ref() != Some(&next) {
            let mut sent = false;
            self.accessibility.update_if_active(|| {
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
        self.presenter.present(&self.window, &pixels)
    }
}
fn project(
    snapshot: &ResultWindowSnapshot,
    image: Option<RasterImage>,
    system_theme: Option<winit::window::Theme>,
) -> ResultSnapshot {
    let general = &snapshot.preferences.general;
    let shell = Snapshot {
        theme: match general.interface_theme.as_str() {
            "studio" => Theme::Studio,
            "paper" => Theme::Paper,
            _ => Theme::Native,
        },
        dark: general.theme == "dark"
            || general.theme == "system" && system_theme == Some(winit::window::Theme::Dark),
        key_style: KeyStyle::Windows,
        ..Snapshot::default()
    };
    let mut metadata = Vec::new();
    if let Some(provider) = &snapshot.ocr_provider {
        metadata.push(provider.clone());
    }
    if let Some(language) = &snapshot.ocr_language {
        metadata.push(language.clone());
    }
    if snapshot.preferences.ocr.show_confidence {
        if let Some(confidence) = snapshot.ocr_confidence {
            metadata.push(format!("置信度 {:.0}%", confidence * 100.));
        }
    }
    ResultSnapshot {
        shell,
        revision: snapshot.revision,
        epoch: snapshot.epoch,
        visible: snapshot.visible,
        mode: if snapshot.mode == ResultWindowMode::Ocr {
            ResultMode::Ocr
        } else {
            ResultMode::Translation
        },
        source: snapshot.source_text.clone(),
        composing: snapshot.composing,
        source_language: snapshot.source_lang.clone(),
        target_language: snapshot.target_lang.clone(),
        detected_language: snapshot
            .rows
            .iter()
            .find_map(|row| row.result.detected_language.clone()),
        rows: snapshot
            .rows
            .iter()
            .map(|row| ResultRow {
                id: row.id.clone(),
                name: row.name.clone(),
                text: row.result.translated_text.clone(),
                status: match row.status {
                    TranslationStatus::Pending => RowStatus::Pending,
                    TranslationStatus::Success => RowStatus::Success,
                    TranslationStatus::Error => RowStatus::Error,
                },
                error: row.result.error.as_ref().map(|error| error.message.clone()),
                favorite: row.favorite,
                retryable: row
                    .result
                    .error
                    .as_ref()
                    .is_none_or(|error| error.retryable),
            })
            .collect(),
        translating: snapshot.translating,
        ocr_text: snapshot.ocr_text.clone(),
        ocr_tokens: crate::application::result_window::copy_tokens(&snapshot.ocr_text, vec![])
            .into_iter()
            .map(|token| snaplingo_ui::result::ResultCopyToken {
                label: token.label,
                value: token.value,
            })
            .collect(),
        ocr_image: image,
        ocr_metadata: metadata.join(" · "),
        ocr_language: snapshot.ocr_language.clone(),
        ocr_confidence: snapshot
            .preferences
            .ocr
            .show_confidence
            .then_some(snapshot.ocr_confidence)
            .flatten(),
        ocr_running: snapshot.ocr_running,
        dialog_open: snapshot.dialog_open,
        favorite_pending: snapshot.favorite_pending,
        ocr_favorite: snapshot.ocr_favorite,
        error: snapshot.error.clone(),
    }
}
pub(crate) fn window_event(_event_loop: &ActiveEventLoop, id: WindowId, event: &WindowEvent) {
    VIEW.with_borrow_mut(|view| {
        let Some(view) = view.as_mut().filter(|view| view.window.id() == id) else {
            return;
        };
        view.accessibility.process_event(&view.window, event);
        match event {
            WindowEvent::CloseRequested => view.input(ResultInput::Activate(
                snaplingo_ui::result::ResultTarget::Close,
            )),
            WindowEvent::RedrawRequested => {
                if let Err(error) = view.draw() {
                    log::error!("Failed to draw result window: {error}");
                    view.window.set_visible(false);
                }
            }
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                view.window.request_redraw()
            }
            WindowEvent::ThemeChanged(_) => {
                let snapshot = view.snapshot.clone();
                view.update(snapshot);
            }
            WindowEvent::Focused(false) => {
                view.input(ResultInput::FocusLost);
                if view.opened_at.elapsed() > Duration::from_millis(300)
                    && view.snapshot.preferences.translation.hide_on_blur
                    && !view.ui.pinned()
                    && !view.controller.snapshot().dialog_open
                {
                    view.window.set_visible(false);
                    view.controller.dispatch(Action::Close);
                }
            }
            WindowEvent::Moved(position) => {
                let position = position.to_logical::<f64>(view.window.scale_factor());
                view.controller.dispatch(Action::Moved(
                    position.x.round() as i32,
                    position.y.round() as i32,
                ));
            }
            WindowEvent::CursorMoved { position, .. } => {
                let position = position.to_logical::<f32>(view.window.scale_factor());
                view.cursor = Point {
                    x: position.x,
                    y: position.y,
                };
                view.input(ResultInput::PointerMove(view.cursor));
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => view.input(ResultInput::PointerDown(view.cursor)),
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => view.input(ResultInput::PointerUp),
            WindowEvent::MouseWheel { delta, .. } => view.input(ResultInput::Scroll(match delta {
                MouseScrollDelta::LineDelta(_, y) => -y * 32.,
                MouseScrollDelta::PixelDelta(p) => -(p.y / view.window.scale_factor()) as f32,
            })),
            WindowEvent::ModifiersChanged(modifiers) => {
                let m = modifiers.state();
                view.modifiers = Modifiers {
                    shift: m.shift_key(),
                    alt: m.alt_key(),
                    control: m.control_key(),
                    meta: m.super_key(),
                };
            }
            WindowEvent::Ime(ime) => match ime {
                Ime::Preedit(text, selection) => view.input(ResultInput::Preedit(
                    text.clone(),
                    selection.map(|(start, end)| start..end),
                )),
                Ime::Commit(text) => view.input(ResultInput::Commit(text.clone())),
                Ime::Disabled => view.input(ResultInput::ImeDisabled),
                _ => {}
            },
            WindowEvent::KeyboardInput { event, .. } if event.state.is_pressed() => {
                if view.ui.text_edit().is_some_and(|edit| edit.is_composing()) {
                    return;
                }
                if view.ui.focused() == Some(&snaplingo_ui::result::ResultTarget::Source)
                    && !view.modifiers.control
                    && !view.modifiers.meta
                    && !view.modifiers.alt
                {
                    if let Some(text) = event
                        .text
                        .as_ref()
                        .filter(|text| text.chars().all(|c| !c.is_control()))
                    {
                        view.input(ResultInput::Text(text.to_string()));
                        return;
                    }
                }
                if let Some(key) = key(&event.logical_key) {
                    view.input(ResultInput::Key(key, view.modifiers));
                }
            }
            _ => {}
        }
    });
}
pub(crate) fn accessibility_event(event: &accesskit_winit::Event) {
    VIEW.with_borrow_mut(|view| {
        let Some(view) = view
            .as_mut()
            .filter(|view| view.window.id() == event.window_id)
        else {
            return;
        };
        match &event.window_event {
            AccessibilityEvent::ActionRequested(request) => {
                let inputs = view.projection.lock().unwrap().actions(request);
                for input in inputs {
                    view.input(input);
                }
            }
            AccessibilityEvent::InitialTreeRequested => view.window.request_redraw(),
            AccessibilityEvent::AccessibilityDeactivated => view.sent = None,
        }
    });
}
fn key(key: &NativeKey) -> Option<Key> {
    Some(match key {
        NativeKey::Character(text) => Key::Character(text.chars().next()?.to_ascii_uppercase()),
        NativeKey::Named(key) => match key {
            NamedKey::Tab => Key::Tab,
            NamedKey::Enter => Key::Enter,
            NamedKey::Space => Key::Space,
            NamedKey::Escape => Key::Escape,
            NamedKey::ArrowLeft => Key::ArrowLeft,
            NamedKey::ArrowRight => Key::ArrowRight,
            NamedKey::ArrowUp => Key::ArrowUp,
            NamedKey::ArrowDown => Key::ArrowDown,
            NamedKey::Home => Key::Home,
            NamedKey::End => Key::End,
            NamedKey::PageUp => Key::PageUp,
            NamedKey::PageDown => Key::PageDown,
            NamedKey::Backspace => Key::Backspace,
            NamedKey::Delete => Key::Delete,
            _ => return None,
        },
        _ => return None,
    })
}

pub(crate) fn close_all() {
    VIEW.with_borrow_mut(Option::take);
}

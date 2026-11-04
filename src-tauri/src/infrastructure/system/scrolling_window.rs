use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter};
#[cfg(not(target_os = "macos"))]
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};

#[cfg(not(target_os = "macos"))]
use super::window_position::{desktop_rect, set_desktop_position};
use crate::application::capture::{
    CaptureSessionSource, ScrollingCaptureGeometry, ScrollingCaptureHost, ScrollingWindowOptions,
};
use crate::domain::capture::{
    CaptureSessionId, LogicalRect, MonitorSnapshot, ScrollingCaptureView, ScrollingShortcutAction,
};
#[cfg(not(target_os = "macos"))]
use crate::domain::capture::{
    DesktopPosition, LogicalPoint, PhysicalPoint, ScrollingCaptureOutputAction,
};

const PANEL_WIDTH: f64 = 408.0;
const COMPACT_HEIGHT: f64 = 132.0;

pub(crate) struct TauriScrollingCaptureHost {
    app: AppHandle,
    source: Arc<dyn CaptureSessionSource>,
    shortcuts: Mutex<HashMap<String, Vec<String>>>,
    #[cfg(target_os = "macos")]
    overlays: Arc<super::scrolling_overlays_macos::ScrollingOverlays>,
    #[cfg(target_os = "macos")]
    preview_cache: Mutex<HashMap<String, String>>,
    #[cfg(target_os = "macos")]
    native_sessions: Mutex<HashMap<String, Arc<super::scrolling_native_macos::Session>>>,
}

impl TauriScrollingCaptureHost {
    pub fn new(app: AppHandle, source: Arc<dyn CaptureSessionSource>) -> Self {
        Self {
            app,
            source,
            shortcuts: Mutex::new(HashMap::new()),
            #[cfg(target_os = "macos")]
            overlays: Arc::new(super::scrolling_overlays_macos::ScrollingOverlays),
            #[cfg(target_os = "macos")]
            preview_cache: Mutex::new(HashMap::new()),
            #[cfg(target_os = "macos")]
            native_sessions: Mutex::new(HashMap::new()),
        }
    }

    #[cfg(target_os = "macos")]
    async fn open_native(
        &self,
        view: &ScrollingCaptureView,
        geometry: &ScrollingCaptureGeometry,
    ) -> crate::Result<bool> {
        let session = Arc::new(super::scrolling_native_macos::Session::default());
        self.native_sessions
            .lock()
            .unwrap()
            .insert(view.id.clone(), session.clone());
        self.preview_cache
            .lock()
            .unwrap()
            .insert(view.id.clone(), view.preview_base64.clone());
        let view = view.clone();
        let geometry = geometry.clone();
        let overlays = self.overlays.clone();
        on_main(&self.app, move |app| {
            super::capture_window::yield_capture_window_focus();
            let width = PANEL_WIDTH * view.ui_scale;
            let height = COMPACT_HEIGHT * view.ui_scale;
            let (x, y) = outside_position(&geometry, width, height).unwrap_or_else(|| {
                let screen = geometry.displays.first().unwrap_or(&geometry.viewport);
                (screen.x + screen.width - width - 12.0, screen.y + 12.0)
            });
            let panel_rect = LogicalRect {
                x,
                y,
                width,
                height,
            };
            let id = view.id.clone();
            let event_id = id.clone();
            super::scrolling_controls_macos::open(view.clone(), &panel_rect, move |action| {
                super::scrolling_native_macos::dispatch(
                    app.clone(),
                    event_id.clone(),
                    session.clone(),
                    action,
                );
            })?;
            overlays.open(&id, &geometry.viewport, view.ui_scale)?;
            if let Some(rect) = preview_placement(&geometry, view.ui_scale, Some(&panel_rect)) {
                overlays.open_preview(&id, &rect)?;
                overlays.update_preview(&id, &view.preview_base64);
                Ok(true)
            } else {
                Ok(false)
            }
        })
        .await
    }
}

#[async_trait::async_trait]
impl ScrollingCaptureHost for TauriScrollingCaptureHost {
    async fn open(
        &self,
        view: &ScrollingCaptureView,
        geometry: &ScrollingCaptureGeometry,
    ) -> crate::Result<ScrollingWindowOptions> {
        #[cfg(target_os = "macos")]
        let preview_expanded = self.open_native(view, geometry).await?;
        #[cfg(not(target_os = "macos"))]
        let preview_expanded = {
            let id = view.id.clone();
            let ui_scale = view.ui_scale;
            let geometry = geometry.clone();
            let preview_expanded = on_main(&self.app, move |app| {
                super::capture_window::yield_capture_window_focus();
                let window = WebviewWindowBuilder::new(
                    &app,
                    &id,
                    WebviewUrl::App(format!("index.html?window=scrolling&captureId={id}").into()),
                )
                .title("SnapLingo 长截图")
                .inner_size(PANEL_WIDTH * ui_scale, COMPACT_HEIGHT * ui_scale)
                .decorations(false)
                .shadow(false)
                .always_on_top(true)
                .visible_on_all_workspaces(true)
                .skip_taskbar(true)
                .focused(false)
                .focusable(false)
                .content_protected(true)
                .visible(false)
                .build()
                .map_err(|error| error.to_string())?;
                place(
                    &window,
                    geometry.viewport.x,
                    geometry.viewport.y,
                    geometry.desktop_scale,
                )?;
                let frame = desktop_rect(&window, geometry.desktop_scale)?;
                let (x, y) =
                    outside_position(&geometry, frame.width, frame.height).unwrap_or_else(|| {
                        let screen = geometry.displays.first().unwrap_or(&geometry.viewport);
                        (
                            screen.x + screen.width - frame.width - 12.0,
                            screen.y + 12.0,
                        )
                    });
                place(&window, x, y, geometry.desktop_scale)?;
                let event_app = app.clone();
                let event_id = id.clone();
                window.on_window_event(move |event| {
                    if matches!(event, WindowEvent::Destroyed) {
                        let app = event_app.clone();
                        let id = event_id.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Some(runtime) = app
                                .try_state::<crate::AppState>()
                                .map(|state| state.capture.runtime.clone())
                            {
                                let _ = runtime.cancel_scrolling_capture(&id).await;
                            }
                        });
                    }
                });
                let padding = 2.0 * ui_scale;
                let outline = WebviewWindowBuilder::new(
                    &app,
                    outline_label(&id),
                    WebviewUrl::App(
                        format!(
                            "index.html?window=scrolling-outline&captureId={id}&scale={ui_scale}&desktopScale={}",
                            geometry.desktop_scale.unwrap_or(0.0),
                        ).into(),
                    ),
                )
                .title("长截图选区")
                .inner_size(
                    geometry.viewport.width + padding * 2.0,
                    geometry.viewport.height + padding * 2.0,
                )
                .transparent(true)
                .decorations(false)
                .shadow(false)
                .always_on_top(true)
                .visible_on_all_workspaces(true)
                .skip_taskbar(true)
                .focused(false)
                .focusable(false)
                .content_protected(true)
                .visible(false)
                .build()
                .map_err(|error| error.to_string())?;
                outline
                    .set_ignore_cursor_events(true)
                    .map_err(|error| error.to_string())?;
                place(
                    &outline,
                    geometry.viewport.x - padding,
                    geometry.viewport.y - padding,
                    geometry.desktop_scale,
                )?;
                if let Some(scale) = geometry.desktop_scale {
                    let ratio = scale / outline.scale_factor().map_err(|error| error.to_string())?;
                    outline
                        .set_size(tauri::LogicalSize::new(
                            (geometry.viewport.width + padding * 2.0) * ratio,
                            (geometry.viewport.height + padding * 2.0) * ratio,
                        ))
                        .map_err(|error| error.to_string())?;
                }
                outline.show().map_err(|error| error.to_string())?;
                let preview_expanded = if let Some(rect) = preview_placement(
                    &geometry,
                    ui_scale,
                    Some(&desktop_rect(&window, geometry.desktop_scale)?),
                ) {
                    let preview = WebviewWindowBuilder::new(
                        &app,
                        preview_label(&id),
                        WebviewUrl::App(
                            format!("index.html?window=scrolling-preview&captureId={id}").into(),
                        ),
                    )
                    .title("长截图预览")
                    .inner_size(rect.width, rect.height)
                    .transparent(true)
                    .decorations(false)
                    .shadow(false)
                    .always_on_top(true)
                    .visible_on_all_workspaces(true)
                    .skip_taskbar(true)
                    .focused(false)
                    .focusable(false)
                    .content_protected(true)
                    .visible(false)
                    .build()
                    .map_err(|error| error.to_string())?;
                    preview
                        .set_ignore_cursor_events(true)
                        .map_err(|error| error.to_string())?;
                    place(&preview, rect.x, rect.y, geometry.desktop_scale)?;
                    if let Some(scale) = geometry.desktop_scale {
                        let ratio =
                            scale / preview.scale_factor().map_err(|error| error.to_string())?;
                        preview
                            .set_size(tauri::LogicalSize::new(
                                rect.width * ratio,
                                rect.height * ratio,
                            ))
                            .map_err(|error| error.to_string())?;
                    }
                    preview.show().map_err(|error| error.to_string())?;
                    true
                } else {
                    false
                };
                window.show().map_err(|error| error.to_string())?;
                Ok(preview_expanded)
            })
            .await?;
            preview_expanded
        };

        let mut installed = Vec::new();
        let mut actions = Vec::new();
        let undo = if cfg!(target_os = "macos") {
            "Super+Z"
        } else {
            "Control+Z"
        };
        for (key, action) in [
            ("Enter", ScrollingShortcutAction::Complete),
            ("Escape", ScrollingShortcutAction::Cancel),
            (undo, ScrollingShortcutAction::Undo),
        ] {
            match super::shortcut::is_shortcut_registered(&self.app, key) {
                Ok(false) => {}
                Ok(true) => continue,
                Err(error) => {
                    log::debug!("Scrolling capture key {key} is unavailable: {error}");
                    continue;
                }
            }
            let app = self.app.clone();
            let id = view.id.clone();
            let intent = action.clone();
            #[cfg(target_os = "macos")]
            let native_session = self
                .native_sessions
                .lock()
                .unwrap()
                .get(&id)
                .cloned()
                .unwrap();
            let registered =
                super::shortcut::register_shortcut_on_release(&self.app, key, move || {
                    #[cfg(target_os = "macos")]
                    {
                        use super::scrolling_controls_macos::Action;
                        let action = match intent {
                            ScrollingShortcutAction::Copy => Action::Copy,
                            ScrollingShortcutAction::Complete => Action::Complete,
                            ScrollingShortcutAction::Undo => Action::Undo,
                            ScrollingShortcutAction::Cancel => Action::Cancel,
                        };
                        super::scrolling_native_macos::dispatch(
                            app.clone(),
                            id.clone(),
                            native_session.clone(),
                            action,
                        );
                    }
                    #[cfg(not(target_os = "macos"))]
                    {
                        let app = app.clone();
                        let id = id.clone();
                        let action = intent.clone();
                        tauri::async_runtime::spawn(async move {
                            let Some(runtime) = app
                                .try_state::<crate::AppState>()
                                .map(|state| state.capture.runtime.clone())
                            else {
                                return;
                            };
                            let result = match action {
                                ScrollingShortcutAction::Copy => {
                                    runtime
                                        .finish_scrolling_capture(
                                            &id,
                                            ScrollingCaptureOutputAction::Copy,
                                        )
                                        .await
                                }
                                ScrollingShortcutAction::Complete => {
                                    runtime
                                        .finish_scrolling_capture(
                                            &id,
                                            ScrollingCaptureOutputAction::Complete,
                                        )
                                        .await
                                }
                                ScrollingShortcutAction::Cancel => {
                                    runtime.cancel_scrolling_capture(&id).await
                                }
                                ScrollingShortcutAction::Undo => {
                                    runtime.undo_scrolling_capture(&id).await.map(|view| {
                                        let _ = app.emit_to(&id, "scrolling-capture-updated", view);
                                    })
                                }
                            };
                            if let Err(error) = result {
                                let _ =
                                    app.emit_to(&id, "scrolling-capture-error", error.to_string());
                            }
                        });
                    }
                });
            match registered {
                Ok(()) => {
                    installed.push(key.to_string());
                    actions.push(action);
                }
                Err(error) => log::debug!("Scrolling capture key {key} is unavailable: {error}"),
            }
        }
        self.shortcuts
            .lock()
            .unwrap()
            .insert(view.id.clone(), installed);
        Ok(ScrollingWindowOptions {
            shortcuts: actions,
            preview_expanded,
        })
    }

    async fn capture_snapshots(
        &self,
        id: &str,
        geometry: &ScrollingCaptureGeometry,
    ) -> crate::Result<Vec<MonitorSnapshot>> {
        #[cfg(target_os = "macos")]
        let overlays = self.overlays.clone();
        #[cfg(target_os = "macos")]
        let outline_id = id.to_string();
        #[cfg(target_os = "macos")]
        let hidden_outline_id = outline_id.clone();
        #[cfg(not(target_os = "macos"))]
        let labels = [id.to_string(), preview_label(id), outline_label(id)];
        let viewport = geometry.viewport.clone();
        #[cfg(not(target_os = "macos"))]
        let scale = geometry.desktop_scale;
        let hidden = on_main(&self.app, move |_app| {
            #[cfg(target_os = "macos")]
            let hidden: Vec<String> = Vec::new();
            #[cfg(not(target_os = "macos"))]
            let mut hidden = Vec::new();
            #[cfg(not(target_os = "macos"))]
            for label in labels {
                if let Some(window) = _app.get_webview_window(&label) {
                    if window.is_visible().map_err(|error| error.to_string())?
                        && overlaps(&desktop_rect(&window, scale)?, &viewport)
                    {
                        window.hide().map_err(|error| error.to_string())?;
                        hidden.push(label);
                    }
                }
            }
            #[cfg(target_os = "macos")]
            let outline_hidden = overlays.hide_if_overlapping(&hidden_outline_id, &viewport);
            #[cfg(target_os = "macos")]
            let preview_hidden =
                overlays.hide_preview_if_overlapping(&hidden_outline_id, &viewport);
            #[cfg(target_os = "macos")]
            let controls_hidden =
                super::scrolling_controls_macos::hide_if_overlapping(&hidden_outline_id, &viewport);
            #[cfg(not(target_os = "macos"))]
            let outline_hidden = false;
            #[cfg(not(target_os = "macos"))]
            let preview_hidden = false;
            #[cfg(not(target_os = "macos"))]
            let controls_hidden = false;
            Ok((hidden, outline_hidden, preview_hidden, controls_hidden))
        })
        .await?;
        if !hidden.0.is_empty() || hidden.1 || hidden.2 || hidden.3 {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        let result = async {
            let mut snapshots = Vec::new();
            for monitor in &geometry.monitors {
                snapshots.push(self.source.capture_monitor_snapshot(&monitor.id).await?);
            }
            Ok(snapshots)
        }
        .await;
        if !hidden.0.is_empty() || hidden.1 || hidden.2 || hidden.3 {
            #[cfg(target_os = "macos")]
            let overlays = self.overlays.clone();
            on_main(&self.app, move |_app| {
                #[cfg(target_os = "macos")]
                if hidden.1 {
                    overlays.show(&outline_id);
                }
                #[cfg(target_os = "macos")]
                if hidden.2 {
                    overlays.set_preview_visible(&outline_id, true);
                }
                #[cfg(target_os = "macos")]
                if hidden.3 {
                    super::scrolling_controls_macos::show(&outline_id);
                }
                #[cfg(not(target_os = "macos"))]
                for label in hidden.0 {
                    if let Some(window) = _app.get_webview_window(&label) {
                        window.show().map_err(|error| error.to_string())?;
                    }
                }
                Ok(())
            })
            .await?;
        }

        result
    }

    fn publish(&self, view: &ScrollingCaptureView) {
        #[cfg(target_os = "macos")]
        if !view.preview_base64.is_empty()
            && self
                .preview_cache
                .lock()
                .unwrap()
                .insert(view.id.clone(), view.preview_base64.clone())
                .as_deref()
                != Some(view.preview_base64.as_str())
        {
            let overlays = self.overlays.clone();
            let id = view.id.clone();
            let preview = view.preview_base64.clone();
            let _ = self
                .app
                .run_on_main_thread(move || overlays.update_preview(&id, &preview));
        }
        #[cfg(not(target_os = "macos"))]
        for label in [&view.id, &preview_label(&view.id)] {
            let _ = self.app.emit_to(label, "scrolling-capture-updated", view);
        }
        #[cfg(target_os = "macos")]
        {
            let view = view.clone();
            if let Some(session) = self.native_sessions.lock().unwrap().get(&view.id).cloned() {
                session.start(self.app.clone(), view.id.clone());
            }
            let _ = self
                .app
                .run_on_main_thread(move || super::scrolling_controls_macos::update(view));
        }
    }

    async fn set_preview_visible(&self, id: &str, visible: bool) -> crate::Result<bool> {
        #[cfg(target_os = "macos")]
        {
            let id = id.to_owned();
            let overlays = self.overlays.clone();
            return on_main(&self.app, move |_| {
                Ok(overlays.set_preview_visible(&id, visible))
            })
            .await;
        }
        #[cfg(not(target_os = "macos"))]
        {
            let label = preview_label(id);
            on_main(&self.app, move |app| {
                if let Some(window) = app.get_webview_window(&label) {
                    if visible {
                        window.show()
                    } else {
                        window.hide()
                    }
                    .map_err(|error| error.to_string())?;
                    Ok(visible)
                } else {
                    Ok(false)
                }
            })
            .await
        }
    }

    async fn close(
        &self,
        id: &str,
        session_id: &CaptureSessionId,
        completed: Option<bool>,
    ) -> crate::Result<()> {
        #[cfg(target_os = "macos")]
        self.preview_cache.lock().unwrap().remove(id);
        #[cfg(target_os = "macos")]
        if let Some(session) = self.native_sessions.lock().unwrap().remove(id) {
            session.stop();
        }
        let keys = self
            .shortcuts
            .lock()
            .unwrap()
            .remove(id)
            .unwrap_or_default();
        for key in keys {
            if let Err(error) = super::shortcut::unregister_shortcut(&self.app, &key) {
                log::debug!("Could not release scrolling capture key {key}: {error}");
            }
        }
        if let Some(completed) = completed {
            let _ = self.app.emit_to(
                "capture",
                "scrolling-capture-ended",
                serde_json::json!({ "sessionId": session_id.0, "completed": completed }),
            );
        }
        let label = id.to_string();
        #[cfg(target_os = "macos")]
        let overlays = self.overlays.clone();
        on_main(&self.app, move |_app| {
            #[cfg(target_os = "macos")]
            super::scrolling_controls_macos::close(&label);
            #[cfg(target_os = "macos")]
            overlays.close(&label);
            #[cfg(target_os = "macos")]
            overlays.close_preview(&label);
            #[cfg(not(target_os = "macos"))]
            let labels = [preview_label(&label), outline_label(&label), label];
            #[cfg(not(target_os = "macos"))]
            for label in labels {
                if let Some(window) = _app.get_webview_window(&label) {
                    window.close().map_err(|error| error.to_string())?;
                }
            }
            Ok(())
        })
        .await
    }
}

#[cfg(not(target_os = "macos"))]
fn preview_label(id: &str) -> String {
    format!("{id}-preview")
}
#[cfg(not(target_os = "macos"))]
fn outline_label(id: &str) -> String {
    format!("{id}-region")
}

fn preview_placement(
    geometry: &ScrollingCaptureGeometry,
    ui_scale: f64,
    controls: Option<&LogicalRect>,
) -> Option<LogicalRect> {
    let view = &geometry.viewport;
    let mut slots = Vec::new();
    for screen in &geometry.displays {
        for (left, right) in [
            (
                screen.x + 12.0,
                (view.x - 12.0).min(screen.x + screen.width - 12.0),
            ),
            (
                (view.x + view.width + 12.0).max(screen.x + 12.0),
                screen.x + screen.width - 12.0,
            ),
        ] {
            let available = right - left;
            if available < 96.0 * ui_scale || screen.height < 96.0 * ui_scale {
                continue;
            }
            let width = available.min(240.0 * ui_scale);
            let height = view.height.min(screen.height - 24.0).min(360.0 * ui_scale);
            let y = (view.y + view.height - height).clamp(
                screen.y + 12.0,
                (screen.y + screen.height - height - 12.0).max(screen.y + 12.0),
            );
            let mut rect = LogicalRect {
                x: if left > screen.x + 12.0 {
                    right - width
                } else {
                    left
                },
                y,
                width,
                height,
            };
            if let Some(controls) = controls.filter(|controls| overlaps(controls, &rect)) {
                let remaining = rect.y + rect.height - controls.y - controls.height - 12.0;
                if remaining < 96.0 * ui_scale {
                    continue;
                }
                rect.y += rect.height - remaining;
                rect.height = remaining;
            }
            slots.push((available, rect));
        }
    }
    slots
        .into_iter()
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, rect)| rect)
}

#[cfg(not(target_os = "macos"))]
fn place(window: &tauri::WebviewWindow, x: f64, y: f64, scale: Option<f64>) -> Result<(), String> {
    let position = match scale {
        Some(scale) => DesktopPosition::Physical(PhysicalPoint {
            x: (x * scale).round() as i32,
            y: (y * scale).round() as i32,
        }),
        None => DesktopPosition::Logical(LogicalPoint { x, y }),
    };
    set_desktop_position(window, position)
}

pub(super) fn overlaps(a: &LogicalRect, b: &LogicalRect) -> bool {
    a.x < b.x + b.width && a.x + a.width > b.x && a.y < b.y + b.height && a.y + a.height > b.y
}

fn outside_position(
    geometry: &ScrollingCaptureGeometry,
    width: f64,
    height: f64,
) -> Option<(f64, f64)> {
    let view = &geometry.viewport;
    for screen in &geometry.displays {
        let clamp_x = |x: f64| {
            x.max(screen.x)
                .min((screen.x + screen.width - width).max(screen.x))
        };
        let clamp_y = |y: f64| {
            y.max(screen.y)
                .min((screen.y + screen.height - height).max(screen.y))
        };
        for (x, y) in [
            (clamp_x(view.x), view.y + view.height + 12.0),
            (clamp_x(view.x), view.y - height - 12.0),
            (view.x + view.width + 12.0, clamp_y(view.y)),
            (view.x - width - 12.0, clamp_y(view.y)),
        ] {
            let candidate = LogicalRect {
                x,
                y,
                width,
                height,
            };
            if x >= screen.x
                && y >= screen.y
                && x + width <= screen.x + screen.width
                && y + height <= screen.y + screen.height
                && !overlaps(&candidate, view)
            {
                return Some((x, y));
            }
        }
    }
    None
}

async fn on_main<T: Send + 'static>(
    app: &AppHandle,
    operation: impl FnOnce(AppHandle) -> Result<T, String> + Send + 'static,
) -> crate::Result<T> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let target = app.clone();
    app.run_on_main_thread(move || {
        let _ = sender.send(operation(target));
    })
    .map_err(|error| error.to_string())?;
    receiver
        .await
        .map_err(|error| error.to_string())?
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::capture::PhysicalRect;

    fn geometry(viewport: LogicalRect) -> ScrollingCaptureGeometry {
        ScrollingCaptureGeometry {
            displays: vec![LogicalRect {
                x: -1920.0,
                y: -100.0,
                width: 1920.0,
                height: 1080.0,
            }],
            viewport,
            desktop_scale: Some(2.0),
            monitors: vec![MonitorSnapshot {
                id: "left".into(),
                logical_bounds: LogicalRect {
                    x: -1920.0,
                    y: -100.0,
                    width: 1920.0,
                    height: 1080.0,
                },
                physical_bounds: PhysicalRect {
                    x: -3840,
                    y: -200,
                    width: 3840,
                    height: 2160,
                },
                scale_factor: 2.0,
                png_data: Vec::new(),
            }],
        }
    }

    #[test]
    fn places_the_panel_outside_a_viewport_on_a_negative_coordinate_display() {
        let viewport = LogicalRect {
            x: -1700.0,
            y: 50.0,
            width: 800.0,
            height: 400.0,
        };
        let geometry = geometry(viewport.clone());
        assert_eq!(
            outside_position(&geometry, 360.0, 460.0),
            Some((-1700.0, 462.0))
        );
    }

    #[test]
    fn a_full_screen_viewport_requires_hiding_the_panel_during_sampling() {
        let geometry = geometry(LogicalRect {
            x: -1920.0,
            y: -100.0,
            width: 1920.0,
            height: 1080.0,
        });
        assert_eq!(outside_position(&geometry, 360.0, 460.0), None);
        assert_eq!(outside_position(&geometry, 360.0, 132.0), None);
        assert!(preview_placement(&geometry, 1.0, None).is_none());
    }

    #[test]
    fn preview_uses_the_wider_side_and_aligns_with_the_viewport_bottom() {
        let geometry = geometry(LogicalRect {
            x: -1700.0,
            y: 50.0,
            width: 800.0,
            height: 400.0,
        });
        assert_eq!(
            preview_placement(&geometry, 1.0, None),
            Some(LogicalRect {
                x: -252.0,
                y: 90.0,
                width: 240.0,
                height: 360.0,
            })
        );
    }

    #[test]
    fn preview_prefers_the_right_when_both_sides_have_equal_room() {
        let geometry = geometry(LogicalRect {
            x: -1360.0,
            y: 50.0,
            width: 800.0,
            height: 400.0,
        });
        assert_eq!(preview_placement(&geometry, 1.0, None).unwrap().x, -252.0);
    }
}

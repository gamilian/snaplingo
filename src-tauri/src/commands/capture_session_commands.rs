use std::path::{Path, PathBuf};

use base64::Engine;
use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, State, WebviewUrl,
    WebviewWindowBuilder,
};

use crate::application::services::{
    image_composition_service::{ImageAnnotation, PngPlacement},
    CaptureOutputService, PinnedImageGroupSwitch, PinnedImageService,
};
use crate::domain::capture::{
    AnnotationCommand, CaptureOutputAction, CaptureSessionId, CaptureSessionView, LogicalPoint,
    LogicalRect, MonitorSnapshotView, PhysicalPoint, PhysicalRect, PinnedImageView,
};
use crate::domain::ocr::{OcrRequest, OcrResult};
use crate::infrastructure::system::screenshot::MonitorSnapshot;

const CAPTURE_WINDOW_LABEL: &str = "capture";
const PIN_WINDOW_MAX_WIDTH: f64 = 900.0;
const PIN_WINDOW_MAX_HEIGHT: f64 = 700.0;

#[derive(Debug, PartialEq)]
struct CaptureImageCompositionPlan {
    width: u32,
    height: u32,
    placements: Vec<CaptureImagePlacement>,
}

#[derive(Debug, PartialEq)]
struct CaptureImagePlacement {
    snapshot_index: usize,
    source_rect: PhysicalRect,
    destination_rect: PhysicalRect,
}

#[derive(Debug, PartialEq)]
struct PinnedWindowVisibilityChange {
    label: String,
    visible: bool,
}

#[tauri::command]
pub async fn open_capture_window(
    mode: String,
    app: AppHandle,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    open_capture_window_for_mode(&app, &state, &mode).await
}

pub async fn open_capture_window_for_mode(
    app: &AppHandle,
    state: &crate::AppState,
    mode: &str,
) -> Result<(), String> {
    let session = state
        .capture_session_service
        .create_session()
        .await
        .map_err(|e| e.to_string())?;
    let bounds = capture_window_bounds(&session.monitors)
        .ok_or_else(|| "Cannot open capture window without monitor bounds".to_string())?;
    open_capture_window_for_session(app, mode, &session.id.0, &bounds)
}

#[tauri::command]
pub async fn create_capture_session(
    state: State<'_, crate::AppState>,
) -> Result<CaptureSessionView, String> {
    state
        .capture_session_service
        .create_session()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_capture_session(
    session_id: String,
    state: State<'_, crate::AppState>,
) -> Result<CaptureSessionView, String> {
    let session = state
        .capture_session_service
        .get_session(&CaptureSessionId(session_id))
        .map_err(|e| e.to_string())?;

    Ok(CaptureSessionView {
        id: session.id,
        monitors: session.snapshots.iter().map(snapshot_to_view).collect(),
        candidates: session.candidates,
    })
}

#[tauri::command]
pub async fn cancel_capture_session(
    session_id: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state
        .capture_session_service
        .cancel_session(&CaptureSessionId(session_id))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn render_capture_output(
    session_id: String,
    rect: LogicalRect,
    annotations: Vec<AnnotationCommand>,
    state: State<'_, crate::AppState>,
) -> Result<String, String> {
    let session_id = CaptureSessionId(session_id);
    let png_data = render_capture_png(&session_id, &rect, &annotations, &state)?;

    Ok(base64::engine::general_purpose::STANDARD.encode(png_data))
}

#[tauri::command]
pub fn default_capture_save_path() -> Result<String, String> {
    let base_dir = dirs::download_dir()
        .or_else(dirs::picture_dir)
        .or_else(dirs::home_dir)
        .unwrap_or_else(std::env::temp_dir);
    let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();

    Ok(capture_save_path(&base_dir, &timestamp)
        .to_string_lossy()
        .to_string())
}

#[tauri::command]
pub async fn output_capture(
    session_id: String,
    rect: LogicalRect,
    annotations: Vec<AnnotationCommand>,
    action: CaptureOutputAction,
    app: AppHandle,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    let session_id = CaptureSessionId(session_id);
    let png_data = render_capture_png(&session_id, &rect, &annotations, &state)?;

    match action {
        CaptureOutputAction::Save { path } => {
            state
                .capture_output_service
                .save_png(&png_data, std::path::Path::new(&path))
                .await
                .map_err(|e| e.to_string())?;
            Ok(())
        }
        CaptureOutputAction::Copy => state
            .capture_output_service
            .copy_png(&png_data)
            .await
            .map_err(|e| e.to_string()),
        CaptureOutputAction::Pin => pin_capture_png(&app, state.inner(), png_data),
    }
}

#[tauri::command]
pub fn get_pinned_image(
    image_id: String,
    state: State<'_, crate::AppState>,
) -> Result<PinnedImageView, String> {
    state
        .inner()
        .pinned_image_service
        .get_pinned_image(&image_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_pinned_image(
    image_id: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state
        .inner()
        .pinned_image_service
        .remove_pinned_image(&image_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn copy_pinned_image(
    image_id: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    let png_data = state
        .inner()
        .pinned_image_service
        .get_pinned_png(&image_id)
        .map_err(|e| e.to_string())?;

    state
        .inner()
        .capture_output_service
        .copy_png(&png_data)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_pinned_image(
    image_id: String,
    path: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    save_pinned_png_by_id(
        &state.inner().pinned_image_service,
        &state.inner().capture_output_service,
        &image_id,
        Path::new(&path),
    )
    .await
}

#[tauri::command]
pub fn toggle_pinned_images_visibility(app: AppHandle) -> Result<Option<bool>, String> {
    let pinned_windows: Vec<_> = app
        .webview_windows()
        .into_iter()
        .filter(|(label, _)| is_pinned_window_label(label))
        .map(|(_, window)| window)
        .collect();
    let visibility: Vec<bool> = pinned_windows
        .iter()
        .map(|window| window.is_visible().unwrap_or(false))
        .collect();
    let Some(next_visible) = next_pinned_windows_visible_state(&visibility) else {
        return Ok(None);
    };

    for window in pinned_windows {
        if next_visible {
            window.show().map_err(|e| e.to_string())?;
        } else {
            window.hide().map_err(|e| e.to_string())?;
        }
    }

    Ok(Some(next_visible))
}

#[tauri::command]
pub fn switch_pinned_image_group(
    app: AppHandle,
    state: State<'_, crate::AppState>,
) -> Result<Option<u32>, String> {
    switch_pinned_image_group_for_state(&app, state.inner())
}

pub fn switch_pinned_image_group_for_state(
    app: &AppHandle,
    state: &crate::AppState,
) -> Result<Option<u32>, String> {
    let Some(group_switch) = state.pinned_image_service.switch_to_next_group() else {
        return Ok(None);
    };

    for change in pinned_group_window_visibility_changes(&group_switch) {
        let Some(window) = app.get_webview_window(&change.label) else {
            continue;
        };

        if change.visible {
            window.show().map_err(|e| e.to_string())?;
        } else {
            window.hide().map_err(|e| e.to_string())?;
        }
    }

    Ok(Some(group_switch.next_group))
}

#[tauri::command]
pub fn move_pinned_image_to_next_group(
    image_id: String,
    app: AppHandle,
    state: State<'_, crate::AppState>,
) -> Result<u32, String> {
    let next_group = state
        .inner()
        .pinned_image_service
        .move_pinned_image_to_next_group(&image_id)
        .map_err(|e| e.to_string())?;
    let change = moved_pinned_image_window_visibility_change(&image_id);

    if let Some(window) = app.get_webview_window(&change.label) {
        window.hide().map_err(|e| e.to_string())?;
    }

    Ok(next_group)
}

async fn save_pinned_png_by_id(
    pinned_images: &PinnedImageService,
    output: &CaptureOutputService,
    image_id: &str,
    path: &Path,
) -> Result<(), String> {
    let png_data = pinned_images
        .get_pinned_png(image_id)
        .map_err(|e| e.to_string())?;

    output
        .save_png(&png_data, path)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn run_capture_ocr(
    session_id: String,
    rect: LogicalRect,
    state: State<'_, crate::AppState>,
) -> Result<OcrResult, String> {
    let session_id = CaptureSessionId(session_id);
    let png_data = render_capture_png(&session_id, &rect, &[], &state)?;
    let request = OcrRequest {
        image_data: png_data,
        language: None,
    };

    state
        .ocr_coordinator
        .recognize(&request)
        .await
        .map_err(|e| e.to_string())
}

pub async fn open_capture_window_from_shortcut(app: AppHandle, mode: &'static str) {
    let result = async {
        let state = app.state::<crate::AppState>();
        let session = state.capture_session_service.create_session().await?;
        let bounds = capture_window_bounds(&session.monitors).ok_or_else(|| {
            crate::AppError::System("Cannot open capture window without monitor bounds".to_string())
        })?;
        open_capture_window_for_session(&app, mode, &session.id.0, &bounds)
            .map_err(crate::AppError::from)
    }
    .await;

    if let Err(err) = result {
        log::error!("Failed to open capture window: {}", err);
    }
}

pub fn open_capture_window_for_session(
    app: &AppHandle,
    mode: &str,
    session_id: &str,
    bounds: &LogicalRect,
) -> Result<(), String> {
    let mode = normalized_capture_mode(mode);

    if let Some(window) = app.get_webview_window(CAPTURE_WINDOW_LABEL) {
        window.set_fullscreen(false).map_err(|e| e.to_string())?;
        window
            .set_position(LogicalPosition::new(bounds.x, bounds.y))
            .map_err(|e| e.to_string())?;
        window
            .set_size(LogicalSize::new(bounds.width, bounds.height))
            .map_err(|e| e.to_string())?;
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        window
            .emit(
                "hotkey-triggered",
                serde_json::json!({
                    "mode": mode,
                    "sessionId": session_id,
                }),
            )
            .map_err(|e| e.to_string())?;
        return Ok(());
    }

    WebviewWindowBuilder::new(
        app,
        CAPTURE_WINDOW_LABEL,
        WebviewUrl::App(capture_window_url_with_session(mode, session_id)),
    )
    .title("SnapLingo Capture")
    .position(bounds.x, bounds.y)
    .inner_size(bounds.width, bounds.height)
    .decorations(false)
    .always_on_top(true)
    .visible_on_all_workspaces(true)
    .skip_taskbar(true)
    .focused(true)
    .shadow(false)
    .build()
    .map_err(|e| e.to_string())?;

    Ok(())
}

fn capture_window_bounds(monitors: &[MonitorSnapshotView]) -> Option<LogicalRect> {
    if monitors.is_empty() {
        return None;
    }

    let left = monitors
        .iter()
        .map(|monitor| monitor.logical_bounds.x)
        .fold(f64::INFINITY, f64::min);
    let top = monitors
        .iter()
        .map(|monitor| monitor.logical_bounds.y)
        .fold(f64::INFINITY, f64::min);
    let right = monitors
        .iter()
        .map(|monitor| monitor.logical_bounds.x + monitor.logical_bounds.width)
        .fold(f64::NEG_INFINITY, f64::max);
    let bottom = monitors
        .iter()
        .map(|monitor| monitor.logical_bounds.y + monitor.logical_bounds.height)
        .fold(f64::NEG_INFINITY, f64::max);

    Some(LogicalRect {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    })
}

fn pin_capture_png(
    app: &AppHandle,
    state: &crate::AppState,
    png_data: Vec<u8>,
) -> Result<(), String> {
    let image_id = state
        .pinned_image_service
        .pin_png(png_data)
        .map_err(|e| e.to_string())?;
    let image = state
        .pinned_image_service
        .get_pinned_image(&image_id)
        .map_err(|e| e.to_string())?;

    open_pinned_image_window(app, &image)
}

fn open_pinned_image_window(app: &AppHandle, image: &PinnedImageView) -> Result<(), String> {
    let label = pinned_window_label(&image.id);
    let (width, height) = pinned_window_size(image.width, image.height);

    WebviewWindowBuilder::new(app, &label, WebviewUrl::App(pinned_window_url(&image.id)))
        .title("SnapLingo Pin")
        .inner_size(width, height)
        .min_inner_size(80.0, 60.0)
        .decorations(false)
        .always_on_top(true)
        .visible_on_all_workspaces(true)
        .skip_taskbar(true)
        .focused(true)
        .shadow(true)
        .build()
        .map_err(|e| e.to_string())?;

    Ok(())
}

fn render_capture_png(
    session_id: &CaptureSessionId,
    rect: &LogicalRect,
    annotations: &[AnnotationCommand],
    state: &crate::AppState,
) -> Result<Vec<u8>, String> {
    let session = state
        .capture_session_service
        .get_session(session_id)
        .map_err(|e| e.to_string())?;

    let plan = capture_image_composition_plan(rect, &session.snapshots)?;
    let placements = plan
        .placements
        .iter()
        .map(|placement| PngPlacement {
            png_data: session.snapshots[placement.snapshot_index]
                .png_data
                .as_slice(),
            source_rect: placement.source_rect.clone(),
            destination_rect: placement.destination_rect.clone(),
        })
        .collect::<Vec<_>>();

    let image_annotations = image_annotations_from_commands(annotations, rect, plan.width)?;

    state
        .image_composition_service
        .compose_png_with_annotations(plan.width, plan.height, &placements, &image_annotations)
        .map_err(|e| e.to_string())
}

fn image_annotations_from_commands(
    annotations: &[AnnotationCommand],
    selection_rect: &LogicalRect,
    output_width: u32,
) -> Result<Vec<ImageAnnotation>, String> {
    if annotations.is_empty() {
        return Ok(Vec::new());
    }

    let output_scale = output_width as f64 / selection_rect.width;
    let annotation_origin = LogicalRect {
        x: 0.0,
        y: 0.0,
        width: selection_rect.width,
        height: selection_rect.height,
    };

    annotations
        .iter()
        .map(|annotation| match annotation {
            AnnotationCommand::Rectangle {
                rect,
                color,
                stroke_width,
            } => Ok(ImageAnnotation::Rectangle {
                rect: scaled_logical_rect_relative_to(rect, &annotation_origin, output_scale)?,
                color: *color,
                stroke_width: ((*stroke_width).max(1) as f64 * output_scale).ceil() as u32,
            }),
            AnnotationCommand::Ellipse {
                rect,
                color,
                stroke_width,
            } => Ok(ImageAnnotation::Ellipse {
                rect: scaled_logical_rect_relative_to(rect, &annotation_origin, output_scale)?,
                color: *color,
                stroke_width: ((*stroke_width).max(1) as f64 * output_scale).ceil() as u32,
            }),
            AnnotationCommand::Arrow {
                start,
                end,
                color,
                stroke_width,
            } => Ok(ImageAnnotation::Arrow {
                start: scaled_logical_point_relative_to(start, &annotation_origin, output_scale)?,
                end: scaled_logical_point_relative_to(end, &annotation_origin, output_scale)?,
                color: *color,
                stroke_width: ((*stroke_width).max(1) as f64 * output_scale).ceil() as u32,
            }),
            AnnotationCommand::Line {
                start,
                end,
                color,
                stroke_width,
            } => Ok(ImageAnnotation::Line {
                start: scaled_logical_point_relative_to(start, &annotation_origin, output_scale)?,
                end: scaled_logical_point_relative_to(end, &annotation_origin, output_scale)?,
                color: *color,
                stroke_width: ((*stroke_width).max(1) as f64 * output_scale).ceil() as u32,
            }),
            AnnotationCommand::Freehand {
                points,
                color,
                stroke_width,
            } => Ok(ImageAnnotation::Freehand {
                points: points
                    .iter()
                    .map(|point| {
                        scaled_logical_point_relative_to(point, &annotation_origin, output_scale)
                    })
                    .collect::<Result<Vec<_>, String>>()?,
                color: *color,
                stroke_width: ((*stroke_width).max(1) as f64 * output_scale).ceil() as u32,
            }),
            AnnotationCommand::Highlight {
                points,
                color,
                stroke_width,
            } => Ok(ImageAnnotation::Highlight {
                points: points
                    .iter()
                    .map(|point| {
                        scaled_logical_point_relative_to(point, &annotation_origin, output_scale)
                    })
                    .collect::<Result<Vec<_>, String>>()?,
                color: *color,
                stroke_width: ((*stroke_width).max(1) as f64 * output_scale).ceil() as u32,
            }),
            AnnotationCommand::Mosaic { rect, block_size } => Ok(ImageAnnotation::Mosaic {
                rect: scaled_logical_rect_relative_to(rect, &annotation_origin, output_scale)?,
                block_size: ((*block_size).max(1) as f64 * output_scale).ceil() as u32,
            }),
            AnnotationCommand::Blur { rect, radius } => Ok(ImageAnnotation::Blur {
                rect: scaled_logical_rect_relative_to(rect, &annotation_origin, output_scale)?,
                radius: ((*radius).max(1) as f64 * output_scale).ceil() as u32,
            }),
            AnnotationCommand::Text {
                position,
                text,
                color,
                font_size,
            } => Ok(ImageAnnotation::Text {
                position: scaled_logical_point_relative_to(
                    position,
                    &annotation_origin,
                    output_scale,
                )?,
                text: text.clone(),
                color: *color,
                font_size: ((*font_size).max(1) as f64 * output_scale).ceil() as u32,
            }),
        })
        .collect()
}

fn snapshot_to_view(snapshot: &MonitorSnapshot) -> MonitorSnapshotView {
    MonitorSnapshotView {
        id: snapshot.id.clone(),
        logical_bounds: snapshot.logical_bounds.clone(),
        physical_bounds: snapshot.physical_bounds.clone(),
        scale_factor: snapshot.scale_factor,
        image_base64: base64::engine::general_purpose::STANDARD.encode(&snapshot.png_data),
    }
}

fn capture_window_url(mode: &str) -> PathBuf {
    PathBuf::from(format!(
        "index.html?window=capture&mode={}",
        normalized_capture_mode(mode)
    ))
}

fn capture_window_url_with_session(mode: &str, session_id: &str) -> PathBuf {
    PathBuf::from(format!(
        "{}&sessionId={}",
        capture_window_url(mode).to_string_lossy(),
        session_id
    ))
}

fn pinned_window_url(image_id: &str) -> PathBuf {
    PathBuf::from(format!("index.html?window=pin&imageId={}", image_id))
}

fn pinned_window_label(image_id: &str) -> String {
    format!("pin-{}", image_id)
}

fn is_pinned_window_label(label: &str) -> bool {
    label.starts_with("pin-")
}

fn next_pinned_windows_visible_state(current_visibility: &[bool]) -> Option<bool> {
    if current_visibility.is_empty() {
        return None;
    }

    Some(current_visibility.iter().any(|is_visible| !*is_visible))
}

fn pinned_group_window_visibility_changes(
    group_switch: &PinnedImageGroupSwitch,
) -> Vec<PinnedWindowVisibilityChange> {
    group_switch
        .hide_image_ids
        .iter()
        .map(|image_id| PinnedWindowVisibilityChange {
            label: pinned_window_label(image_id),
            visible: false,
        })
        .chain(
            group_switch
                .show_image_ids
                .iter()
                .map(|image_id| PinnedWindowVisibilityChange {
                    label: pinned_window_label(image_id),
                    visible: true,
                }),
        )
        .collect()
}

fn moved_pinned_image_window_visibility_change(image_id: &str) -> PinnedWindowVisibilityChange {
    PinnedWindowVisibilityChange {
        label: pinned_window_label(image_id),
        visible: false,
    }
}

fn pinned_window_size(width: u32, height: u32) -> (f64, f64) {
    let width = width.max(1) as f64;
    let height = height.max(1) as f64;
    let scale = (PIN_WINDOW_MAX_WIDTH / width)
        .min(PIN_WINDOW_MAX_HEIGHT / height)
        .min(1.0);

    ((width * scale).max(80.0), (height * scale).max(60.0))
}

fn capture_save_path(base_dir: &Path, timestamp: &str) -> PathBuf {
    base_dir.join(format!("SnapLingo-{}.png", timestamp))
}

fn normalized_capture_mode(mode: &str) -> &'static str {
    match mode {
        "screenshot" | "screenshot-ocr" | "screenshot-translate" => match mode {
            "screenshot-ocr" => "screenshot-ocr",
            "screenshot-translate" => "screenshot-translate",
            _ => "screenshot",
        },
        _ => "screenshot",
    }
}

fn capture_image_composition_plan(
    rect: &LogicalRect,
    snapshots: &[MonitorSnapshot],
) -> Result<CaptureImageCompositionPlan, String> {
    if rect.width <= 0.0 || rect.height <= 0.0 {
        return Err("Selection has no area".to_string());
    }

    let intersections = snapshots
        .iter()
        .enumerate()
        .filter_map(|(snapshot_index, snapshot)| {
            logical_rect_intersection(rect, &snapshot.logical_bounds)
                .map(|intersection| (snapshot_index, snapshot, intersection))
        })
        .collect::<Vec<_>>();

    if intersections.is_empty() {
        return Err("Selection does not intersect any captured monitor".to_string());
    }

    let output_scale = intersections
        .iter()
        .map(|(_, snapshot, _)| snapshot.scale_factor)
        .fold(1.0_f64, f64::max);
    let output_width = scaled_extent(rect.width, output_scale)?;
    let output_height = scaled_extent(rect.height, output_scale)?;
    let placements = intersections
        .into_iter()
        .map(|(snapshot_index, snapshot, intersection)| {
            let source_rect = scaled_logical_rect_relative_to(
                &intersection,
                &snapshot.logical_bounds,
                snapshot.scale_factor,
            )?;
            let destination_rect =
                scaled_logical_rect_relative_to(&intersection, rect, output_scale)?;

            Ok(CaptureImagePlacement {
                snapshot_index,
                source_rect,
                destination_rect,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    Ok(CaptureImageCompositionPlan {
        width: output_width,
        height: output_height,
        placements,
    })
}

fn logical_rect_intersection(a: &LogicalRect, b: &LogicalRect) -> Option<LogicalRect> {
    let left = a.x.max(b.x);
    let top = a.y.max(b.y);
    let right = (a.x + a.width).min(b.x + b.width);
    let bottom = (a.y + a.height).min(b.y + b.height);

    if right <= left || bottom <= top {
        return None;
    }

    Some(LogicalRect {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    })
}

fn scaled_logical_rect_relative_to(
    rect: &LogicalRect,
    origin: &LogicalRect,
    scale: f64,
) -> Result<PhysicalRect, String> {
    let left = ((rect.x - origin.x) * scale).floor();
    let top = ((rect.y - origin.y) * scale).floor();
    let right = ((rect.x + rect.width - origin.x) * scale).ceil();
    let bottom = ((rect.y + rect.height - origin.y) * scale).ceil();

    if left < 0.0 || top < 0.0 || right <= left || bottom <= top {
        return Err("Selection has invalid scaled capture bounds".to_string());
    }

    Ok(PhysicalRect {
        x: left as i32,
        y: top as i32,
        width: (right - left) as u32,
        height: (bottom - top) as u32,
    })
}

fn scaled_logical_point_relative_to(
    point: &LogicalPoint,
    origin: &LogicalRect,
    scale: f64,
) -> Result<PhysicalPoint, String> {
    let x = ((point.x - origin.x) * scale).round();
    let y = ((point.y - origin.y) * scale).round();

    if x < 0.0 || y < 0.0 {
        return Err("Annotation has invalid scaled capture point".to_string());
    }

    Ok(PhysicalPoint {
        x: x as i32,
        y: y as i32,
    })
}

fn scaled_extent(value: f64, scale: f64) -> Result<u32, String> {
    let scaled = (value * scale).ceil();
    if scaled <= 0.0 {
        return Err("Selection has invalid scaled capture size".to_string());
    }

    Ok(scaled as u32)
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use image::ImageEncoder;

    use crate::application::services::image_composition_service::ImageAnnotation;
    use crate::application::services::{
        CaptureOutputService, PinnedImageGroupSwitch, PinnedImageService,
    };
    use crate::domain::capture::{
        AnnotationCommand, LogicalPoint, LogicalRect, MonitorSnapshotView, PhysicalPoint,
        PhysicalRect,
    };
    use crate::infrastructure::system::screenshot::MonitorSnapshot;

    #[test]
    fn capture_window_url_encodes_supported_mode() {
        assert_eq!(
            super::capture_window_url("screenshot-ocr").to_string_lossy(),
            "index.html?window=capture&mode=screenshot-ocr"
        );
    }

    #[test]
    fn capture_window_url_falls_back_to_screenshot_for_unknown_mode() {
        assert_eq!(
            super::capture_window_url("unknown").to_string_lossy(),
            "index.html?window=capture&mode=screenshot"
        );
    }

    #[test]
    fn capture_window_bounds_union_monitor_logical_bounds() {
        let monitors = vec![
            MonitorSnapshotView {
                id: "left".to_string(),
                logical_bounds: LogicalRect {
                    x: -1280.0,
                    y: 0.0,
                    width: 1280.0,
                    height: 720.0,
                },
                physical_bounds: PhysicalRect {
                    x: -2560,
                    y: 0,
                    width: 2560,
                    height: 1440,
                },
                scale_factor: 2.0,
                image_base64: String::new(),
            },
            MonitorSnapshotView {
                id: "primary".to_string(),
                logical_bounds: LogicalRect {
                    x: 0.0,
                    y: 0.0,
                    width: 1440.0,
                    height: 900.0,
                },
                physical_bounds: PhysicalRect {
                    x: 0,
                    y: 0,
                    width: 2880,
                    height: 1800,
                },
                scale_factor: 2.0,
                image_base64: String::new(),
            },
            MonitorSnapshotView {
                id: "top".to_string(),
                logical_bounds: LogicalRect {
                    x: 0.0,
                    y: -600.0,
                    width: 960.0,
                    height: 600.0,
                },
                physical_bounds: PhysicalRect {
                    x: 0,
                    y: -1200,
                    width: 1920,
                    height: 1200,
                },
                scale_factor: 2.0,
                image_base64: String::new(),
            },
        ];

        assert_eq!(
            super::capture_window_bounds(&monitors),
            Some(LogicalRect {
                x: -1280.0,
                y: -600.0,
                width: 2720.0,
                height: 1500.0,
            })
        );
    }

    #[test]
    fn capture_image_placements_split_selection_across_monitors() {
        let snapshots = vec![
            MonitorSnapshot {
                id: "left".to_string(),
                logical_bounds: LogicalRect {
                    x: -4.0,
                    y: 0.0,
                    width: 4.0,
                    height: 2.0,
                },
                physical_bounds: PhysicalRect {
                    x: -4,
                    y: 0,
                    width: 4,
                    height: 2,
                },
                scale_factor: 1.0,
                png_data: Vec::new(),
            },
            MonitorSnapshot {
                id: "primary".to_string(),
                logical_bounds: LogicalRect {
                    x: 0.0,
                    y: 0.0,
                    width: 4.0,
                    height: 2.0,
                },
                physical_bounds: PhysicalRect {
                    x: 0,
                    y: 0,
                    width: 4,
                    height: 2,
                },
                scale_factor: 1.0,
                png_data: Vec::new(),
            },
        ];

        let plan = super::capture_image_composition_plan(
            &LogicalRect {
                x: -2.0,
                y: 0.0,
                width: 4.0,
                height: 2.0,
            },
            &snapshots,
        )
        .unwrap();

        assert_eq!(plan.width, 4);
        assert_eq!(plan.height, 2);
        assert_eq!(plan.placements.len(), 2);
        assert_eq!(plan.placements[0].snapshot_index, 0);
        assert_eq!(
            plan.placements[0].source_rect,
            PhysicalRect {
                x: 2,
                y: 0,
                width: 2,
                height: 2,
            }
        );
        assert_eq!(
            plan.placements[0].destination_rect,
            PhysicalRect {
                x: 0,
                y: 0,
                width: 2,
                height: 2,
            }
        );
        assert_eq!(plan.placements[1].snapshot_index, 1);
        assert_eq!(
            plan.placements[1].source_rect,
            PhysicalRect {
                x: 0,
                y: 0,
                width: 2,
                height: 2,
            }
        );
        assert_eq!(
            plan.placements[1].destination_rect,
            PhysicalRect {
                x: 2,
                y: 0,
                width: 2,
                height: 2,
            }
        );
    }

    #[test]
    fn scales_selection_local_annotations_to_output_pixels() {
        let annotations = super::image_annotations_from_commands(
            &[
                AnnotationCommand::Rectangle {
                    rect: LogicalRect {
                        x: 1.0,
                        y: 2.0,
                        width: 3.0,
                        height: 4.0,
                    },
                    color: [255, 77, 79, 255],
                    stroke_width: 2,
                },
                AnnotationCommand::Arrow {
                    start: LogicalPoint { x: 2.0, y: 3.0 },
                    end: LogicalPoint { x: 5.0, y: 7.0 },
                    color: [255, 77, 79, 255],
                    stroke_width: 2,
                },
                AnnotationCommand::Line {
                    start: LogicalPoint { x: 3.0, y: 4.0 },
                    end: LogicalPoint { x: 6.0, y: 8.0 },
                    color: [250, 219, 20, 255],
                    stroke_width: 3,
                },
                AnnotationCommand::Freehand {
                    points: vec![
                        LogicalPoint { x: 1.0, y: 1.5 },
                        LogicalPoint { x: 3.0, y: 4.0 },
                    ],
                    color: [24, 144, 255, 255],
                    stroke_width: 1,
                },
                AnnotationCommand::Highlight {
                    points: vec![
                        LogicalPoint { x: 2.0, y: 2.5 },
                        LogicalPoint { x: 4.0, y: 5.0 },
                    ],
                    color: [250, 219, 20, 96],
                    stroke_width: 3,
                },
                AnnotationCommand::Mosaic {
                    rect: LogicalRect {
                        x: 2.0,
                        y: 3.0,
                        width: 4.0,
                        height: 5.0,
                    },
                    block_size: 3,
                },
                AnnotationCommand::Blur {
                    rect: LogicalRect {
                        x: 1.0,
                        y: 2.0,
                        width: 3.0,
                        height: 4.0,
                    },
                    radius: 3,
                },
                AnnotationCommand::Ellipse {
                    rect: LogicalRect {
                        x: 0.5,
                        y: 1.0,
                        width: 2.5,
                        height: 3.5,
                    },
                    color: [40, 167, 69, 255],
                    stroke_width: 2,
                },
                AnnotationCommand::Text {
                    position: LogicalPoint { x: 3.5, y: 4.5 },
                    text: "Snap".to_string(),
                    color: [255, 255, 255, 255],
                    font_size: 12,
                },
            ],
            &LogicalRect {
                x: 100.0,
                y: 200.0,
                width: 10.0,
                height: 10.0,
            },
            20,
        )
        .unwrap();

        assert_eq!(annotations.len(), 9);
        assert_eq!(
            annotations[0],
            ImageAnnotation::Rectangle {
                rect: PhysicalRect {
                    x: 2,
                    y: 4,
                    width: 6,
                    height: 8,
                },
                color: [255, 77, 79, 255],
                stroke_width: 4,
            }
        );
        assert_eq!(
            annotations[1],
            ImageAnnotation::Arrow {
                start: PhysicalPoint { x: 4, y: 6 },
                end: PhysicalPoint { x: 10, y: 14 },
                color: [255, 77, 79, 255],
                stroke_width: 4,
            }
        );
        assert_eq!(
            annotations[2],
            ImageAnnotation::Line {
                start: PhysicalPoint { x: 6, y: 8 },
                end: PhysicalPoint { x: 12, y: 16 },
                color: [250, 219, 20, 255],
                stroke_width: 6,
            }
        );
        assert_eq!(
            annotations[3],
            ImageAnnotation::Freehand {
                points: vec![PhysicalPoint { x: 2, y: 3 }, PhysicalPoint { x: 6, y: 8 }],
                color: [24, 144, 255, 255],
                stroke_width: 2,
            }
        );
        assert_eq!(
            annotations[4],
            ImageAnnotation::Highlight {
                points: vec![PhysicalPoint { x: 4, y: 5 }, PhysicalPoint { x: 8, y: 10 }],
                color: [250, 219, 20, 96],
                stroke_width: 6,
            }
        );
        assert_eq!(
            annotations[5],
            ImageAnnotation::Mosaic {
                rect: PhysicalRect {
                    x: 4,
                    y: 6,
                    width: 8,
                    height: 10,
                },
                block_size: 6,
            }
        );
        assert_eq!(
            annotations[6],
            ImageAnnotation::Blur {
                rect: PhysicalRect {
                    x: 2,
                    y: 4,
                    width: 6,
                    height: 8,
                },
                radius: 6,
            }
        );
        assert_eq!(
            annotations[7],
            ImageAnnotation::Ellipse {
                rect: PhysicalRect {
                    x: 1,
                    y: 2,
                    width: 5,
                    height: 7,
                },
                color: [40, 167, 69, 255],
                stroke_width: 4,
            }
        );
        assert_eq!(
            annotations[8],
            ImageAnnotation::Text {
                position: PhysicalPoint { x: 7, y: 9 },
                text: "Snap".to_string(),
                color: [255, 255, 255, 255],
                font_size: 24,
            }
        );
    }

    #[test]
    fn pinned_window_url_targets_pin_route() {
        assert_eq!(
            super::pinned_window_url("pin-1").to_string_lossy(),
            "index.html?window=pin&imageId=pin-1"
        );
    }

    #[test]
    fn identifies_pinned_window_labels() {
        assert!(super::is_pinned_window_label("pin-pin-1"));
        assert!(!super::is_pinned_window_label("capture"));
        assert!(!super::is_pinned_window_label("main"));
    }

    #[test]
    fn toggles_pinned_windows_based_on_current_visibility() {
        assert_eq!(
            super::next_pinned_windows_visible_state(&[true, false]),
            Some(true)
        );
        assert_eq!(
            super::next_pinned_windows_visible_state(&[true, true]),
            Some(false)
        );
        assert_eq!(
            super::next_pinned_windows_visible_state(&[false, false]),
            Some(true)
        );
        assert_eq!(super::next_pinned_windows_visible_state(&[]), None);
    }

    #[test]
    fn plans_pinned_group_window_visibility_changes() {
        let group_switch = PinnedImageGroupSwitch {
            previous_group: 0,
            next_group: 1,
            hide_image_ids: vec!["pin-1".to_string()],
            show_image_ids: vec!["pin-2".to_string(), "pin-3".to_string()],
        };

        assert_eq!(
            super::pinned_group_window_visibility_changes(&group_switch),
            vec![
                super::PinnedWindowVisibilityChange {
                    label: "pin-pin-1".to_string(),
                    visible: false,
                },
                super::PinnedWindowVisibilityChange {
                    label: "pin-pin-2".to_string(),
                    visible: true,
                },
                super::PinnedWindowVisibilityChange {
                    label: "pin-pin-3".to_string(),
                    visible: true,
                },
            ]
        );
    }

    #[test]
    fn plans_moved_pinned_image_to_hide_current_window() {
        assert_eq!(
            super::moved_pinned_image_window_visibility_change("pin-1"),
            super::PinnedWindowVisibilityChange {
                label: "pin-pin-1".to_string(),
                visible: false,
            }
        );
    }

    #[test]
    fn pinned_window_size_preserves_aspect_ratio_with_cap() {
        assert_eq!(super::pinned_window_size(300, 200), (300.0, 200.0));
        assert_eq!(super::pinned_window_size(1800, 900), (900.0, 450.0));
    }

    #[test]
    fn capture_save_path_uses_timestamped_png_name() {
        let path = super::capture_save_path(std::path::Path::new("/tmp"), "20260617-023000");

        assert_eq!(path.to_string_lossy(), "/tmp/SnapLingo-20260617-023000.png");
    }

    #[tokio::test]
    async fn save_pinned_png_by_id_writes_original_png_to_path() {
        let pinned_images = PinnedImageService::new();
        let output = CaptureOutputService::new();
        let png = make_test_png(2, 3);
        let image_id = pinned_images.pin_png(png.clone()).unwrap();
        let path = temp_png_path();

        super::save_pinned_png_by_id(&pinned_images, &output, &image_id, &path)
            .await
            .unwrap();

        assert_eq!(std::fs::read(&path).unwrap(), png);

        let _ = std::fs::remove_file(path);
    }

    fn make_test_png(width: u32, height: u32) -> Vec<u8> {
        let pixels = vec![255; (width * height * 4) as usize];
        let mut png = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut png);
        encoder
            .write_image(&pixels, width, height, image::ExtendedColorType::Rgba8)
            .unwrap();
        png
    }

    fn temp_png_path() -> std::path::PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir()
            .join("snaplingo-pinned-output-tests")
            .join(format!("pin-{}.png", suffix))
    }
}

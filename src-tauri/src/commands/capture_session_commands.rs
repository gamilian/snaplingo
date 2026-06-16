use std::path::PathBuf;

use base64::Engine;
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};

use crate::domain::capture::{
    CaptureOutputAction, CaptureSessionId, CaptureSessionView, LogicalRect, MonitorSnapshotView,
    PhysicalRect, PinnedImageView,
};
use crate::domain::ocr::{OcrRequest, OcrResult};
use crate::infrastructure::system::screenshot::MonitorSnapshot;

const CAPTURE_WINDOW_LABEL: &str = "capture";
const PIN_WINDOW_MAX_WIDTH: f64 = 900.0;
const PIN_WINDOW_MAX_HEIGHT: f64 = 700.0;

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
    open_capture_window_for_session(app, mode, &session.id.0)
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
    state: State<'_, crate::AppState>,
) -> Result<String, String> {
    let session_id = CaptureSessionId(session_id);
    let png_data = render_capture_png(&session_id, &rect, &state)?;

    Ok(base64::engine::general_purpose::STANDARD.encode(png_data))
}

#[tauri::command]
pub async fn output_capture(
    session_id: String,
    rect: LogicalRect,
    action: CaptureOutputAction,
    app: AppHandle,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    let session_id = CaptureSessionId(session_id);
    let png_data = render_capture_png(&session_id, &rect, &state)?;

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
pub async fn run_capture_ocr(
    session_id: String,
    rect: LogicalRect,
    state: State<'_, crate::AppState>,
) -> Result<OcrResult, String> {
    let session_id = CaptureSessionId(session_id);
    let png_data = render_capture_png(&session_id, &rect, &state)?;
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
        open_capture_window_for_session(&app, mode, &session.id.0).map_err(crate::AppError::from)
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
) -> Result<(), String> {
    let mode = normalized_capture_mode(mode);

    if let Some(window) = app.get_webview_window(CAPTURE_WINDOW_LABEL) {
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
    .fullscreen(true)
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

    WebviewWindowBuilder::new(
        app,
        &label,
        WebviewUrl::App(pinned_window_url(&image.id)),
    )
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
    state: &crate::AppState,
) -> Result<Vec<u8>, String> {
    let physical_rect = state
        .capture_session_service
        .logical_rect_to_physical(session_id, rect)
        .map_err(|e| e.to_string())?;
    let session = state
        .capture_session_service
        .get_session(session_id)
        .map_err(|e| e.to_string())?;
    let snapshot = session
        .snapshots
        .iter()
        .find(|snapshot| physical_rects_intersect(&physical_rect, &snapshot.physical_bounds))
        .ok_or_else(|| "Selection does not intersect any captured monitor".to_string())?;
    let crop_rect = snapshot_relative_crop_rect(&physical_rect, &snapshot.physical_bounds);
    state
        .image_composition_service
        .crop_png(&snapshot.png_data, &crop_rect)
        .map_err(|e| e.to_string())
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

fn pinned_window_size(width: u32, height: u32) -> (f64, f64) {
    let width = width.max(1) as f64;
    let height = height.max(1) as f64;
    let scale = (PIN_WINDOW_MAX_WIDTH / width)
        .min(PIN_WINDOW_MAX_HEIGHT / height)
        .min(1.0);

    ((width * scale).max(80.0), (height * scale).max(60.0))
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

fn snapshot_relative_crop_rect(
    selected: &PhysicalRect,
    snapshot_bounds: &PhysicalRect,
) -> PhysicalRect {
    PhysicalRect {
        x: selected.x - snapshot_bounds.x,
        y: selected.y - snapshot_bounds.y,
        width: selected.width,
        height: selected.height,
    }
}

fn physical_rects_intersect(a: &PhysicalRect, b: &PhysicalRect) -> bool {
    let a_right = a.x + a.width as i32;
    let a_bottom = a.y + a.height as i32;
    let b_right = b.x + b.width as i32;
    let b_bottom = b.y + b.height as i32;

    a.x < b_right && a_right > b.x && a.y < b_bottom && a_bottom > b.y
}

#[cfg(test)]
mod tests {
    use crate::domain::capture::PhysicalRect;

    #[test]
    fn converts_global_physical_rect_to_snapshot_local_crop_rect() {
        let snapshot_bounds = PhysicalRect {
            x: 100,
            y: 200,
            width: 300,
            height: 400,
        };
        let selected = PhysicalRect {
            x: 110,
            y: 220,
            width: 30,
            height: 40,
        };

        let crop_rect = super::snapshot_relative_crop_rect(&selected, &snapshot_bounds);

        assert_eq!(
            crop_rect,
            PhysicalRect {
                x: 10,
                y: 20,
                width: 30,
                height: 40,
            }
        );
    }

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
    fn pinned_window_url_targets_pin_route() {
        assert_eq!(
            super::pinned_window_url("pin-1").to_string_lossy(),
            "index.html?window=pin&imageId=pin-1"
        );
    }

    #[test]
    fn pinned_window_size_preserves_aspect_ratio_with_cap() {
        assert_eq!(super::pinned_window_size(300, 200), (300.0, 200.0));
        assert_eq!(super::pinned_window_size(1800, 900), (900.0, 450.0));
    }
}

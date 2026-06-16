use base64::Engine;
use tauri::State;

use crate::domain::capture::{CaptureSessionId, CaptureSessionView, LogicalRect, PhysicalRect};

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
    let physical_rect = state
        .capture_session_service
        .logical_rect_to_physical(&session_id, &rect)
        .map_err(|e| e.to_string())?;
    let session = state
        .capture_session_service
        .get_session(&session_id)
        .map_err(|e| e.to_string())?;
    let snapshot = session
        .snapshots
        .iter()
        .find(|snapshot| physical_rects_intersect(&physical_rect, &snapshot.physical_bounds))
        .ok_or_else(|| "Selection does not intersect any captured monitor".to_string())?;
    let crop_rect = snapshot_relative_crop_rect(&physical_rect, &snapshot.physical_bounds);
    let png_data = state
        .image_composition_service
        .crop_png(&snapshot.png_data, &crop_rect)
        .map_err(|e| e.to_string())?;

    Ok(base64::engine::general_purpose::STANDARD.encode(png_data))
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
}

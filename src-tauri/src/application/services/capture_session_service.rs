use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;

use crate::domain::capture::{
    CaptureSessionId, CaptureSessionView, LogicalRect, MonitorSnapshotView, PhysicalRect,
};
use crate::error::{AppError, Result};
use crate::infrastructure::system::screenshot::{MonitorSnapshot, ScreenshotBackend};

static NEXT_SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone)]
pub struct CaptureSession {
    pub id: CaptureSessionId,
    pub snapshots: Vec<MonitorSnapshot>,
    pub created_at: SystemTime,
}

/// Owns frozen screenshot sessions.
pub struct CaptureSessionService {
    screenshot_backend: Arc<dyn ScreenshotBackend>,
    sessions: Arc<Mutex<HashMap<CaptureSessionId, CaptureSession>>>,
}

impl CaptureSessionService {
    pub fn new(screenshot_backend: Arc<dyn ScreenshotBackend>) -> Self {
        Self {
            screenshot_backend,
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn create_session(&self) -> Result<CaptureSessionView> {
        let snapshots = self.screenshot_backend.capture_monitor_snapshots().await?;
        if snapshots.is_empty() {
            return Err(AppError::System(
                "Cannot create capture session without monitor snapshots".to_string(),
            ));
        }

        let id = CaptureSessionId(generate_session_id());
        let view = CaptureSessionView {
            id: id.clone(),
            monitors: snapshots.iter().map(snapshot_to_view).collect(),
        };
        let session = CaptureSession {
            id: id.clone(),
            snapshots,
            created_at: SystemTime::now(),
        };

        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| AppError::System("Capture session lock poisoned".to_string()))?;
        sessions.insert(id, session);

        Ok(view)
    }

    pub fn cancel_session(&self, id: &CaptureSessionId) -> Result<()> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| AppError::System("Capture session lock poisoned".to_string()))?;
        sessions.remove(id);
        Ok(())
    }

    pub fn get_session(&self, id: &CaptureSessionId) -> Result<CaptureSession> {
        let sessions = self
            .sessions
            .lock()
            .map_err(|_| AppError::System("Capture session lock poisoned".to_string()))?;
        sessions
            .get(id)
            .cloned()
            .ok_or_else(|| AppError::System(format!("Capture session not found: {}", id.0)))
    }

    pub fn has_session(&self, id: &CaptureSessionId) -> bool {
        self.sessions
            .lock()
            .map(|sessions| sessions.contains_key(id))
            .unwrap_or(false)
    }

    pub fn logical_rect_to_physical(
        &self,
        id: &CaptureSessionId,
        rect: &LogicalRect,
    ) -> Result<PhysicalRect> {
        let session = self.get_session(id)?;
        let snapshot = session
            .snapshots
            .iter()
            .find(|snapshot| logical_rects_intersect(rect, &snapshot.logical_bounds))
            .ok_or_else(|| {
                AppError::System("Selection does not intersect any captured monitor".to_string())
            })?;

        logical_rect_to_snapshot_physical(rect, snapshot)
    }
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

fn generate_session_id() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let counter = NEXT_SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("capture-{}-{}", timestamp, counter)
}

fn logical_rect_to_snapshot_physical(
    rect: &LogicalRect,
    snapshot: &MonitorSnapshot,
) -> Result<PhysicalRect> {
    let left = rect.x.max(snapshot.logical_bounds.x);
    let top = rect.y.max(snapshot.logical_bounds.y);
    let right =
        (rect.x + rect.width).min(snapshot.logical_bounds.x + snapshot.logical_bounds.width);
    let bottom =
        (rect.y + rect.height).min(snapshot.logical_bounds.y + snapshot.logical_bounds.height);

    if right <= left || bottom <= top {
        return Err(AppError::System(
            "Selection has no area inside captured monitor".to_string(),
        ));
    }

    let scale = snapshot.scale_factor;
    let relative_left = left - snapshot.logical_bounds.x;
    let relative_top = top - snapshot.logical_bounds.y;
    let relative_right = right - snapshot.logical_bounds.x;
    let relative_bottom = bottom - snapshot.logical_bounds.y;

    let physical_left = snapshot.physical_bounds.x + (relative_left * scale).floor() as i32;
    let physical_top = snapshot.physical_bounds.y + (relative_top * scale).floor() as i32;
    let physical_right = snapshot.physical_bounds.x + (relative_right * scale).ceil() as i32;
    let physical_bottom = snapshot.physical_bounds.y + (relative_bottom * scale).ceil() as i32;

    Ok(PhysicalRect {
        x: physical_left,
        y: physical_top,
        width: (physical_right - physical_left) as u32,
        height: (physical_bottom - physical_top) as u32,
    })
}

fn logical_rects_intersect(a: &LogicalRect, b: &LogicalRect) -> bool {
    let a_right = a.x + a.width;
    let a_bottom = a.y + a.height;
    let b_right = b.x + b.width;
    let b_bottom = b.y + b.height;

    a.x < b_right && a_right > b.x && a.y < b_bottom && a_bottom > b.y
}

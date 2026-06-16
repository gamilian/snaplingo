use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;

use crate::domain::capture::{CaptureSessionId, CaptureSessionView, MonitorSnapshotView};
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

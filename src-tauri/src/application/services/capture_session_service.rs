use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use base64::Engine;

use crate::domain::capture::{
    CaptureCandidateView, CaptureSessionId, CaptureSessionView, CapturedCursorView, LogicalPoint,
    LogicalRect, MonitorSnapshotView, PhysicalRect,
};
use crate::error::{AppError, Result};
use crate::infrastructure::system::screenshot::{
    monitor_snapshot_from_layout, CapturedCursor, MonitorSnapshot, ScreenRegion, ScreenshotBackend,
    WindowCandidate,
};

static NEXT_SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone)]
pub struct CaptureSession {
    pub id: CaptureSessionId,
    pub layout_snapshots: Vec<MonitorSnapshot>,
    pub snapshots: Vec<MonitorSnapshot>,
    pub candidates: Vec<CaptureCandidateView>,
    pub captured_cursor: Option<CapturedCursor>,
    pub hidden_window_labels: Vec<String>,
    pub created_at: SystemTime,
}

pub struct CaptureSessionSnapshotCache {
    snapshots: Vec<MonitorSnapshot>,
    captured_cursor: Option<CapturedCursor>,
}

/// Owns frozen screenshot sessions.
pub struct CaptureSessionService {
    screenshot_backend: Arc<dyn ScreenshotBackend>,
    sessions: Arc<Mutex<HashMap<CaptureSessionId, CaptureSession>>>,
    hydrating_sessions: Arc<Mutex<HashSet<CaptureSessionId>>>,
    hydration_notify: Arc<tokio::sync::Notify>,
}

impl CaptureSessionService {
    pub fn new(screenshot_backend: Arc<dyn ScreenshotBackend>) -> Self {
        Self {
            screenshot_backend,
            sessions: Arc::new(Mutex::new(HashMap::new())),
            hydrating_sessions: Arc::new(Mutex::new(HashSet::new())),
            hydration_notify: Arc::new(tokio::sync::Notify::new()),
        }
    }

    pub async fn create_session(&self) -> Result<CaptureSessionView> {
        self.create_session_with_hidden_window_labels(Vec::new())
            .await
    }

    pub async fn create_session_with_hidden_window_labels(
        &self,
        hidden_window_labels: Vec<String>,
    ) -> Result<CaptureSessionView> {
        let total_start = Instant::now();

        let snapshots_start = Instant::now();
        let snapshots = self.screenshot_backend.capture_monitor_snapshots().await?;
        let snapshots_ms = elapsed_ms(snapshots_start);
        if snapshots.is_empty() {
            return Err(AppError::System(
                "Cannot create capture session without monitor snapshots".to_string(),
            ));
        }

        let window_candidates_start = Instant::now();
        let window_candidates = self
            .screenshot_backend
            .capture_window_candidates(&snapshots)
            .await
            .map_err(|err| {
                log::warn!("Failed to capture window candidates: {}", err);
                err
            })
            .unwrap_or_default();
        let window_candidates_ms = elapsed_ms(window_candidates_start);

        let captured_cursor_start = Instant::now();
        let captured_cursor = self
            .screenshot_backend
            .capture_cursor(&snapshots)
            .await
            .map_err(|err| {
                log::warn!("Failed to capture cursor: {}", err);
                err
            })
            .unwrap_or_default();
        let captured_cursor_ms = elapsed_ms(captured_cursor_start);
        let candidates = window_candidates
            .iter()
            .enumerate()
            .map(|(index, candidate)| window_candidate_to_view(candidate, index))
            .collect::<Vec<_>>();

        let id = CaptureSessionId(generate_session_id());
        let session = CaptureSession {
            id: id.clone(),
            layout_snapshots: snapshots
                .iter()
                .cloned()
                .map(snapshot_without_pixels)
                .collect(),
            snapshots,
            candidates,
            captured_cursor,
            hidden_window_labels,
            created_at: SystemTime::now(),
        };

        let view_start = Instant::now();
        let view = session_to_view(&session);
        let metrics = capture_session_payload_metrics(&session, &view);
        let view_ms = elapsed_ms(view_start);

        let insert_start = Instant::now();
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| AppError::System("Capture session lock poisoned".to_string()))?;
        sessions.insert(id, session);
        let insert_ms = elapsed_ms(insert_start);

        log::info!(
            "[capture-perf] create_session monitors={} candidates={} cursor={} snapshot_png_bytes={} cursor_png_bytes={} view_base64_bytes={} snapshots_ms={:.1} candidates_ms={:.1} cursor_ms={:.1} view_ms={:.1} insert_ms={:.1} total_ms={:.1}",
            view.monitors.len(),
            view.candidates.len(),
            view.captured_cursor.is_some(),
            metrics.snapshot_png_bytes,
            metrics.cursor_png_bytes,
            metrics.view_base64_bytes,
            snapshots_ms,
            window_candidates_ms,
            captured_cursor_ms,
            view_ms,
            insert_ms,
            elapsed_ms(total_start),
        );

        Ok(view)
    }

    pub async fn capture_session_snapshot_cache(&self) -> Result<CaptureSessionSnapshotCache> {
        let snapshots = self.screenshot_backend.capture_monitor_snapshots().await?;
        if snapshots.is_empty() {
            return Err(AppError::System(
                "Cannot cache capture session without monitor snapshots".to_string(),
            ));
        }

        let captured_cursor = self
            .screenshot_backend
            .capture_cursor(&snapshots)
            .await
            .map_err(|err| {
                log::warn!("Failed to capture cursor while hydrating session: {}", err);
                err
            })
            .unwrap_or_default();

        Ok(CaptureSessionSnapshotCache {
            snapshots,
            captured_cursor,
        })
    }

    pub fn store_session_snapshot_cache(
        &self,
        id: &CaptureSessionId,
        cache: CaptureSessionSnapshotCache,
    ) -> Result<CaptureSessionView> {
        let cached_snapshots = cache
            .snapshots
            .into_iter()
            .map(|snapshot| (snapshot.id.clone(), snapshot))
            .collect::<HashMap<_, _>>();
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| AppError::System("Capture session lock poisoned".to_string()))?;
        let session = sessions
            .get_mut(id)
            .ok_or_else(|| AppError::System(format!("Capture session not found: {}", id.0)))?;
        let snapshots = session
            .layout_snapshots
            .iter()
            .map(|layout| {
                let cached = cached_snapshots.get(&layout.id).ok_or_else(|| {
                    AppError::System(format!(
                        "Capture session snapshot cache is missing monitor: {}",
                        layout.id
                    ))
                })?;
                let mut snapshot = layout.clone();
                snapshot.png_data = cached.png_data.clone();
                Ok(snapshot)
            })
            .collect::<Result<Vec<_>>>()?;
        session.snapshots = snapshots;
        session.captured_cursor = cache.captured_cursor;

        Ok(session_to_view(session))
    }

    pub async fn create_layout_session(&self) -> Result<CaptureSessionView> {
        self.create_layout_session_with_hidden_window_labels(Vec::new())
            .await
    }

    pub async fn create_layout_session_with_hidden_window_labels(
        &self,
        hidden_window_labels: Vec<String>,
    ) -> Result<CaptureSessionView> {
        let total_start = Instant::now();

        let layouts_start = Instant::now();
        let layouts = self.screenshot_backend.capture_monitor_layouts().await?;
        let layouts_ms = elapsed_ms(layouts_start);
        if layouts.is_empty() {
            return Err(AppError::System(
                "Cannot create capture session without monitor layout".to_string(),
            ));
        }

        let layout_snapshots = layouts
            .into_iter()
            .map(|layout| monitor_snapshot_from_layout(layout, Vec::new()))
            .collect::<Vec<_>>();

        let window_candidates_start = Instant::now();
        let window_candidates = self
            .screenshot_backend
            .capture_window_candidates(&layout_snapshots)
            .await
            .map_err(|err| {
                log::warn!("Failed to capture window candidates: {}", err);
                err
            })
            .unwrap_or_default();
        let window_candidates_ms = elapsed_ms(window_candidates_start);
        let candidates = window_candidates
            .iter()
            .enumerate()
            .map(|(index, candidate)| window_candidate_to_view(candidate, index))
            .collect::<Vec<_>>();

        let id = CaptureSessionId(generate_session_id());
        let session = CaptureSession {
            id: id.clone(),
            layout_snapshots: layout_snapshots.clone(),
            snapshots: layout_snapshots,
            candidates,
            captured_cursor: None,
            hidden_window_labels,
            created_at: SystemTime::now(),
        };

        let view = session_to_view(&session);
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| AppError::System("Capture session lock poisoned".to_string()))?;
        sessions.insert(id, session);

        log::info!(
            "[capture-perf] create_layout_session monitors={} candidates={} layouts_ms={:.1} candidates_ms={:.1} total_ms={:.1}",
            view.monitors.len(),
            view.candidates.len(),
            layouts_ms,
            window_candidates_ms,
            elapsed_ms(total_start),
        );

        Ok(view)
    }

    pub async fn hydrate_session_snapshots(
        &self,
        id: &CaptureSessionId,
    ) -> Result<CaptureSessionView> {
        loop {
            if let Some(view) = self.hydrated_session_view(id)? {
                return Ok(view);
            }

            if self.try_begin_session_hydration(id)? {
                let result = self.capture_and_store_session_snapshots(id).await;
                self.finish_session_hydration(id)?;
                self.hydration_notify.notify_waiters();
                return result;
            }

            self.hydration_notify.notified().await;
        }
    }

    async fn capture_and_store_session_snapshots(
        &self,
        id: &CaptureSessionId,
    ) -> Result<CaptureSessionView> {
        self.get_session(id)?;

        let cache = self.capture_session_snapshot_cache().await?;

        self.store_session_snapshot_cache(id, cache)
    }

    fn try_begin_session_hydration(&self, id: &CaptureSessionId) -> Result<bool> {
        self.get_session(id)?;

        let mut hydrating_sessions = self
            .hydrating_sessions
            .lock()
            .map_err(|_| AppError::System("Capture hydration lock poisoned".to_string()))?;

        Ok(hydrating_sessions.insert(id.clone()))
    }

    fn finish_session_hydration(&self, id: &CaptureSessionId) -> Result<()> {
        let mut hydrating_sessions = self
            .hydrating_sessions
            .lock()
            .map_err(|_| AppError::System("Capture hydration lock poisoned".to_string()))?;
        hydrating_sessions.remove(id);
        Ok(())
    }

    pub async fn freeze_session_selection(
        &self,
        id: &CaptureSessionId,
        rect: &LogicalRect,
    ) -> Result<CaptureSessionView> {
        let session = self.get_session(id)?;
        if session_snapshots_cover_rect(&session, rect) {
            return Ok(session_to_view(&session));
        }

        let snapshots = self
            .capture_selection_snapshots(&session.layout_snapshots, rect)
            .await?;

        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| AppError::System("Capture session lock poisoned".to_string()))?;
        let session = sessions
            .get_mut(id)
            .ok_or_else(|| AppError::System(format!("Capture session not found: {}", id.0)))?;
        session.snapshots = snapshots;

        Ok(session_to_view(session))
    }

    pub fn session_selection_needs_freeze(
        &self,
        id: &CaptureSessionId,
        rect: &LogicalRect,
    ) -> Result<bool> {
        let session = self.get_session(id)?;

        Ok(!session_snapshots_cover_rect(&session, rect))
    }

    pub fn take_hidden_window_labels(&self, id: &CaptureSessionId) -> Result<Vec<String>> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| AppError::System("Capture session lock poisoned".to_string()))?;

        Ok(sessions
            .get_mut(id)
            .map(|session| std::mem::take(&mut session.hidden_window_labels))
            .unwrap_or_default())
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

    fn hydrated_session_view(&self, id: &CaptureSessionId) -> Result<Option<CaptureSessionView>> {
        let sessions = self
            .sessions
            .lock()
            .map_err(|_| AppError::System("Capture session lock poisoned".to_string()))?;
        let session = sessions
            .get(id)
            .ok_or_else(|| AppError::System(format!("Capture session not found: {}", id.0)))?;

        Ok(session_has_cached_monitor_snapshots(session).then(|| session_to_view(session)))
    }

    pub fn get_session_view(&self, id: &CaptureSessionId) -> Result<CaptureSessionView> {
        let session = self.get_session(id)?;

        Ok(session_to_view(&session))
    }

    pub fn get_session_view_without_monitor_images(
        &self,
        id: &CaptureSessionId,
    ) -> Result<CaptureSessionView> {
        let session = self.get_session(id)?;

        Ok(session_to_view_without_monitor_images(&session))
    }

    pub fn current_cursor_position(&self, id: &CaptureSessionId) -> Result<Option<LogicalPoint>> {
        let session = self.get_session(id)?;

        self.screenshot_backend
            .current_cursor_position(&session.layout_snapshots)
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
            .layout_snapshots
            .iter()
            .find(|snapshot| logical_rects_intersect(rect, &snapshot.logical_bounds))
            .ok_or_else(|| {
                AppError::System("Selection does not intersect any captured monitor".to_string())
            })?;

        logical_rect_to_snapshot_physical(rect, snapshot)
    }

    async fn capture_selection_snapshots(
        &self,
        layout_snapshots: &[MonitorSnapshot],
        rect: &LogicalRect,
    ) -> Result<Vec<MonitorSnapshot>> {
        let mut snapshots = Vec::new();

        for layout in layout_snapshots {
            let Some(intersection) = logical_rect_intersection(rect, &layout.logical_bounds) else {
                continue;
            };
            let physical_rect = logical_rect_to_snapshot_physical(&intersection, layout)?;
            let png_data = self
                .screenshot_backend
                .capture_region(ScreenRegion {
                    x: physical_rect.x,
                    y: physical_rect.y,
                    width: physical_rect.width,
                    height: physical_rect.height,
                })
                .await?;

            snapshots.push(MonitorSnapshot {
                id: layout.id.clone(),
                logical_bounds: intersection,
                physical_bounds: physical_rect,
                scale_factor: layout.scale_factor,
                png_data,
            });
        }

        if snapshots.is_empty() {
            return Err(AppError::System(
                "Selection does not intersect any captured monitor".to_string(),
            ));
        }

        Ok(snapshots)
    }
}

fn session_to_view(session: &CaptureSession) -> CaptureSessionView {
    CaptureSessionView {
        id: session.id.clone(),
        monitors: session.snapshots.iter().map(snapshot_to_view).collect(),
        candidates: session.candidates.clone(),
        captured_cursor: session
            .captured_cursor
            .as_ref()
            .map(captured_cursor_to_view),
    }
}

fn session_to_view_without_monitor_images(session: &CaptureSession) -> CaptureSessionView {
    CaptureSessionView {
        id: session.id.clone(),
        monitors: session
            .layout_snapshots
            .iter()
            .map(snapshot_to_view)
            .collect(),
        candidates: session.candidates.clone(),
        captured_cursor: session
            .captured_cursor
            .as_ref()
            .map(captured_cursor_to_view),
    }
}

const WINDOW_CANDIDATE_BASE_PRIORITY: i32 = 10_000;

fn window_candidate_to_view(candidate: &WindowCandidate, index: usize) -> CaptureCandidateView {
    CaptureCandidateView {
        id: candidate.id.clone(),
        kind: "window".to_string(),
        rect: candidate.logical_bounds.clone(),
        priority: window_candidate_priority(index),
    }
}

fn window_candidate_priority(index: usize) -> i32 {
    WINDOW_CANDIDATE_BASE_PRIORITY.saturating_sub(index as i32)
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

fn captured_cursor_to_view(cursor: &CapturedCursor) -> CapturedCursorView {
    CapturedCursorView {
        logical_position: cursor.logical_position.clone(),
        hotspot: cursor.hotspot.clone(),
        image_width: cursor.image_width,
        image_height: cursor.image_height,
        scale_factor: cursor.scale_factor,
        image_base64: base64::engine::general_purpose::STANDARD.encode(&cursor.png_data),
    }
}

fn snapshot_without_pixels(mut snapshot: MonitorSnapshot) -> MonitorSnapshot {
    snapshot.png_data.clear();
    snapshot
}

fn session_snapshots_cover_rect(session: &CaptureSession, rect: &LogicalRect) -> bool {
    let required_area = snapshots_intersection_area(rect, &session.layout_snapshots, false);
    if required_area <= 0.0 {
        return false;
    }

    let captured_area = snapshots_intersection_area(rect, &session.snapshots, true);

    captured_area + f64::EPSILON >= required_area
}

fn session_has_cached_monitor_snapshots(session: &CaptureSession) -> bool {
    let Some(bounds) = snapshots_logical_bounds(&session.layout_snapshots) else {
        return false;
    };

    session_snapshots_cover_rect(session, &bounds)
}

fn snapshots_logical_bounds(snapshots: &[MonitorSnapshot]) -> Option<LogicalRect> {
    if snapshots.is_empty() {
        return None;
    }

    let left = snapshots
        .iter()
        .map(|snapshot| snapshot.logical_bounds.x)
        .fold(f64::INFINITY, f64::min);
    let top = snapshots
        .iter()
        .map(|snapshot| snapshot.logical_bounds.y)
        .fold(f64::INFINITY, f64::min);
    let right = snapshots
        .iter()
        .map(|snapshot| snapshot.logical_bounds.x + snapshot.logical_bounds.width)
        .fold(f64::NEG_INFINITY, f64::max);
    let bottom = snapshots
        .iter()
        .map(|snapshot| snapshot.logical_bounds.y + snapshot.logical_bounds.height)
        .fold(f64::NEG_INFINITY, f64::max);

    Some(LogicalRect {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    })
}

fn snapshots_intersection_area(
    rect: &LogicalRect,
    snapshots: &[MonitorSnapshot],
    require_pixels: bool,
) -> f64 {
    snapshots
        .iter()
        .filter(|snapshot| !require_pixels || !snapshot.png_data.is_empty())
        .filter_map(|snapshot| logical_rect_intersection(rect, &snapshot.logical_bounds))
        .map(|intersection| intersection.width * intersection.height)
        .sum()
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

#[derive(Debug, PartialEq, Eq)]
struct CaptureSessionPayloadMetrics {
    snapshot_png_bytes: usize,
    cursor_png_bytes: usize,
    view_base64_bytes: usize,
}

fn capture_session_payload_metrics(
    session: &CaptureSession,
    view: &CaptureSessionView,
) -> CaptureSessionPayloadMetrics {
    CaptureSessionPayloadMetrics {
        snapshot_png_bytes: session
            .snapshots
            .iter()
            .map(|snapshot| snapshot.png_data.len())
            .sum(),
        cursor_png_bytes: session
            .captured_cursor
            .as_ref()
            .map(|cursor| cursor.png_data.len())
            .unwrap_or_default(),
        view_base64_bytes: view
            .monitors
            .iter()
            .map(|monitor| monitor.image_base64.len())
            .sum::<usize>()
            + view
                .captured_cursor
                .as_ref()
                .map(|cursor| cursor.image_base64.len())
                .unwrap_or_default(),
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

fn elapsed_ms(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}

#[cfg(test)]
mod payload_metrics_tests {
    use super::*;
    use crate::domain::capture::LogicalPoint;

    #[test]
    fn capture_session_payload_metrics_counts_snapshot_cursor_and_base64_bytes() {
        let session = CaptureSession {
            id: CaptureSessionId("capture-test".to_string()),
            layout_snapshots: Vec::new(),
            snapshots: vec![
                MonitorSnapshot {
                    id: "primary".to_string(),
                    logical_bounds: LogicalRect {
                        x: 0.0,
                        y: 0.0,
                        width: 10.0,
                        height: 10.0,
                    },
                    physical_bounds: PhysicalRect {
                        x: 0,
                        y: 0,
                        width: 10,
                        height: 10,
                    },
                    scale_factor: 1.0,
                    png_data: vec![1, 2, 3],
                },
                MonitorSnapshot {
                    id: "secondary".to_string(),
                    logical_bounds: LogicalRect {
                        x: 10.0,
                        y: 0.0,
                        width: 10.0,
                        height: 10.0,
                    },
                    physical_bounds: PhysicalRect {
                        x: 10,
                        y: 0,
                        width: 10,
                        height: 10,
                    },
                    scale_factor: 1.0,
                    png_data: vec![4, 5],
                },
            ],
            candidates: Vec::new(),
            captured_cursor: Some(CapturedCursor {
                logical_position: LogicalPoint { x: 1.0, y: 2.0 },
                hotspot: LogicalPoint { x: 0.0, y: 0.0 },
                image_width: 8,
                image_height: 8,
                scale_factor: 1.0,
                png_data: vec![6, 7, 8, 9],
            }),
            hidden_window_labels: Vec::new(),
            created_at: SystemTime::UNIX_EPOCH,
        };
        let view = session_to_view(&session);

        assert_eq!(
            capture_session_payload_metrics(&session, &view),
            CaptureSessionPayloadMetrics {
                snapshot_png_bytes: 5,
                cursor_png_bytes: 4,
                view_base64_bytes: 16,
            },
        );
    }
}

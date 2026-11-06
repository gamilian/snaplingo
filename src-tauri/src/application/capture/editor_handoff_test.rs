use std::sync::atomic::{AtomicI32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use image::ImageEncoder;

use super::*;
use crate::application::capture::{
    CaptureEditorToolbarPresentation, CaptureImageComposer, CaptureOutput,
    CaptureSessionRuntimeHost, CaptureSessionSource, CaptureSessions, CaptureWindowGeometry,
};
use crate::application::providers::ocr::OcrCoordinator;
use crate::domain::capture::{MonitorLayout, MonitorSnapshot, PhysicalRect, ScreenRegion};
use crate::infrastructure::storage::SqliteConfigStore;

struct Source;
#[async_trait]
impl CaptureSessionSource for Source {
    async fn capture_monitor_snapshots(&self) -> Result<Vec<MonitorSnapshot>> {
        let mut png = Vec::new();
        image::codecs::png::PngEncoder::new(&mut png)
            .write_image(
                &vec![100; 20 * 20 * 4],
                20,
                20,
                image::ExtendedColorType::Rgba8,
            )
            .unwrap();
        Ok(vec![crate::domain::capture::monitor_snapshot_from_layout(
            self.capture_monitor_layouts().await?.remove(0),
            png,
        )])
    }
    async fn capture_monitor_layouts(&self) -> Result<Vec<MonitorLayout>> {
        Ok(vec![MonitorLayout {
            id: "screen".into(),
            logical_bounds: LogicalRect {
                x: 0.0,
                y: 0.0,
                width: 20.0,
                height: 20.0,
            },
            physical_bounds: PhysicalRect {
                x: 0,
                y: 0,
                width: 20,
                height: 20,
            },
            scale_factor: 1.0,
        }])
    }
    async fn capture_region(&self, _: ScreenRegion) -> Result<Vec<u8>> {
        unreachable!()
    }
}

#[derive(Default)]
struct Host {
    error: Mutex<Option<String>>,
    opened: tokio::sync::Notify,
    open_count: AtomicUsize,
    finished: AtomicUsize,
    discarded: AtomicUsize,
    presentations: AtomicI32,
    toolbar_updates: AtomicUsize,
}
#[async_trait]
impl CaptureSessionRuntimeHost for Host {
    async fn begin_capture_presentation(&self) -> Result<()> {
        self.presentations.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    async fn end_capture_presentation(&self) -> Result<()> {
        self.presentations.fetch_sub(1, Ordering::SeqCst);
        Ok(())
    }
    async fn prepare_capture_window_for_reveal(
        &self,
        _: Option<&CaptureSessionId>,
        _: Option<&CaptureWindowGeometry>,
    ) -> Result<()> {
        Ok(())
    }
    async fn reveal_capture_window(&self, _: Option<&CaptureWindowGeometry>) -> Result<()> {
        Ok(())
    }
    async fn hide_capture_window(&self) -> Result<()> {
        Ok(())
    }
    async fn destroy_inactive_capture_window(&self) -> Result<()> {
        Ok(())
    }
    async fn open_capture_window_for_session(
        &self,
        _: &str,
        _: &str,
        _: &CaptureWindowGeometry,
    ) -> Result<()> {
        Ok(())
    }
    async fn restore_capture_snapshot_windows(&self, _: Vec<String>) -> Result<()> {
        Ok(())
    }
    async fn open_capture_editor_window(
        &self,
        _: &CaptureEditorSelection,
        _: &CaptureWindowGeometry,
    ) -> Result<()> {
        self.open_count.fetch_add(1, Ordering::SeqCst);
        self.opened.notify_one();
        self.error
            .lock()
            .unwrap()
            .clone()
            .map_or(Ok(()), |error| Err(error.into()))
    }
    async fn finish_capture_editor_transfer(&self, _: &CaptureSessionId) -> Result<()> {
        self.finished.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    async fn discard_capture_editor(&self) -> Result<()> {
        self.discarded.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    async fn update_capture_editor_toolbar(
        &self,
        _: &CaptureSessionId,
        _: Option<&CaptureEditorToolbarPresentation>,
    ) -> Result<bool> {
        self.toolbar_updates.fetch_add(1, Ordering::SeqCst);
        Ok(true)
    }
}
async fn fixture() -> (Arc<CaptureSessionRuntime>, Arc<Host>, CaptureSessionId) {
    let host = Arc::new(Host::default());
    let runtime = Arc::new(CaptureSessionRuntime::with_host(
        Arc::new(CaptureSessions::new(Arc::new(Source))),
        Arc::new(CaptureImageComposer::new()),
        Arc::new(CaptureOutput::new()),
        Arc::new(OcrCoordinator::new(Arc::new(SqliteConfigStore::new_temp()))),
        host.clone(),
        Arc::new(super::super::runtime_host::UnconfiguredCapturePinOutput),
    ));
    let id = runtime
        .create_session_from_visible_desktop()
        .await
        .unwrap()
        .id;
    (runtime, host, id)
}
fn rect() -> LogicalRect {
    LogicalRect {
        x: 1.0,
        y: 2.0,
        width: 8.0,
        height: 6.0,
    }
}
async fn transfer(
    runtime: &Arc<CaptureSessionRuntime>,
    host: &Host,
    id: &CaptureSessionId,
) -> tokio::task::JoinHandle<Result<()>> {
    let runtime = runtime.clone();
    let id = id.clone();
    let task = tokio::spawn(async move { runtime.open_capture_editor(&id, &rect(), true).await });
    host.opened.notified().await;
    task
}

#[tokio::test]
async fn frozen_selection_transfers_only_after_editor_reveal() {
    let (runtime, host, id) = fixture().await;
    let task = transfer(&runtime, &host, &id).await;
    let selection = runtime.capture_editor_selection(&id).unwrap().unwrap();
    assert_eq!(
        selection,
        CaptureEditorSelection {
            session_id: id.0.clone(),
            rect: rect(),
            include_cursor: true
        }
    );
    assert_eq!(host.finished.load(Ordering::SeqCst), 0);
    assert!(runtime.sessions.has_session(&id));
    assert!(runtime
        .reveal_capture_window(Some(&CaptureSessionId("old".into())))
        .await
        .is_err());
    runtime.reveal_capture_window(Some(&id)).await.unwrap();
    task.await.unwrap().unwrap();
    assert_eq!(host.finished.load(Ordering::SeqCst), 1);
    assert_eq!(host.presentations.load(Ordering::SeqCst), 1);
    assert!(runtime.sessions.has_session(&id));
    assert!(runtime.capture_editor_selection(&id).unwrap().is_none());
    runtime
        .cancel_capture_session_from_window(&id)
        .await
        .unwrap();
    assert!(!runtime.sessions.has_session(&id));
    assert_eq!(host.presentations.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn failed_editor_can_retry_the_same_frozen_selection() {
    let (runtime, host, id) = fixture().await;
    *host.error.lock().unwrap() = Some("Editor failed to load".into());
    assert!(transfer(&runtime, &host, &id).await.await.unwrap().is_err());
    runtime
        .cancel_capture_session_from_window(&id)
        .await
        .unwrap();
    assert!(runtime.sessions.has_session(&id));
    assert_eq!(host.presentations.load(Ordering::SeqCst), 1);
    assert_eq!(host.discarded.load(Ordering::SeqCst), 1);
    assert!(runtime
        .prepare_capture_window_for_reveal(Some(&id))
        .await
        .is_err());
    *host.error.lock().unwrap() = None;
    let task = transfer(&runtime, &host, &id).await;
    runtime.reveal_capture_window(Some(&id)).await.unwrap();
    task.await.unwrap().unwrap();
    assert_eq!(host.finished.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn cancellation_rejects_late_editor_reveal_and_releases_session() {
    let (runtime, host, id) = fixture().await;
    let task = transfer(&runtime, &host, &id).await;
    tokio::time::timeout(Duration::from_secs(1), runtime.cancel_capture_session(&id))
        .await
        .expect("Cancellation must interrupt editor startup before taking its lock")
        .unwrap();
    assert!(runtime.reveal_capture_window(Some(&id)).await.is_err());
    assert!(task.await.unwrap().is_err());
    assert_eq!(host.finished.load(Ordering::SeqCst), 0);
    assert_eq!(host.discarded.load(Ordering::SeqCst), 1);
    assert!(!runtime.sessions.has_session(&id));
}

#[tokio::test]
async fn closing_unrevealed_editor_retains_native_selection_for_recovery() {
    let (runtime, host, id) = fixture().await;
    let task = transfer(&runtime, &host, &id).await;
    runtime
        .cancel_capture_session_from_window(&id)
        .await
        .unwrap();
    assert!(task.await.unwrap().is_err());
    assert!(runtime.sessions.has_session(&id));
    assert_eq!(host.presentations.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn stale_selection_cannot_start_an_editor() {
    let (runtime, host, id) = fixture().await;
    runtime.create_session_from_visible_desktop().await.unwrap();
    assert!(runtime
        .open_capture_editor(&id, &rect(), false)
        .await
        .is_err());
    assert_eq!(host.open_count.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn unrevealed_editor_times_out_without_losing_native_pixels() {
    let (runtime, host, id) = fixture().await;
    let task = transfer(&runtime, &host, &id).await;
    assert!(task
        .await
        .unwrap()
        .unwrap_err()
        .to_string()
        .contains("Timed out"));
    assert!(runtime.sessions.has_session(&id));
    assert_eq!(host.discarded.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn toolbar_rejects_old_presentations_but_permits_owned_cleanup() {
    let (runtime, host, id) = fixture().await;
    let presentation = CaptureEditorToolbarPresentation {
        frame: rect(),
        ui_scale: 1.0,
        dark: false,
        visible: true,
        active_tool: None,
        can_undo: false,
        can_redo: false,
        busy: false,
        styling: crate::application::capture::CaptureEditorStylePresentation::default(),
    };
    assert!(runtime
        .update_capture_editor_toolbar(&id, Some(presentation.clone()))
        .await
        .unwrap());
    runtime.create_session_from_visible_desktop().await.unwrap();
    assert!(!runtime
        .update_capture_editor_toolbar(&id, Some(presentation))
        .await
        .unwrap());
    assert_eq!(host.toolbar_updates.load(Ordering::SeqCst), 1);
    assert!(runtime
        .update_capture_editor_toolbar(&id, None)
        .await
        .unwrap());
    assert_eq!(host.toolbar_updates.load(Ordering::SeqCst), 2);
}

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::Arc;

use super::{CaptureSessionSource, CaptureSessions};
use crate::domain::capture::{
    ControlCandidate, LogicalPoint, LogicalRect, MonitorLayout, MonitorSnapshot, PhysicalRect,
    ScreenRegion,
};
use crate::error::AppError;

thread_local! {
    static ALLOCATED_BYTES: Cell<Option<usize>> = const { Cell::new(None) };
}

struct TrackingAllocator;

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = System.alloc(layout);
        if !pointer.is_null() {
            let _ = ALLOCATED_BYTES.try_with(|bytes| {
                if let Some(total) = bytes.get() {
                    bytes.set(Some(total + layout.size()));
                }
            });
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        System.dealloc(pointer, layout);
    }
}

#[global_allocator]
static ALLOCATOR: TrackingAllocator = TrackingAllocator;

fn measured_allocations<T>(operation: impl FnOnce() -> T) -> (T, usize) {
    ALLOCATED_BYTES.with(|bytes| bytes.set(Some(0)));
    let result = operation();
    let allocated = ALLOCATED_BYTES.with(|bytes| bytes.replace(None).unwrap());
    (result, allocated)
}

const PIXEL_PAYLOAD_BYTES: usize = 4 * 1024 * 1024;
const METADATA_ALLOCATION_BUDGET: usize = 64 * 1024;

struct FixtureSource;

#[async_trait::async_trait]
impl CaptureSessionSource for FixtureSource {
    async fn capture_monitor_snapshots(&self) -> Result<Vec<MonitorSnapshot>, AppError> {
        Ok(vec![MonitorSnapshot {
            id: "display".into(),
            logical_bounds: LogicalRect {
                x: 0.0,
                y: 0.0,
                width: 1024.0,
                height: 1024.0,
            },
            physical_bounds: PhysicalRect {
                x: 0,
                y: 0,
                width: 1024,
                height: 1024,
            },
            scale_factor: 1.0,
            // Metadata queries must neither decode nor duplicate this opaque payload.
            png_data: vec![7; PIXEL_PAYLOAD_BYTES],
        }])
    }

    async fn capture_monitor_layouts(&self) -> Result<Vec<MonitorLayout>, AppError> {
        unreachable!("this fixture starts with frozen pixels")
    }

    async fn capture_control_candidate(
        &self,
        _point: &LogicalPoint,
        monitors: &[MonitorSnapshot],
    ) -> Result<Option<ControlCandidate>, AppError> {
        assert_eq!(monitors.len(), 1);
        assert!(monitors[0].png_data.is_empty());
        Ok(None)
    }

    async fn capture_region(&self, _region: ScreenRegion) -> Result<Vec<u8>, AppError> {
        unreachable!("metadata queries must not capture the desktop")
    }
}

#[test]
fn geometry_queries_do_not_duplicate_frozen_screen_pixels() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let sessions = CaptureSessions::new(Arc::new(FixtureSource));
    let (session, startup_bytes) = measured_allocations(|| {
        runtime
            .block_on(sessions.create_session_without_monitor_images())
            .unwrap()
    });
    let (_, cursor_bytes) =
        measured_allocations(|| sessions.current_cursor_position(&session.id).unwrap());
    let (_, control_bytes) = measured_allocations(|| {
        runtime
            .block_on(
                sessions.control_candidate_at(&session.id, &LogicalPoint { x: 10.0, y: 20.0 }),
            )
            .unwrap()
    });
    let (rect, geometry_bytes) = measured_allocations(|| {
        sessions
            .logical_rect_to_physical(
                &session.id,
                &LogicalRect {
                    x: 10.0,
                    y: 20.0,
                    width: 30.0,
                    height: 40.0,
                },
            )
            .unwrap()
    });
    assert_eq!(
        rect,
        PhysicalRect {
            x: 10,
            y: 20,
            width: 30,
            height: 40
        }
    );

    let allocations = [
        (
            "startup excluding the original pixels",
            startup_bytes - PIXEL_PAYLOAD_BYTES,
        ),
        ("cursor position", cursor_bytes),
        ("control hit test", control_bytes),
        ("coordinate conversion", geometry_bytes),
    ];
    for (operation, bytes) in allocations {
        eprintln!("{operation}: {bytes} allocated bytes");
    }
    for (operation, bytes) in allocations {
        assert!(
            bytes < METADATA_ALLOCATION_BUDGET,
            "{operation} allocated {bytes} bytes while querying a session with a 4 MiB image",
        );
    }
}

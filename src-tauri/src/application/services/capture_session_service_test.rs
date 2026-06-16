#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::error::AppError;
    use crate::infrastructure::system::screenshot::{
        MonitorSnapshot, ScreenRegion, ScreenshotBackend,
    };

    use crate::application::services::capture_session_service::CaptureSessionService;

    struct MockScreenshotBackend {
        snapshot: MonitorSnapshot,
    }

    #[async_trait::async_trait]
    impl ScreenshotBackend for MockScreenshotBackend {
        async fn capture_monitor_snapshots(&self) -> Result<Vec<MonitorSnapshot>, AppError> {
            Ok(vec![self.snapshot.clone()])
        }

        async fn capture_full_screen(&self) -> Result<Vec<u8>, AppError> {
            Ok(self.snapshot.png_data.clone())
        }

        async fn capture_region(&self, _region: ScreenRegion) -> Result<Vec<u8>, AppError> {
            Ok(self.snapshot.png_data.clone())
        }
    }

    fn make_backend() -> MockScreenshotBackend {
        MockScreenshotBackend {
            snapshot: MonitorSnapshot {
                id: "primary".to_string(),
                logical_bounds: crate::domain::capture::LogicalRect {
                    x: 0.0,
                    y: 0.0,
                    width: 10.0,
                    height: 10.0,
                },
                physical_bounds: crate::domain::capture::PhysicalRect {
                    x: 0,
                    y: 0,
                    width: 10,
                    height: 10,
                },
                scale_factor: 1.0,
                png_data: vec![1, 2, 3],
            },
        }
    }

    #[tokio::test]
    async fn create_session_stores_snapshot_and_returns_view() {
        let service = CaptureSessionService::new(Arc::new(make_backend()));

        let view = service.create_session().await.unwrap();

        assert_eq!(view.monitors.len(), 1);
        assert_eq!(view.monitors[0].id, "primary");
        assert_eq!(view.monitors[0].image_base64, "AQID");
        assert!(service.has_session(&view.id));
    }

    #[tokio::test]
    async fn cancel_session_removes_stored_snapshot() {
        let service = CaptureSessionService::new(Arc::new(make_backend()));
        let view = service.create_session().await.unwrap();

        service.cancel_session(&view.id).unwrap();

        assert!(!service.has_session(&view.id));
    }
}

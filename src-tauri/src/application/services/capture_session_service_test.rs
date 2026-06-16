#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::domain::capture::LogicalRect;
    use crate::error::AppError;
    use crate::infrastructure::system::screenshot::{
        MonitorSnapshot, ScreenRegion, ScreenshotBackend, WindowCandidate,
    };

    use crate::application::services::capture_session_service::CaptureSessionService;

    struct MockScreenshotBackend {
        snapshots: Vec<MonitorSnapshot>,
        window_candidates: Vec<WindowCandidate>,
    }

    #[async_trait::async_trait]
    impl ScreenshotBackend for MockScreenshotBackend {
        async fn capture_monitor_snapshots(&self) -> Result<Vec<MonitorSnapshot>, AppError> {
            Ok(self.snapshots.clone())
        }

        async fn capture_window_candidates(
            &self,
            _monitors: &[MonitorSnapshot],
        ) -> Result<Vec<WindowCandidate>, AppError> {
            Ok(self.window_candidates.clone())
        }

        async fn capture_full_screen(&self) -> Result<Vec<u8>, AppError> {
            Ok(self.snapshots[0].png_data.clone())
        }

        async fn capture_region(&self, _region: ScreenRegion) -> Result<Vec<u8>, AppError> {
            Ok(self.snapshots[0].png_data.clone())
        }
    }

    fn make_backend() -> MockScreenshotBackend {
        make_backend_with_scale(1.0)
    }

    fn make_backend_with_scale(scale_factor: f64) -> MockScreenshotBackend {
        MockScreenshotBackend {
            snapshots: vec![MonitorSnapshot {
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
                scale_factor,
                png_data: vec![1, 2, 3],
            }],
            window_candidates: Vec::new(),
        }
    }

    fn make_multi_monitor_backend() -> MockScreenshotBackend {
        MockScreenshotBackend {
            snapshots: vec![
                MonitorSnapshot {
                    id: "primary".to_string(),
                    logical_bounds: crate::domain::capture::LogicalRect {
                        x: 0.0,
                        y: 0.0,
                        width: 100.0,
                        height: 80.0,
                    },
                    physical_bounds: crate::domain::capture::PhysicalRect {
                        x: 0,
                        y: 0,
                        width: 200,
                        height: 160,
                    },
                    scale_factor: 2.0,
                    png_data: vec![1, 2, 3],
                },
                MonitorSnapshot {
                    id: "left".to_string(),
                    logical_bounds: crate::domain::capture::LogicalRect {
                        x: -120.0,
                        y: 0.0,
                        width: 120.0,
                        height: 90.0,
                    },
                    physical_bounds: crate::domain::capture::PhysicalRect {
                        x: -120,
                        y: 0,
                        width: 120,
                        height: 90,
                    },
                    scale_factor: 1.0,
                    png_data: vec![4, 5, 6],
                },
            ],
            window_candidates: Vec::new(),
        }
    }

    fn make_backend_with_window_candidate() -> MockScreenshotBackend {
        let mut backend = make_backend();
        backend.window_candidates = vec![WindowCandidate {
            id: "window-42".to_string(),
            title: "Settings".to_string(),
            app_name: "System Settings".to_string(),
            logical_bounds: LogicalRect {
                x: 2.0,
                y: 3.0,
                width: 4.0,
                height: 5.0,
            },
        }];
        backend
    }

    #[tokio::test]
    async fn create_session_stores_snapshot_and_returns_view() {
        let service = CaptureSessionService::new(Arc::new(make_backend()));

        let view = service.create_session().await.unwrap();

        assert_eq!(view.monitors.len(), 1);
        assert_eq!(view.monitors[0].id, "primary");
        assert_eq!(view.monitors[0].image_base64, "AQID");
        assert!(view.candidates.is_empty());
        assert!(service.has_session(&view.id));
    }

    #[tokio::test]
    async fn create_session_returns_window_candidates_for_hover_selection() {
        let service = CaptureSessionService::new(Arc::new(make_backend_with_window_candidate()));

        let view = service.create_session().await.unwrap();

        assert_eq!(view.candidates.len(), 1);
        assert_eq!(view.candidates[0].id, "window-42");
        assert_eq!(view.candidates[0].kind, "window");
        assert_eq!(
            view.candidates[0].rect,
            LogicalRect {
                x: 2.0,
                y: 3.0,
                width: 4.0,
                height: 5.0,
            }
        );
        assert_eq!(view.candidates[0].priority, 10);
    }

    #[tokio::test]
    async fn cancel_session_removes_stored_snapshot() {
        let service = CaptureSessionService::new(Arc::new(make_backend()));
        let view = service.create_session().await.unwrap();

        service.cancel_session(&view.id).unwrap();

        assert!(!service.has_session(&view.id));
    }

    #[tokio::test]
    async fn converts_logical_rect_to_physical_rect() {
        let service = CaptureSessionService::new(Arc::new(make_backend_with_scale(2.0)));
        let view = service.create_session().await.unwrap();

        let physical = service
            .logical_rect_to_physical(
                &view.id,
                &crate::domain::capture::LogicalRect {
                    x: 1.0,
                    y: 2.0,
                    width: 3.0,
                    height: 4.0,
                },
            )
            .unwrap();

        assert_eq!(physical.x, 2);
        assert_eq!(physical.y, 4);
        assert_eq!(physical.width, 6);
        assert_eq!(physical.height, 8);
    }

    #[tokio::test]
    async fn clamps_logical_rect_to_monitor_bounds() {
        let service = CaptureSessionService::new(Arc::new(make_backend_with_scale(2.0)));
        let view = service.create_session().await.unwrap();

        let physical = service
            .logical_rect_to_physical(
                &view.id,
                &crate::domain::capture::LogicalRect {
                    x: -1.0,
                    y: -2.0,
                    width: 4.0,
                    height: 5.0,
                },
            )
            .unwrap();

        assert_eq!(physical.x, 0);
        assert_eq!(physical.y, 0);
        assert_eq!(physical.width, 6);
        assert_eq!(physical.height, 6);
    }

    #[tokio::test]
    async fn converts_logical_rect_on_secondary_monitor_to_that_monitor_physical_space() {
        let service = CaptureSessionService::new(Arc::new(make_multi_monitor_backend()));
        let view = service.create_session().await.unwrap();

        let physical = service
            .logical_rect_to_physical(
                &view.id,
                &crate::domain::capture::LogicalRect {
                    x: -110.0,
                    y: 10.0,
                    width: 20.0,
                    height: 30.0,
                },
            )
            .unwrap();

        assert_eq!(
            physical,
            crate::domain::capture::PhysicalRect {
                x: -110,
                y: 10,
                width: 20,
                height: 30,
            }
        );
    }
}

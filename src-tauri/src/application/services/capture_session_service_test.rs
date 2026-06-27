#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::{SystemTime, UNIX_EPOCH};

    use base64::Engine;
    use image::ImageEncoder;

    use crate::application::providers::common::Provider;
    use crate::application::providers::ocr::{OcrCoordinator, OcrProvider};
    use crate::application::services::{
        CaptureOutputService, CaptureSessionOutput, CaptureSessionRuntime,
    };
    use crate::domain::capture::{CaptureOutputAction, LogicalPoint, LogicalRect};
    use crate::domain::ocr::{OcrRequest, OcrResult};
    use crate::error::AppError;
    use crate::infrastructure::storage::ConfigFile;
    use crate::infrastructure::system::screenshot::{
        CapturedCursor, MonitorLayout, MonitorSnapshot, ScreenRegion, ScreenshotBackend,
        WindowCandidate,
    };

    use crate::application::services::capture_session_service::CaptureSessionService;
    use crate::application::services::image_composition_service::ImageCompositionService;

    struct MockScreenshotBackend {
        snapshots: Vec<MonitorSnapshot>,
        monitor_layouts: Vec<MonitorLayout>,
        window_candidates: Vec<WindowCandidate>,
        captured_cursor: Option<CapturedCursor>,
        current_cursor_position: Option<LogicalPoint>,
        capture_monitor_snapshots_calls: Arc<Mutex<usize>>,
        capture_monitor_layouts_calls: Arc<Mutex<usize>>,
        captured_regions: Arc<Mutex<Vec<ScreenRegion>>>,
        region_png_data: Vec<u8>,
    }

    struct RecordingOcrProvider {
        observed_request: Arc<Mutex<Option<OcrRequest>>>,
    }

    impl Provider for RecordingOcrProvider {
        fn id(&self) -> &str {
            "recording"
        }

        fn name(&self) -> &str {
            "Recording OCR"
        }

        fn is_configured(&self) -> bool {
            true
        }

        fn requires_api_key(&self) -> bool {
            false
        }
    }

    #[async_trait::async_trait]
    impl OcrProvider for RecordingOcrProvider {
        async fn recognize(&self, request: &OcrRequest) -> crate::Result<OcrResult> {
            *self.observed_request.lock().unwrap() = Some(request.clone());

            Ok(OcrResult {
                text: "recorded".to_string(),
                confidence: Some(1.0),
            })
        }
    }

    #[async_trait::async_trait]
    impl ScreenshotBackend for MockScreenshotBackend {
        async fn capture_monitor_snapshots(&self) -> Result<Vec<MonitorSnapshot>, AppError> {
            *self.capture_monitor_snapshots_calls.lock().unwrap() += 1;
            Ok(self.snapshots.clone())
        }

        async fn capture_monitor_layouts(&self) -> Result<Vec<MonitorLayout>, AppError> {
            *self.capture_monitor_layouts_calls.lock().unwrap() += 1;
            Ok(self.monitor_layouts.clone())
        }

        async fn capture_window_candidates(
            &self,
            _monitors: &[MonitorSnapshot],
        ) -> Result<Vec<WindowCandidate>, AppError> {
            Ok(self.window_candidates.clone())
        }

        async fn capture_cursor(
            &self,
            _monitors: &[MonitorSnapshot],
        ) -> Result<Option<CapturedCursor>, AppError> {
            Ok(self.captured_cursor.clone())
        }

        fn current_cursor_position(
            &self,
            _monitors: &[MonitorSnapshot],
        ) -> Result<Option<LogicalPoint>, AppError> {
            Ok(self.current_cursor_position.clone())
        }

        async fn capture_full_screen(&self) -> Result<Vec<u8>, AppError> {
            Ok(self.snapshots[0].png_data.clone())
        }

        async fn capture_region(&self, region: ScreenRegion) -> Result<Vec<u8>, AppError> {
            self.captured_regions.lock().unwrap().push(region);
            Ok(self.region_png_data.clone())
        }
    }

    fn make_backend() -> MockScreenshotBackend {
        make_backend_with_scale(1.0)
    }

    fn make_backend_with_scale(scale_factor: f64) -> MockScreenshotBackend {
        let snapshots = vec![MonitorSnapshot {
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
        }];
        MockScreenshotBackend {
            monitor_layouts: snapshots.iter().map(monitor_layout_from_snapshot).collect(),
            snapshots,
            window_candidates: Vec::new(),
            captured_cursor: None,
            current_cursor_position: None,
            capture_monitor_snapshots_calls: Arc::new(Mutex::new(0)),
            capture_monitor_layouts_calls: Arc::new(Mutex::new(0)),
            captured_regions: Arc::new(Mutex::new(Vec::new())),
            region_png_data: vec![1, 2, 3],
        }
    }

    fn make_multi_monitor_backend() -> MockScreenshotBackend {
        let snapshots = vec![
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
        ];
        MockScreenshotBackend {
            monitor_layouts: snapshots.iter().map(monitor_layout_from_snapshot).collect(),
            snapshots,
            window_candidates: Vec::new(),
            captured_cursor: None,
            current_cursor_position: None,
            capture_monitor_snapshots_calls: Arc::new(Mutex::new(0)),
            capture_monitor_layouts_calls: Arc::new(Mutex::new(0)),
            captured_regions: Arc::new(Mutex::new(Vec::new())),
            region_png_data: vec![1, 2, 3],
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

    fn make_backend_with_captured_cursor() -> MockScreenshotBackend {
        let mut backend = make_backend();
        backend.captured_cursor = Some(CapturedCursor {
            logical_position: crate::domain::capture::LogicalPoint { x: 4.0, y: 5.0 },
            hotspot: crate::domain::capture::LogicalPoint { x: 1.0, y: 2.0 },
            image_width: 16,
            image_height: 20,
            scale_factor: 2.0,
            png_data: vec![9, 8, 7],
        });
        backend
    }

    fn make_backend_with_current_cursor_position() -> MockScreenshotBackend {
        let mut backend = make_backend();
        backend.current_cursor_position =
            Some(crate::domain::capture::LogicalPoint { x: 6.0, y: 7.0 });
        backend
    }

    fn make_backend_with_renderable_png() -> MockScreenshotBackend {
        let mut backend = make_backend();
        backend.snapshots[0].logical_bounds = crate::domain::capture::LogicalRect {
            x: 0.0,
            y: 0.0,
            width: 4.0,
            height: 4.0,
        };
        backend.snapshots[0].physical_bounds = crate::domain::capture::PhysicalRect {
            x: 0,
            y: 0,
            width: 4,
            height: 4,
        };
        backend.snapshots[0].png_data = make_solid_png(4, 4, [10, 20, 30, 255]);
        backend.monitor_layouts = backend
            .snapshots
            .iter()
            .map(monitor_layout_from_snapshot)
            .collect();
        backend.region_png_data = backend.snapshots[0].png_data.clone();
        backend
    }

    fn monitor_layout_from_snapshot(snapshot: &MonitorSnapshot) -> MonitorLayout {
        MonitorLayout {
            id: snapshot.id.clone(),
            logical_bounds: snapshot.logical_bounds.clone(),
            physical_bounds: snapshot.physical_bounds.clone(),
            scale_factor: snapshot.scale_factor,
        }
    }

    fn make_solid_png(width: u32, height: u32, rgba: [u8; 4]) -> Vec<u8> {
        let pixels = rgba.repeat((width * height) as usize);
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
            .join("snaplingo-capture-session-output-tests")
            .join(format!("capture-{}.png", suffix))
    }

    #[tokio::test]
    async fn create_layout_session_returns_geometry_without_image_payload() {
        let backend = make_backend();
        let snapshot_calls = backend.capture_monitor_snapshots_calls.clone();
        let layout_calls = backend.capture_monitor_layouts_calls.clone();
        let service = CaptureSessionService::new(Arc::new(backend));

        let view = service.create_layout_session().await.unwrap();

        assert_eq!(*layout_calls.lock().unwrap(), 1);
        assert_eq!(*snapshot_calls.lock().unwrap(), 0);
        assert_eq!(view.monitors.len(), 1);
        assert_eq!(view.monitors[0].id, "primary");
        assert_eq!(view.monitors[0].image_base64, "");
        assert!(service.get_session(&view.id).unwrap().snapshots[0]
            .png_data
            .is_empty());
    }

    #[tokio::test]
    async fn freeze_session_selection_captures_only_selected_region() {
        let mut backend = make_backend_with_renderable_png();
        backend.region_png_data = make_solid_png(2, 3, [40, 50, 60, 255]);
        let captured_regions = backend.captured_regions.clone();
        let service = CaptureSessionService::new(Arc::new(backend));
        let view = service.create_layout_session().await.unwrap();

        let frozen_view = service
            .freeze_session_selection(
                &view.id,
                &LogicalRect {
                    x: 1.0,
                    y: 1.0,
                    width: 2.0,
                    height: 3.0,
                },
            )
            .await
            .unwrap();

        let regions = captured_regions.lock().unwrap();
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].x, 1);
        assert_eq!(regions[0].y, 1);
        assert_eq!(regions[0].width, 2);
        assert_eq!(regions[0].height, 3);
        assert_eq!(
            frozen_view.monitors[0].image_base64,
            base64::engine::general_purpose::STANDARD.encode(make_solid_png(
                2,
                3,
                [40, 50, 60, 255]
            ))
        );
        assert!(!service
            .session_selection_needs_freeze(
                &view.id,
                &LogicalRect {
                    x: 1.0,
                    y: 1.0,
                    width: 2.0,
                    height: 3.0,
                },
            )
            .unwrap());
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
    async fn get_session_view_returns_stored_view() {
        let service = CaptureSessionService::new(Arc::new(make_backend_with_captured_cursor()));

        let created_view = service.create_session().await.unwrap();
        let stored_view = service.get_session_view(&created_view.id).unwrap();

        assert_eq!(stored_view, created_view);
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
    async fn create_session_returns_captured_cursor_for_output_composition() {
        let service = CaptureSessionService::new(Arc::new(make_backend_with_captured_cursor()));

        let view = service.create_session().await.unwrap();

        let captured_cursor = view.captured_cursor.unwrap();
        assert_eq!(
            captured_cursor.logical_position,
            crate::domain::capture::LogicalPoint { x: 4.0, y: 5.0 }
        );
        assert_eq!(
            captured_cursor.hotspot,
            crate::domain::capture::LogicalPoint { x: 1.0, y: 2.0 }
        );
        assert_eq!(captured_cursor.image_width, 16);
        assert_eq!(captured_cursor.image_height, 20);
        assert_eq!(captured_cursor.scale_factor, 2.0);
        assert_eq!(captured_cursor.image_base64, "CQgH");

        let session = service.get_session(&view.id).unwrap();
        assert!(session.captured_cursor.is_some());
    }

    #[tokio::test]
    async fn returns_current_cursor_position_for_active_session() {
        let service =
            CaptureSessionService::new(Arc::new(make_backend_with_current_cursor_position()));
        let view = service.create_session().await.unwrap();

        let position = service.current_cursor_position(&view.id).unwrap();

        assert_eq!(
            position,
            Some(crate::domain::capture::LogicalPoint { x: 6.0, y: 7.0 })
        );
    }

    #[tokio::test]
    async fn cancel_session_removes_stored_snapshot() {
        let service = CaptureSessionService::new(Arc::new(make_backend()));
        let view = service.create_session().await.unwrap();

        service.cancel_session(&view.id).unwrap();

        assert!(!service.has_session(&view.id));
    }

    #[tokio::test]
    async fn hidden_window_labels_are_stored_until_explicit_restore() {
        let service = CaptureSessionService::new(Arc::new(make_backend()));
        let view = service
            .create_session_with_hidden_window_labels(vec![
                "main".to_string(),
                "pin-pin-1".to_string(),
            ])
            .await
            .unwrap();

        assert_eq!(
            service.take_hidden_window_labels(&view.id).unwrap(),
            vec!["main".to_string(), "pin-pin-1".to_string()]
        );
        assert!(service
            .take_hidden_window_labels(&view.id)
            .unwrap()
            .is_empty());
        assert!(service.has_session(&view.id));
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

    #[tokio::test]
    async fn recognize_selection_text_sends_raw_selection_png_to_ocr() {
        let observed_request = Arc::new(Mutex::new(None));
        let ocr = OcrCoordinator::new(Arc::new(ConfigFile::new_temp()));
        ocr.register(RecordingOcrProvider {
            observed_request: observed_request.clone(),
        })
        .unwrap();
        ocr.activate("recording").unwrap();

        let mut backend = make_backend_with_renderable_png();
        backend.captured_cursor = Some(CapturedCursor {
            logical_position: crate::domain::capture::LogicalPoint { x: 1.0, y: 1.0 },
            hotspot: crate::domain::capture::LogicalPoint { x: 0.0, y: 0.0 },
            image_width: 2,
            image_height: 2,
            scale_factor: 1.0,
            png_data: make_solid_png(2, 2, [255, 0, 0, 255]),
        });

        let service = CaptureSessionService::new(Arc::new(backend));
        let view = service.create_session().await.unwrap();

        let result = service
            .recognize_selection_text(
                &ImageCompositionService::new(),
                &ocr,
                &view.id,
                &LogicalRect {
                    x: 1.0,
                    y: 1.0,
                    width: 2.0,
                    height: 2.0,
                },
            )
            .await
            .unwrap();

        let request = observed_request.lock().unwrap().clone().unwrap();
        let decoded = image::load_from_memory(&request.image_data)
            .unwrap()
            .to_rgba8();

        assert_eq!(result.text, "recorded");
        assert_eq!(request.language, None);
        assert_eq!((decoded.width(), decoded.height()), (2, 2));
        assert!(decoded.pixels().all(|pixel| pixel.0 == [10, 20, 30, 255]));
    }

    #[tokio::test]
    async fn capture_session_runtime_recognizes_selection_text_through_one_interface() {
        let observed_request = Arc::new(Mutex::new(None));
        let ocr = OcrCoordinator::new(Arc::new(ConfigFile::new_temp()));
        ocr.register(RecordingOcrProvider {
            observed_request: observed_request.clone(),
        })
        .unwrap();
        ocr.activate("recording").unwrap();

        let sessions = Arc::new(CaptureSessionService::new(Arc::new(
            make_backend_with_renderable_png(),
        )));
        let runtime = CaptureSessionRuntime::new(
            sessions.clone(),
            Arc::new(ImageCompositionService::new()),
            Arc::new(CaptureOutputService::new()),
            Arc::new(ocr),
        );
        let view = sessions.create_session().await.unwrap();

        let result = runtime
            .recognize_selection_text(
                &view.id,
                &LogicalRect {
                    x: 1.0,
                    y: 1.0,
                    width: 2.0,
                    height: 2.0,
                },
            )
            .await
            .unwrap();

        assert_eq!(result.text, "recorded");
        assert!(observed_request.lock().unwrap().is_some());
    }

    #[tokio::test]
    async fn output_selection_save_writes_rendered_png_to_path() {
        let service = CaptureSessionService::new(Arc::new(make_backend_with_renderable_png()));
        let view = service.create_session().await.unwrap();
        let path = temp_png_path();

        let output = service
            .output_selection(
                &ImageCompositionService::new(),
                &CaptureOutputService::new(),
                &view.id,
                &LogicalRect {
                    x: 1.0,
                    y: 1.0,
                    width: 2.0,
                    height: 2.0,
                },
                &[],
                false,
                CaptureOutputAction::Save {
                    path: path.to_string_lossy().to_string(),
                },
            )
            .await
            .unwrap();

        let saved = std::fs::read(&path).unwrap();
        let decoded = image::load_from_memory(&saved).unwrap().to_rgba8();

        assert!(matches!(output, CaptureSessionOutput::Completed));
        assert_eq!((decoded.width(), decoded.height()), (2, 2));
        assert!(decoded.pixels().all(|pixel| pixel.0 == [10, 20, 30, 255]));

        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn render_png_base64_returns_rendered_selection_as_base64() {
        let service = CaptureSessionService::new(Arc::new(make_backend_with_renderable_png()));
        let view = service.create_session().await.unwrap();

        let encoded = service
            .render_png_base64(
                &ImageCompositionService::new(),
                &view.id,
                &LogicalRect {
                    x: 1.0,
                    y: 1.0,
                    width: 2.0,
                    height: 2.0,
                },
                &[],
                false,
            )
            .unwrap();

        let png_data = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .unwrap();
        let decoded = image::load_from_memory(&png_data).unwrap().to_rgba8();

        assert_eq!((decoded.width(), decoded.height()), (2, 2));
        assert!(decoded.pixels().all(|pixel| pixel.0 == [10, 20, 30, 255]));
    }

    #[tokio::test]
    async fn output_selection_pin_returns_rendered_png() {
        let service = CaptureSessionService::new(Arc::new(make_backend_with_renderable_png()));
        let view = service.create_session().await.unwrap();

        let output = service
            .output_selection(
                &ImageCompositionService::new(),
                &CaptureOutputService::new(),
                &view.id,
                &LogicalRect {
                    x: 1.0,
                    y: 1.0,
                    width: 2.0,
                    height: 2.0,
                },
                &[],
                false,
                CaptureOutputAction::Pin,
            )
            .await
            .unwrap();

        let CaptureSessionOutput::Pin(png_data) = output else {
            panic!("expected pin output");
        };
        let decoded = image::load_from_memory(&png_data).unwrap().to_rgba8();

        assert_eq!((decoded.width(), decoded.height()), (2, 2));
        assert!(decoded.pixels().all(|pixel| pixel.0 == [10, 20, 30, 255]));
    }
}

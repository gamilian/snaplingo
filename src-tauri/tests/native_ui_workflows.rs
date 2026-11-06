// Run the real native adapters on AppKit's main thread with an isolated database
// and synthetic capture/clipboard ports. The installed app and clipboard are untouched.
#![allow(dead_code, unused_imports, ambiguous_glob_reexports)]

#[cfg(target_os = "macos")]
#[path = "../src/app_actions.rs"]
mod app_actions;
#[cfg(target_os = "macos")]
#[path = "../src/app_shell.rs"]
mod app_shell;
#[cfg(target_os = "macos")]
#[path = "../src/app_state.rs"]
mod app_state;
#[cfg(target_os = "macos")]
#[path = "../src/application/mod.rs"]
mod application;
#[cfg(target_os = "macos")]
#[path = "../src/commands/mod.rs"]
mod commands;
#[cfg(target_os = "macos")]
mod composition {
    include!(concat!(env!("OUT_DIR"), "/native_ui_composition.rs"));
}
#[cfg(target_os = "macos")]
#[path = "../src/domain/mod.rs"]
mod domain;
#[cfg(target_os = "macos")]
#[path = "../src/error.rs"]
mod error;
#[cfg(target_os = "macos")]
#[path = "../src/infrastructure/mod.rs"]
mod infrastructure;
#[cfg(target_os = "macos")]
#[path = "../src/settings_window.rs"]
mod settings_window;
#[cfg(target_os = "macos")]
#[path = "../src/startup_shortcuts.rs"]
mod startup_shortcuts;
#[cfg(target_os = "macos")]
pub use app_state::AppState;
#[cfg(target_os = "macos")]
pub use application::*;
#[cfg(target_os = "macos")]
pub use domain::*;
#[cfg(target_os = "macos")]
pub use error::{AppError, Result};
#[cfg(target_os = "macos")]
pub use infrastructure::*;
#[cfg(target_os = "macos")]
#[path = "native_ui/capture_window.rs"]
mod capture_workflows;
#[cfg(target_os = "macos")]
#[path = "native_ui/result_window.rs"]
mod result_workflows;

#[cfg(target_os = "macos")]
mod fixture {
    use std::io::Cursor;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use async_trait::async_trait;
    use chrono::{DateTime, Local};
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSEvent, NSEventModifierFlags, NSEventType, NSView};
    use objc2_foundation::{NSPoint, NSString};
    use tauri::{Listener, Manager};

    use crate::application::capture::{
        CaptureImageComposer, CaptureOcrDelivery, CaptureOutputHost, CaptureOutputSystemPaths,
        CaptureSelectionMode, CaptureSessionRuntimeHost, CaptureWindowGeometry,
    };
    use crate::application::pinned_image::{
        PinnedImagePlacement, PinnedImageRuntimeHost, PinnedImageState,
    };
    use crate::domain::capture::*;
    use crate::infrastructure::system::capture_window::{
        NativeCaptureSessionRuntimeHost, TauriCaptureSessionRuntimeHost,
    };
    use crate::infrastructure::system::pinned_window::TauriPinnedImageRuntimeHost;
    use crate::infrastructure::system::scrolling_window::TauriScrollingCaptureHost;
    use crate::*;

    pub(super) fn png(width: u32, height: u32, offset: u32) -> Vec<u8> {
        let pixels = image::RgbaImage::from_fn(width, height, |x, y| {
            let hash = x.wrapping_mul(0x45d9f3b) ^ (y + offset).wrapping_mul(0x119de1f3);
            image::Rgba([(hash >> 16) as u8, (hash >> 8) as u8, hash as u8, 255])
        });
        let mut bytes = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(pixels)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        bytes.into_inner()
    }

    struct Source {
        frames: AtomicUsize,
    }
    impl Source {
        fn layout() -> MonitorLayout {
            MonitorLayout {
                id: "fixture".into(),
                logical_bounds: LogicalRect {
                    x: 0.0,
                    y: 0.0,
                    width: 1280.0,
                    height: 720.0,
                },
                physical_bounds: PhysicalRect {
                    x: 0,
                    y: 0,
                    width: 1280,
                    height: 720,
                },
                scale_factor: 1.0,
            }
        }
    }
    #[async_trait]
    impl CaptureSessionSource for Source {
        async fn capture_monitor_snapshots(&self) -> Result<Vec<MonitorSnapshot>> {
            let frame = self.frames.fetch_add(1, Ordering::SeqCst);
            Ok(vec![domain::capture::monitor_snapshot_from_layout(
                Self::layout(),
                png(1280, 720, frame as u32 * 16),
            )])
        }
        async fn capture_monitor_layouts(&self) -> Result<Vec<MonitorLayout>> {
            Ok(vec![Self::layout()])
        }
        async fn capture_window_candidates(
            &self,
            _: &[MonitorSnapshot],
        ) -> Result<Vec<WindowCandidate>> {
            Ok(vec![WindowCandidate {
                id: "fixture-window".into(),
                title: "Fixture Window".into(),
                app_name: "Fixture".into(),
                logical_bounds: LogicalRect {
                    x: 400.0,
                    y: 300.0,
                    width: 320.0,
                    height: 200.0,
                },
            }])
        }
        async fn capture_control_candidate(
            &self,
            point: &LogicalPoint,
            _: &[MonitorSnapshot],
        ) -> Result<Option<ControlCandidate>> {
            Ok(
                (point.x >= 400.0 && point.x < 720.0 && point.y >= 300.0 && point.y < 500.0).then(
                    || ControlCandidate {
                        id: "fixture-control".into(),
                        logical_bounds: LogicalRect {
                            x: 450.0,
                            y: 320.0,
                            width: 160.0,
                            height: 80.0,
                        },
                    },
                ),
            )
        }
        fn current_cursor_position(&self, _: &[MonitorSnapshot]) -> Result<Option<LogicalPoint>> {
            Ok(Some(LogicalPoint { x: 500.0, y: 350.0 }))
        }
        async fn capture_region(&self, region: ScreenRegion) -> Result<Vec<u8>> {
            let snapshot = self.capture_monitor_snapshot("fixture").await?;
            let image = image::load_from_memory(&snapshot.png_data)
                .unwrap()
                .crop_imm(
                    region.x as u32,
                    region.y as u32,
                    region.width,
                    region.height,
                );
            let mut bytes = Cursor::new(Vec::new());
            image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
            Ok(bytes.into_inner())
        }
    }

    struct CaptureHost {
        session: Mutex<String>,
        native: NativeCaptureSessionRuntimeHost,
        legacy: TauriCaptureSessionRuntimeHost,
        legacy_quick: bool,
    }
    #[async_trait]
    impl CaptureSessionRuntimeHost for CaptureHost {
        async fn begin_capture_presentation(&self) -> Result<()> {
            self.native.begin_capture_presentation().await
        }
        async fn end_capture_presentation(&self) -> Result<()> {
            self.native.end_capture_presentation().await
        }
        async fn prepare_capture_window_for_mode(&self, mode: &str) -> Result<()> {
            if CaptureSelectionMode::parse(mode).is_none() {
                return Ok(());
            }
            if self.legacy_quick {
                self.legacy.prepare_capture_window().await
            } else {
                self.native.prepare_capture_window_for_mode(mode).await
            }
        }
        async fn prepare_capture_window(&self) -> Result<()> {
            self.legacy.prepare_capture_window().await
        }
        async fn open_capture_editor_window(
            &self,
            selection: &crate::application::capture::CaptureEditorSelection,
            geometry: &CaptureWindowGeometry,
        ) -> Result<()> {
            self.native
                .open_capture_editor_window(selection, geometry)
                .await
        }
        async fn finish_capture_editor_transfer(&self, id: &CaptureSessionId) -> Result<()> {
            self.native.finish_capture_editor_transfer(id).await
        }
        async fn discard_capture_editor(&self) -> Result<()> {
            self.native.discard_capture_editor().await
        }
        async fn update_capture_editor_toolbar(
            &self,
            id: &CaptureSessionId,
            presentation: Option<&crate::application::capture::CaptureEditorToolbarPresentation>,
        ) -> Result<bool> {
            self.native
                .update_capture_editor_toolbar(id, presentation)
                .await
        }
        async fn capture_editor_owns_focus(&self, id: &CaptureSessionId) -> Result<bool> {
            self.native.capture_editor_owns_focus(id).await
        }
        fn capture_selection_history(&self, legacy: Vec<LogicalRect>) -> Result<Vec<LogicalRect>> {
            self.native.capture_selection_history(legacy)
        }
        fn remember_capture_selection(&self, rect: &LogicalRect) -> Result<()> {
            self.native.remember_capture_selection(rect)
        }
        fn clipboard_revision(&self) -> u64 {
            self.native.clipboard_revision()
        }
        fn reserve_capture_ocr_result(
            &self,
            target: crate::application::capture::CaptureOcrTarget,
        ) -> Result<u64> {
            self.native.reserve_capture_ocr_result(target)
        }
        fn set_capture_ocr_status(
            &self,
            status: Option<crate::application::capture::CaptureOcrStatus>,
        ) {
            self.native.set_capture_ocr_status(status);
        }
        async fn publish_capture_ocr(
            &self,
            target: crate::application::capture::CaptureOcrTarget,
            result_id: u64,
            result: crate::domain::ocr::OcrResult,
            preview: Option<String>,
        ) -> Result<()> {
            self.native
                .publish_capture_ocr(target, result_id, result, preview)
                .await
        }
        async fn report_capture_ocr_error(
            &self,
            target: crate::application::capture::CaptureOcrTarget,
            result_id: u64,
            error: &str,
        ) {
            self.native
                .report_capture_ocr_error(target, result_id, error)
                .await;
        }
        async fn request_capture_selection_pin(&self, id: &CaptureSessionId) -> Result<bool> {
            self.native.request_capture_selection_pin(id).await
        }
        async fn prepare_capture_window_for_reveal(
            &self,
            session: Option<&CaptureSessionId>,
            geometry: Option<&CaptureWindowGeometry>,
        ) -> Result<()> {
            self.legacy
                .prepare_capture_window_for_reveal(session, geometry)
                .await
        }
        async fn reveal_capture_window(
            &self,
            geometry: Option<&CaptureWindowGeometry>,
        ) -> Result<()> {
            self.legacy.reveal_capture_window(geometry).await
        }
        async fn hide_capture_window(&self) -> Result<()> {
            self.native.hide_capture_window().await
        }
        async fn destroy_inactive_capture_window(&self) -> Result<()> {
            self.native.destroy_inactive_capture_window().await
        }
        async fn restore_capture_snapshot_windows(&self, _: Vec<String>) -> Result<()> {
            Ok(())
        }
        async fn open_capture_window_for_session(
            &self,
            mode: &str,
            id: &str,
            geometry: &CaptureWindowGeometry,
        ) -> Result<()> {
            *self.session.lock().unwrap() = id.to_owned();
            if CaptureSelectionMode::parse(mode).is_none() {
                return Ok(());
            }
            if self.legacy_quick {
                self.legacy
                    .open_capture_window_for_session(mode, id, geometry)
                    .await
            } else {
                self.native
                    .open_capture_window_for_session(mode, id, geometry)
                    .await
            }
        }
    }

    struct Output {
        directory: PathBuf,
        clipboard: Mutex<Vec<u8>>,
    }
    struct FixtureCursor;
    impl CaptureCursorMover for FixtureCursor {
        fn move_relative(&self, _: i32, _: i32) {}
    }
    impl CaptureOutputHost for Output {
        fn system_paths(&self) -> CaptureOutputSystemPaths {
            CaptureOutputSystemPaths {
                download_dir: Some(self.directory.clone()),
                picture_dir: Some(self.directory.clone()),
                home_dir: Some(self.directory.clone()),
                temp_dir: self.directory.clone(),
            }
        }
        fn now(&self) -> DateTime<Local> {
            Local::now()
        }
        fn path_exists(&self, path: &Path) -> bool {
            path.exists()
        }
        fn write_file(&self, path: &Path, bytes: &[u8]) -> Result<()> {
            assert!(
                path.starts_with(&self.directory),
                "Fixture output must stay in its directory"
            );
            std::fs::write(path, bytes).map_err(|error| error.to_string().into())
        }
        fn copy_png(&self, bytes: &[u8]) -> Result<()> {
            *self.clipboard.lock().unwrap() = bytes.to_vec();
            Ok(())
        }
        fn read_clipboard_png(&self) -> Result<Vec<u8>> {
            Ok(self.clipboard.lock().unwrap().clone())
        }
        fn read_clipboard_text(&self) -> Result<String> {
            Err("Fixture has an image".into())
        }
    }

    pub(super) async fn on_main<T: Send + 'static>(
        app: &tauri::AppHandle,
        operation: impl FnOnce() -> T + Send + 'static,
    ) -> T {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        app.run_on_main_thread(move || {
            let _ = sender.send(operation());
        })
        .unwrap();
        receiver.await.unwrap()
    }

    async fn pin_key(
        app: &tauri::AppHandle,
        text: &'static str,
        flags: NSEventModifierFlags,
        code: u16,
    ) {
        on_main(app, move || {
            let mtm = MainThreadMarker::new().unwrap();
            let windows = NSApplication::sharedApplication(mtm).windows();
            let window = windows.iter().find(|window| window.isVisible() && window.title().to_string() == "SnapLingo Pin").expect("Native pin is visible");
            let event = NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(
                NSEventType::KeyDown, NSPoint::new(0.0, 0.0), flags, 0.0, window.windowNumber(), None, &NSString::from_str(text), &NSString::from_str(text), false, code,
            ).unwrap();
            window.contentView().unwrap().keyDown(&event);
        }).await;
    }

    async fn pin_windows(app: &tauri::AppHandle) -> usize {
        on_main(app, || {
            NSApplication::sharedApplication(MainThreadMarker::new().unwrap())
                .windows()
                .iter()
                .filter(|window| window.title().to_string() == "SnapLingo Pin")
                .count()
        })
        .await
    }

    async fn workflow(
        app: tauri::AppHandle,
        source: Arc<Source>,
        capture_host: Arc<CaptureHost>,
        pin_state: Arc<PinnedImageState>,
        pin_host: Arc<dyn PinnedImageRuntimeHost>,
        output: Arc<Output>,
        result_services: Arc<crate::result_workflows::Services>,
    ) {
        let state = app.state::<AppState>();
        let capture = state.capture.runtime.clone();
        capture
            .open_capture_window_for_mode("fixture-scrolling")
            .await
            .unwrap();
        let session = CaptureSessionId(capture_host.session.lock().unwrap().clone());
        let initial = capture
            .begin_scrolling_capture(
                session,
                LogicalRect {
                    x: 400.0,
                    y: 300.0,
                    width: 320.0,
                    height: 200.0,
                },
            )
            .await
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(12);
        loop {
            let view = capture.scrolling_capture_view(&initial.id).await.unwrap();
            if view.height > initial.height {
                assert!(view.can_undo);
                break;
            }
            assert!(
                Instant::now() < deadline,
                "Native sampler did not append a frame"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert!(
            app.webview_windows().is_empty(),
            "Scrolling controls must not create a WebView"
        );
        capture.undo_scrolling_capture(&initial.id).await.unwrap();
        assert!(!capture
            .set_scrolling_preview_visible(&initial.id, false)
            .await
            .unwrap());
        assert!(capture
            .set_scrolling_preview_visible(&initial.id, true)
            .await
            .unwrap());
        let path = output.directory.join("longshot.png");
        capture
            .finish_scrolling_capture(
                &initial.id,
                ScrollingCaptureOutputAction::Save {
                    path: path.to_string_lossy().into_owned(),
                    format: "png".into(),
                    quality: 90,
                    copy_after_save: false,
                    remember_choice: true,
                },
            )
            .await
            .unwrap();
        assert!(image::open(&path).unwrap().height() >= initial.height);
        let frames = source.frames.load(Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(600)).await;
        assert_eq!(
            source.frames.load(Ordering::SeqCst),
            frames,
            "Sampler continued after closing"
        );

        let first = pin_state.pin_png_view(png(300, 100, 0)).unwrap();
        pin_host
            .open(first.clone(), PinnedImagePlacement::default())
            .await
            .unwrap();
        assert!(
            app.webview_windows().is_empty(),
            "Pin must not create a WebView"
        );
        pin_key(&app, "=", NSEventModifierFlags::empty(), 24).await;
        pin_key(&app, "c", NSEventModifierFlags::Command, 8).await;
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let copied = image::load_from_memory(&output.clipboard.lock().unwrap()).unwrap();
            if copied.width() == 330 && copied.height() == 110 {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "Native copy did not export the current size"
            );
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
        *output.clipboard.lock().unwrap() = png(200, 100, 10);
        pin_key(&app, "v", NSEventModifierFlags::Command, 9).await;
        let deadline = Instant::now() + Duration::from_secs(5);
        while state.capture.pinned_images.get(&first.id).unwrap().width != 200 {
            assert!(
                Instant::now() < deadline,
                "Native paste did not replace the image"
            );
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
        pin_key(&app, "b", NSEventModifierFlags::empty(), 11).await;
        let deadline = Instant::now() + Duration::from_secs(5);
        while !state
            .settings
            .configuration
            .snapshot()
            .unwrap()
            .screenshot
            .pin_border
        {
            assert!(
                Instant::now() < deadline,
                "Native border key did not update settings"
            );
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
        let second = pin_state.pin_png_view(png(180, 80, 0)).unwrap();
        pin_host
            .open(second.clone(), PinnedImagePlacement::default())
            .await
            .unwrap();
        state.capture.pinned_images.close(&first.id).await.unwrap();
        state.capture.pinned_images.close(&second.id).await.unwrap();
        assert!(
            pin_state.get_pinned_image(&first.id).is_ok(),
            "Group restore must keep older pins"
        );
        assert_eq!(pin_windows(&app).await, 2);
        assert_eq!(
            on_main(&app, || {
                infrastructure::system::pinned_window::test_decoded_images()
            })
            .await,
            0
        );
        state.capture.pinned_images.pin_clipboard().await.unwrap();
        state
            .capture
            .pinned_images
            .move_to_next_group(&second.id)
            .await
            .unwrap();
        state.capture.pinned_images.switch_group().await.unwrap();
        state
            .capture
            .pinned_images
            .destroy_group(&second.id)
            .await
            .unwrap();
        state.capture.pinned_images.close_all().await.unwrap();
        assert_eq!(pin_windows(&app).await, 0);
        crate::result_workflows::run(&app, result_services).await;
        crate::capture_workflows::run(&app, &output.directory, &output.clipboard).await;
        crate::capture_workflows::editor_handoff(&app, &output.clipboard).await;
        println!("Native scrolling sampler, output, keyboard actions, replacement, settings, groups and resource cleanup passed");
    }

    fn value(args: &[String], name: &str, fallback: &str) -> String {
        args.iter()
            .position(|arg| arg == name)
            .and_then(|index| args.get(index + 1))
            .map(String::as_str)
            .unwrap_or(fallback)
            .to_owned()
    }

    pub fn run() {
        let args = std::env::args().collect::<Vec<_>>();
        let benchmark = args.iter().any(|arg| arg == "--benchmark");
        let renderer = value(&args, "--benchmark", "native");
        assert!(matches!(renderer.as_str(), "native" | "webview"));
        let surface = value(&args, "--surface", "pin");
        assert!(matches!(
            surface.as_str(),
            "pin" | "result" | "capture" | "editor"
        ));
        let pins = value(&args, "--pins", "1").parse::<usize>().unwrap();
        let seconds = value(&args, "--seconds", "20").parse::<u64>().unwrap();
        assert!((1..=20).contains(&pins));
        let mut context = tauri::generate_context!();
        context.config_mut().identifier =
            format!("com.snaplingo.native-ui-test-{}", std::process::id());
        #[cfg(debug_assertions)]
        if benchmark {
            context.config_mut().build.dev_url =
                Some(tauri::Url::parse("http://127.0.0.1:5189").unwrap());
        }
        tauri::Builder::default()
            .manage(HotkeyRecording::default())
            .register_uri_scheme_protocol("capture-image", infrastructure::system::capture_window::image_protocol::respond)
            .plugin(tauri_plugin_global_shortcut::Builder::new().build())
            .plugin(tauri_plugin_dialog::init())
            .invoke_handler(tauri::generate_handler![
                commands::get_settings_snapshot,
                commands::update_annotation_colors,
                commands::update_screenshot_settings,
                commands::get_pinned_image,
                commands::list_translation_providers,
                commands::translate_text_with_provider,
                commands::record_translation_history,
                commands::current_capture_result_window_request_id,
                commands::take_capture_result_window_payload,
                commands::get_capture_session,
                commands::create_capture_session,
                commands::hydrate_capture_session_snapshots,
                commands::hydrate_capture_monitor_snapshot,
                commands::log_capture_frontend_perf,
                commands::current_capture_cursor_position,
                commands::current_capture_control_candidate,
                commands::prepare_capture_window_for_reveal,
                commands::reveal_capture_window,
                commands::hide_capture_window,
                commands::cancel_capture_session,
                commands::list_annotation_font_families,
                commands::last_annotation_tool,
                commands::capture_selection_history,
                commands::capture_editor_selection,
                commands::update_capture_editor_toolbar,
                commands::capture_editor_owns_focus,
                commands::remember_capture_selection,
                commands::remember_annotation_tool,
                commands::move_capture_cursor,
                commands::restore_capture_snapshot_windows_for_session,
                commands::render_capture_output,
                commands::output_capture,
                commands::begin_scrolling_capture,
                commands::prepare_capture_ocr,
                commands::complete_capture_ocr,
            ])
            .setup(move |app| {
                let directory = tempfile::tempdir().unwrap();
                let path = directory.path().join("fixture.db");
                let database = Arc::new(infrastructure::storage::Database::open(&path).unwrap());
                let mut state = composition::build_app_state(database, path, app.handle().clone());
                let result_services = crate::result_workflows::configure(&mut state);
                let capture_services = crate::capture_workflows::configure(&mut state);
                if benchmark && surface == "result" { crate::result_workflows::prepare_benchmark(app.handle(), &mut state, &renderer); }
                let mut screenshot = state.settings.configuration.snapshot().unwrap().screenshot;
                screenshot.save_path = directory.path().to_string_lossy().into_owned();
                screenshot.pin_border = false;
                state.settings.configuration.update_screenshot(screenshot).unwrap();
                let source = Arc::new(Source { frames: AtomicUsize::new(0) });
                let delivery = Arc::new(CaptureOcrDelivery::new(state.result_window.clone(), capture_services.clone()));
                let capture_host = Arc::new(CaptureHost {
                    session: Mutex::new(String::new()),
                    native: NativeCaptureSessionRuntimeHost::new(app.handle().clone(), delivery.clone()),
                    legacy: TauriCaptureSessionRuntimeHost::new(app.handle().clone(), delivery),
                    legacy_quick: benchmark && surface == "capture" && renderer == "webview",
                });
                let output = Arc::new(Output { directory: directory.path().to_owned(), clipboard: Mutex::new(png(200, 100, 0)) });
                let composer = Arc::new(CaptureImageComposer::with_font_provider(Arc::new(
                    infrastructure::system::annotation_fonts::SystemAnnotationFonts::default(),
                )));
                let capture_output = Arc::new(CaptureOutput::with_host(output.clone()));
                let sessions = Arc::new(CaptureSessions::new(source.clone()));
                let pin_state = Arc::new(PinnedImageState::new());
                let pin_host: Arc<dyn PinnedImageRuntimeHost> = Arc::new(TauriPinnedImageRuntimeHost::new(app.handle().clone()));
                let pinned = Arc::new(PinnedImageRuntime::new(pin_state.clone(), composer.clone(), capture_output.clone(), pin_host.clone()).with_settings(state.settings.configuration.clone()));
                let runtime = Arc::new(CaptureSessionRuntime::with_host(sessions.clone(), composer, capture_output.clone(), state.providers.ocr.clone(), capture_host.clone(), pinned.clone())
                    .with_scrolling_host(Arc::new(TauriScrollingCaptureHost::new(app.handle().clone(), source.clone())))
                    .with_settings(state.settings.configuration.clone()));
                state.capture = Arc::new(app_state::CaptureRuntimeState {
                    sessions, output: capture_output, runtime, pinned_images: pinned, cursor: Arc::new(FixtureCursor),
                });
                app.manage(state);
                app.manage(capture_services);
                let handle = app.handle().clone();
                if benchmark {
                    app.listen("native-ui-benchmark-loaded", |event| println!("{}", event.payload()));
                    if surface == "result" {
                        crate::result_workflows::start_benchmark(app.handle(), &renderer);
                    } else if surface == "capture" {
                        crate::capture_workflows::start_benchmark(app.handle(), &renderer);
                    } else { let bytes = png(800, 600, 0); for index in 0..pins {
                        let image = pin_state.pin_png_view(bytes.clone()).unwrap();
                        if renderer == "native" {
                            infrastructure::system::pinned_window::open_pinned_image_window(app.handle(), &image,
                                PinnedImagePlacement { origin: Some(DesktopPosition::Logical(LogicalPoint { x: 180.0 + index as f64 * 20.0, y: 120.0 + index as f64 * 20.0 })), centered: false }).unwrap();
                        } else {
                            assert_eq!(renderer, "webview");
                            tauri::WebviewWindowBuilder::new(app, format!("pin-{}", image.id), tauri::WebviewUrl::App(format!("index.html?window=pin&imageId={}", image.id).into()))
                                .title("SnapLingo UI benchmark").inner_size(800.0, 600.0).position(180.0 + index as f64 * 20.0, 120.0 + index as f64 * 20.0)
                                .decorations(false).transparent(true).always_on_top(true).build().unwrap();
                        }
                    }
                    }
                    println!("{}", serde_json::json!({ "mode": renderer, "surface": surface, "pins": pins, "pid": std::process::id(), "webviews": app.webview_windows().len(), "seconds": seconds }));
                    std::thread::spawn(move || {
                        std::thread::sleep(Duration::from_secs(3));
                        for window in handle.webview_windows().values() {
                            window.eval(if surface == "result" {
                                "window.__TAURI__.event.emit('native-ui-benchmark-loaded', {sourceText:document.querySelector('textarea')?.value??'',resultText:document.querySelector('.translation-result-copy')?.textContent?.trim()??'',resultCards:document.querySelectorAll('.translation-result-card').length})"
                            } else if surface == "capture" {
                                "const monitor = document.querySelector('[data-capture-monitor]'); monitor?.parentElement?.parentElement?.dispatchEvent(new PointerEvent('pointermove', {bubbles:true,clientX:500,clientY:350,pointerId:1,pointerType:'mouse'})); window.__TAURI__.event.emit('native-ui-benchmark-loaded', {captureWidth:monitor?.naturalWidth??0,captureHeight:monitor?.naturalHeight??0,selecting:!!document.querySelector('[aria-label=\"选区快捷键提示\"]'),loading:!!document.querySelector('[aria-label=\"Loading capture\"]'),webviews:1})"
                            } else {
                                "window.__TAURI__.event.emit('native-ui-benchmark-loaded', {imageWidth:document.querySelector('img')?.naturalWidth??0,toolbarButtons:document.querySelectorAll('.pin-toolbar button').length})"
                            }).unwrap();
                        }
                        std::thread::sleep(Duration::from_secs(seconds.saturating_sub(3)));
                        handle.exit(0);
                        drop(directory);
                    });
                } else {
                    std::thread::spawn(move || {
                        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            if surface == "editor" {
                                tauri::async_runtime::block_on(crate::capture_workflows::editor_handoff(&handle, &output.clipboard));
                            } else if surface == "capture" {
                                tauri::async_runtime::block_on(crate::capture_workflows::run(&handle, &output.directory, &output.clipboard));
                            } else {
                                tauri::async_runtime::block_on(workflow(handle.clone(), source, capture_host, pin_state, pin_host, output, result_services));
                            }
                        }));
                        if result.is_err() {
                            drop(directory);
                            std::process::exit(1);
                        }
                        handle.exit(0);
                        drop(directory);
                    });
                }
                Ok(())
            })
            .build(context).expect("Native UI workflow app failed")
            .run(|_, event| {
                if let tauri::RunEvent::ExitRequested { code, api, .. } = event {
                    if app_shell::should_prevent_implicit_exit(code) {
                        api.prevent_exit();
                    }
                }
            });
    }
}

#[cfg(target_os = "macos")]
fn main() {
    fixture::run();
}

#[cfg(not(target_os = "macos"))]
fn main() {}

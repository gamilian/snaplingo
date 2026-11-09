// Run the real native adapters on AppKit's main thread with an isolated database
// and synthetic capture/clipboard ports. The installed app and clipboard are untouched.
#![allow(dead_code, unused_imports, ambiguous_glob_reexports)]

#[cfg(target_os = "macos")]
#[path = "../app_actions.rs"]
mod app_actions;
#[cfg(target_os = "macos")]
#[path = "../app_shell.rs"]
mod app_shell;
#[cfg(target_os = "macos")]
#[path = "../app_state.rs"]
mod app_state;
#[cfg(target_os = "macos")]
use snaplingo_core::application;
#[cfg(target_os = "macos")]
mod composition {
    include!(concat!(env!("OUT_DIR"), "/native_ui_composition.rs"));
}
#[cfg(target_os = "macos")]
use snaplingo_core::domain;
#[cfg(target_os = "macos")]
use snaplingo_core::error;
#[cfg(target_os = "macos")]
#[path = "../infrastructure/mod.rs"]
mod infrastructure;
#[cfg(target_os = "macos")]
#[path = "../startup_shortcuts.rs"]
mod startup_shortcuts;
#[cfg(target_os = "macos")]
#[path = "../window/mod.rs"]
mod window;
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
#[path = "../native_app.rs"]
mod native_app;
#[cfg(target_os = "macos")]
#[path = "../native_capacity.rs"]
mod native_capacity;
#[cfg(target_os = "macos")]
#[path = "../native_permission_prompt.rs"]
mod native_permission_prompt;
#[cfg(target_os = "macos")]
#[path = "../native_settings_window.rs"]
mod native_settings_window;
#[cfg(target_os = "macos")]
#[path = "../runtime_lifecycle.rs"]
mod runtime_lifecycle;
#[cfg(target_os = "macos")]
pub use runtime_lifecycle::{should_show_settings_on_startup, RuntimeLifecycle};

#[cfg(target_os = "macos")]
mod fixture {
    mod carbon {
        include!("native_ui/carbon_hotkey.rs");
    }
    use std::io::Cursor;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use async_trait::async_trait;
    use chrono::{DateTime, Local};
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSEvent, NSEventModifierFlags, NSEventType, NSView};
    use objc2_foundation::{NSPoint, NSString};

    use crate::application::capture::{
        CaptureImageComposer, CaptureOcrDelivery, CaptureOutputHost, CaptureOutputSystemPaths,
        CaptureSelectionMode, CaptureSessionRuntimeHost, CaptureWindowGeometry,
    };
    use crate::application::pinned_image::{
        PinnedImagePlacement, PinnedImageRuntimeHost, PinnedImageState,
    };
    use crate::domain::capture::*;
    use crate::infrastructure::system::capture_window::NativeCaptureSessionRuntimeHost;
    use crate::window::scrolling::macos::NativeScrollingCaptureHost;
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
        fail_scrolling_open: AtomicBool,
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

    struct ScrollingHost {
        native: Arc<NativeScrollingCaptureHost>,
        source: Arc<Source>,
    }
    #[async_trait]
    impl crate::application::capture::ScrollingCaptureHost for ScrollingHost {
        async fn open(
            &self,
            view: &ScrollingCaptureView,
            geometry: &crate::application::capture::ScrollingCaptureGeometry,
        ) -> Result<crate::application::capture::ScrollingWindowOptions> {
            if self
                .source
                .fail_scrolling_open
                .swap(false, Ordering::SeqCst)
            {
                return Err("Synthetic scrolling window open failure".into());
            }
            self.native.open(view, geometry).await
        }
        async fn capture_snapshots(
            &self,
            id: &str,
            geometry: &crate::application::capture::ScrollingCaptureGeometry,
        ) -> Result<Vec<MonitorSnapshot>> {
            self.native.capture_snapshots(id, geometry).await
        }
        async fn close(
            &self,
            id: &str,
            session: &CaptureSessionId,
            completed: Option<bool>,
        ) -> Result<()> {
            self.native.close(id, session, completed).await
        }
        fn publish(&self, view: &ScrollingCaptureView) {
            self.native.publish(view);
        }
        async fn set_preview_visible(&self, id: &str, visible: bool) -> Result<bool> {
            self.native.set_preview_visible(id, visible).await
        }
    }

    struct CaptureHost {
        session: Mutex<String>,
        native: NativeCaptureSessionRuntimeHost,
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

            self.native.prepare_capture_window_for_mode(mode).await
        }
        async fn prepare_capture_window(&self) -> Result<()> {
            self.native.prepare_capture_window().await
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
            self.native
                .prepare_capture_window_for_reveal(session, geometry)
                .await
        }
        async fn reveal_capture_window(
            &self,
            geometry: Option<&CaptureWindowGeometry>,
        ) -> Result<()> {
            self.native.reveal_capture_window(geometry).await
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

            self.native
                .open_capture_window_for_session(mode, id, geometry)
                .await
        }
    }

    struct Output {
        directory: PathBuf,
        clipboard: Mutex<Vec<u8>>,
        fail_copy: AtomicBool,
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
            if self.fail_copy.swap(false, Ordering::SeqCst) {
                return Err("Synthetic native editor copy failure".into());
            }
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
        _app: &NativeTestApp,
        operation: impl FnOnce() -> T + Send + 'static,
    ) -> T {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        crate::infrastructure::ui_thread::dispatch(move || {
            let _ = sender.send(std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                operation,
            )));
        });
        match receiver.await.unwrap() {
            Ok(value) => value,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }

    async fn pin_key(
        app: &NativeTestApp,
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

    async fn pin_windows(app: &NativeTestApp) -> usize {
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
        app: NativeTestApp,
        source: Arc<Source>,
        capture_host: Arc<CaptureHost>,
        pin_state: Arc<PinnedImageState>,
        pin_host: Arc<dyn PinnedImageRuntimeHost>,
        output: Arc<Output>,
        result_services: Arc<crate::result_workflows::Services>,
    ) {
        test_notifications(&app);
        test_hotkey_event_route(&app).await;
        let state = app.state.clone();
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
            on_main(&app, || { window::pinned::test_decoded_images() }).await,
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
        crate::capture_workflows::editor_output_recovery(
            &app,
            &output.clipboard,
            &output.fail_copy,
            &source.fail_scrolling_open,
        )
        .await;
        println!("Native scrolling sampler, output, keyboard actions, replacement, settings, groups and resource cleanup passed");
    }

    async fn test_hotkey_event_route(app: &NativeTestApp) {
        use crate::application::hotkeys::{
            HotkeyRegistrar, HotkeyRegistration, HotkeyTriggerTiming,
        };
        use crate::domain::hotkey_config::{SHOW_TRANSLATION_WINDOW_ACTION, TRANSLATION_CATEGORY};
        use crate::infrastructure::desktop_events::DesktopEvent;
        let state = app.state.clone();
        // Frontend bridge replacement must not register another action route.

        let events = state.desktop_events.clone();
        let registrar =
            infrastructure::system::NativeHotkeyRegistrar::new(move |category, action| {
                events.publish(DesktopEvent::HotkeyTriggered { category, action })
            });
        let actions = Arc::new(AtomicUsize::new(0));
        let menu_actions = Arc::new(AtomicUsize::new(0));
        let observed = actions.clone();
        let observed_menu = menu_actions.clone();
        let _subscription = state.desktop_events.subscribe(move |event| {
            if matches!(event, DesktopEvent::HotkeyTriggered { category, action } if category == TRANSLATION_CATEGORY && action == SHOW_TRANSLATION_WINDOW_ACTION) { observed.fetch_add(1, Ordering::SeqCst); }
            if *event == DesktopEvent::AppActionRequested(crate::app_actions::AppAction::OpenTranslationWindow) { observed_menu.fetch_add(1, Ordering::SeqCst); }
        });
        let mut selected = None;
        for key in ["F18", "F19", "F20"] {
            if registrar
                .register(HotkeyRegistration {
                    category: TRANSLATION_CATEGORY.into(),
                    action: SHOW_TRANSLATION_WINDOW_ACTION.into(),
                    accelerator: key.into(),
                    timing: HotkeyTriggerTiming::Pressed,
                })
                .is_ok()
            {
                selected = Some(key);
                break;
            }
        }
        let key = selected.expect("a free test function key");
        let request_before = state.result_window.reserve_request().unwrap();
        on_main(app, move || {
            let key = key.parse().unwrap();
            carbon::send_hotkey(&key, false);
            carbon::send_hotkey(&key, true);
        })
        .await;
        assert_eq!(
            actions.load(Ordering::SeqCst),
            1,
            "Native press timing routed a released key twice"
        );
        let deadline = Instant::now() + Duration::from_secs(8);
        while !state.result_session.snapshot().visible {
            assert!(
                Instant::now() < deadline,
                "Native hotkey did not reach the result-window action"
            );
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
        assert!(
            state
                .result_window
                .is_current(crate::application::result_window::ResultWindowRequestId(
                    request_before.0 + 1
                ))
                .unwrap(),
            "A duplicate desktop observer dispatched the same hotkey action more than once"
        );
        registrar.unregister(key).unwrap();

        state.result_session.close();
        on_main(
            app,
            infrastructure::system::result_window::native_view::test_destroy,
        )
        .await;
        let request_before = state.result_window.reserve_request().unwrap();
        on_main(app, app_shell::test_show_translation_menu_action).await;
        assert_eq!(menu_actions.load(Ordering::SeqCst), 1);
        let deadline = Instant::now() + Duration::from_secs(8);
        while !state.result_session.snapshot().visible {
            assert!(
                Instant::now() < deadline,
                "Native menu did not reach the result-window action"
            );
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
        assert!(
            state
                .result_window
                .is_current(crate::application::result_window::ResultWindowRequestId(
                    request_before.0 + 1
                ))
                .unwrap(),
            "Native menu action did not dispatch exactly once"
        );
        state.result_session.close();
        on_main(
            app,
            infrastructure::system::result_window::native_view::test_destroy,
        )
        .await;
        println!("Real Carbon hotkey and AppKit menu each routed once through Rust desktop events and application actions to the native result window");
    }

    fn test_notifications(app: &NativeTestApp) {
        use crate::infrastructure::desktop_events::DesktopEvent;
        let state = &app.state;
        let settings = state.settings.configuration.clone();
        let original = settings.snapshot().unwrap().general;
        let expected = if original.log_retention_days == 7 {
            8
        } else {
            7
        };
        let committed = Arc::new(AtomicBool::new(false));
        let received = Arc::new(Mutex::new(vec![0usize; 6]));
        let kinds = [
            DesktopEvent::SettingsChanged,
            DesktopEvent::HotkeysChanged,
            DesktopEvent::ProvidersChanged,
            DesktopEvent::HistoryChanged,
            DesktopEvent::FavoritesChanged,
            DesktopEvent::ScreenshotFavoritesChanged,
        ];
        let counts = received.clone();
        let observed = settings.clone();
        let commit = committed.clone();
        let matching = kinds.clone();
        let subscription = state.desktop_events.subscribe(move |event| {
            if let Some(index) = matching.iter().position(|k| k == event) {
                counts.lock().unwrap()[index] += 1;
                if index == 0 && observed.snapshot().unwrap().general.log_retention_days == expected
                {
                    commit.store(true, Ordering::SeqCst);
                }
            }
        });
        let recordings = Arc::new(Mutex::new(vec![]));
        let output = recordings.clone();
        let recording_subscription = state.desktop_events.subscribe(move |event| {
            if let DesktopEvent::HotkeyRecorded {
                window_label,
                recording_id,
                hotkey,
            } = event
            {
                output.lock().unwrap().push((
                    window_label.clone(),
                    recording_id.clone(),
                    hotkey.clone(),
                ));
            }
        });
        let recording = DesktopEvent::HotkeyRecorded {
            window_label: "settings".into(),
            recording_id: "native-recording".into(),
            hotkey: "⌘F20".into(),
        };
        state.desktop_events.publish(recording.clone());
        assert_eq!(
            *recordings.lock().unwrap(),
            vec![("settings".into(), "native-recording".into(), "⌘F20".into())]
        );
        let mut general = original.clone();
        general.log_retention_days = expected;
        settings.update_general(general).unwrap();
        for event in kinds.iter().skip(1) {
            state.desktop_events.publish(event.clone());
        }
        assert_eq!(*received.lock().unwrap(), vec![1; 6]);
        assert!(
            committed.load(Ordering::SeqCst),
            "UI notification arrived before settings were committed"
        );
        drop(subscription);
        drop(recording_subscription);
        state.desktop_events.publish(recording);
        for event in kinds {
            state.desktop_events.publish(event);
        }
        assert_eq!(
            *received.lock().unwrap(),
            vec![1; 6],
            "Disposed native subscription delivered another notification"
        );
        assert_eq!(recordings.lock().unwrap().len(), 1);
        settings.update_general(original).unwrap();
        println!("Rust desktop events preserved all six notifications, recording payloads, committed settings visibility and subscription disposal");
    }

    fn value(args: &[String], name: &str, fallback: &str) -> String {
        args.iter()
            .position(|arg| arg == name)
            .and_then(|index| args.get(index + 1))
            .map(String::as_str)
            .unwrap_or(fallback)
            .to_owned()
    }

    #[derive(Clone)]
    pub(crate) struct NativeTestApp {
        pub state: Arc<AppState>,
        pub capture_services: Arc<crate::capture_workflows::Services>,
        pub recording: HotkeyRecording,
    }
    async fn benchmark(
        app: &NativeTestApp,
        surface: &str,
        pins: usize,
        seconds: u64,
        pin_state: &PinnedImageState,
        pin_host: &dyn PinnedImageRuntimeHost,
    ) {
        match surface {
            "capture" => {
                app.state
                    .capture
                    .runtime
                    .open_capture_window_for_mode("screenshot-copy")
                    .await
                    .unwrap();
                let deadline = Instant::now() + Duration::from_secs(8);
                let (width, height, visible) = loop {
                    let size = on_main(app, || {
                        use infrastructure::system::capture_window::native_view as native;
                        native::session_id().map(|id| native::test_ready_size(&id))
                    })
                    .await;
                    if let Some(size) = size.filter(|s| s.2) {
                        break size;
                    }
                    assert!(
                        Instant::now() < deadline,
                        "Native benchmark capture did not load"
                    );
                    tokio::time::sleep(Duration::from_millis(20)).await;
                };
                println!(
                    "{}",
                    serde_json::json!({"captureWidth":width,"captureHeight":height,"selecting":visible,"loading":false})
                );
            }
            "result" => {
                app.state.result_window.open(crate::application::result_window::ResultWindowOpenRequest::selection_translation("native result benchmark".into())).await.unwrap();
                let deadline = Instant::now() + Duration::from_secs(8);
                while app
                    .state
                    .result_session
                    .snapshot()
                    .rows
                    .first()
                    .is_none_or(|row| {
                        row.status != crate::application::result_window::TranslationStatus::Success
                    })
                {
                    assert!(
                        Instant::now() < deadline,
                        "Native result benchmark did not finish"
                    );
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
                let view = app.state.result_session.snapshot();
                let row = view.rows.first().expect("Native result benchmark row");
                println!(
                    "{}",
                    serde_json::json!({"sourceText":view.source_text,"resultText":row.result.translated_text,"resultCards":view.rows.len()})
                );
            }
            _ => {
                for index in 0..pins {
                    let image = pin_state.pin_png_view(png(800, 600, 0)).unwrap();
                    pin_host
                        .open(
                            image,
                            PinnedImagePlacement {
                                origin: Some(DesktopPosition::Logical(LogicalPoint {
                                    x: 180. + index as f64 * 20.,
                                    y: 120. + index as f64 * 20.,
                                })),
                                centered: false,
                            },
                        )
                        .await
                        .unwrap();
                }
            }
        }
        println!(
            "{}",
            serde_json::json!({"mode":"native","surface":surface,"pins":pins,"pid":std::process::id(),"seconds":seconds})
        );
        tokio::time::sleep(Duration::from_secs(seconds)).await;
    }
    pub fn run() {
        let args = std::env::args().collect::<Vec<_>>();
        let surface = value(&args, "--surface", "pin");
        let measuring = args.iter().any(|arg| arg == "--benchmark");
        if measuring {
            assert_eq!(
                value(&args, "--benchmark", "native"),
                "native",
                "Only the native application is benchmarked"
            );
        }
        let pins = value(&args, "--pins", "1").parse::<usize>().unwrap();
        assert!((1..=20).contains(&pins));
        let seconds = value(&args, "--seconds", "14").parse::<u64>().unwrap();
        assert!(matches!(
            surface.as_str(),
            "pin" | "result" | "capture" | "editor"
        ));
        let mtm = MainThreadMarker::new().expect("Native workflows run on the process main thread");
        let app = NSApplication::sharedApplication(mtm);
        app.setActivationPolicy(objc2_app_kit::NSApplicationActivationPolicy::Regular);
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("fixture.db");
        let database = Arc::new(infrastructure::storage::Database::open(&path).unwrap());
        let provider = Arc::new(
            infrastructure::system::selection::macos::MacSelectionProvider::new(Some(
                "com.snaplingo.native-ui-tests".into(),
            )),
        );
        let mut state = composition::build_app_state(
            database,
            path,
            composition::CompositionOptions {
                identifier: "com.snaplingo.native-ui-tests".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                ocr_resource_root: None,
                cache_dir: Ok(directory.path().join("cache")),
                selected_text: Arc::new(SelectedTextAcquirer::new(
                    SelectionScheme::new(provider.default_scheme()),
                    provider.methods(),
                    provider,
                )),
            },
        );
        let result_services = crate::result_workflows::configure(&mut state);
        let capture_services = crate::capture_workflows::configure(&mut state);
        let mut screenshot = state.settings.configuration.snapshot().unwrap().screenshot;
        screenshot.save_path = directory.path().to_string_lossy().into_owned();
        screenshot.pin_border = false;
        state
            .settings
            .configuration
            .update_screenshot(screenshot)
            .unwrap();
        let source = Arc::new(Source {
            frames: AtomicUsize::new(0),
            fail_scrolling_open: AtomicBool::new(false),
        });
        let delivery = Arc::new(CaptureOcrDelivery::new(
            state.result_window.clone(),
            capture_services.clone(),
        ));
        let capture_host = Arc::new(CaptureHost {
            session: Mutex::new(String::new()),
            native: NativeCaptureSessionRuntimeHost::new(
                "com.snaplingo.native-ui-tests".into(),
                delivery,
                app_shell::set_capture_ocr_status_native,
            ),
        });
        let output = Arc::new(Output {
            directory: directory.path().into(),
            clipboard: Mutex::new(png(200, 100, 0)),
            fail_copy: AtomicBool::new(false),
        });
        let composer = Arc::new(CaptureImageComposer::with_font_provider(Arc::new(
            infrastructure::system::annotation_fonts::SystemAnnotationFonts::default(),
        )));
        let capture_output = Arc::new(CaptureOutput::with_host(output.clone()));
        let sessions = Arc::new(CaptureSessions::new(source.clone()));
        let pin_state = Arc::new(PinnedImageState::new());
        let pin_host: Arc<dyn PinnedImageRuntimeHost> = state.native_pinned_images.clone();
        let pinned = Arc::new(
            PinnedImageRuntime::new(
                pin_state.clone(),
                composer.clone(),
                capture_output.clone(),
                pin_host.clone(),
            )
            .with_settings(state.settings.configuration.clone()),
        );
        let scrolling_host = Arc::new(NativeScrollingCaptureHost::new(source.clone()));
        let runtime = Arc::new(
            CaptureSessionRuntime::with_host(
                sessions.clone(),
                composer,
                capture_output.clone(),
                state.providers.ocr.clone(),
                capture_host.clone(),
                pinned.clone(),
            )
            .with_scrolling_host(Arc::new(ScrollingHost {
                native: scrolling_host.clone(),
                source: source.clone(),
            }))
            .with_settings(state.settings.configuration.clone()),
        );
        state.capture = Arc::new(app_state::CaptureRuntimeState {
            sessions,
            output: capture_output,
            runtime,
            pinned_images: pinned,
            cursor: Arc::new(FixtureCursor),
        });
        state.actions = app_actions::AppActionDispatcher::new(
            state.permissions.clone(),
            state.capture.clone(),
            state.selection.clone(),
            state.result_window.clone(),
            state.desktop_events.clone(),
        );
        capture_host.native.bind(&state);
        state.native_result_window.bind(&state);
        let events = state.desktop_events.clone();
        scrolling_host.bind(&state, move |session_id, completed| {
            events.publish(
                infrastructure::desktop_events::DesktopEvent::ScrollingCaptureEnded {
                    session_id: session_id.clone(),
                    completed,
                },
            )
        });
        let events = state.desktop_events.clone();
        state.native_pinned_images.bind(&state, move || {
            events.request(infrastructure::desktop_events::DesktopEvent::OpenSettings)
        });
        let recording = HotkeyRecording::default();
        let events = state.desktop_events.clone();
        infrastructure::system::shortcut::initialize_shortcuts(
            recording.clone(),
            move |label, payload| {
                events.publish(
                    infrastructure::desktop_events::DesktopEvent::HotkeyRecorded {
                        window_label: label.into(),
                        recording_id: payload.recording_id,
                        hotkey: payload.hotkey,
                    },
                )
            },
        )
        .unwrap();
        app_shell::initialize_menu_events();
        app_shell::test_native_menu_bar();
        let events = state.desktop_events.clone();
        app_shell::setup_menu_bar(&state.settings.hotkeys.snapshot().unwrap(), move |action| {
            events.publish(infrastructure::desktop_events::DesktopEvent::AppActionRequested(action))
        })
        .unwrap();
        let test_app = NativeTestApp {
            state: Arc::new(state),
            capture_services,
            recording,
        };
        let worker = test_app.clone();
        app.finishLaunching();
        std::thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                crate::infrastructure::runtime::block_on(async {
                    if measuring {
                        benchmark(&worker, &surface, pins, seconds, &pin_state, &*pin_host).await;
                    } else {
                        match surface.as_str() {
                            "editor" => {
                                crate::capture_workflows::editor_handoff(
                                    &worker,
                                    &output.clipboard,
                                )
                                .await;
                                crate::capture_workflows::editor_output_recovery(
                                    &worker,
                                    &output.clipboard,
                                    &output.fail_copy,
                                    &source.fail_scrolling_open,
                                )
                                .await;
                            }
                            "capture" => {
                                crate::capture_workflows::run(
                                    &worker,
                                    &output.directory,
                                    &output.clipboard,
                                )
                                .await
                            }
                            "result" => {
                                crate::result_workflows::run(&worker, result_services).await
                            }
                            _ => {
                                workflow(
                                    worker.clone(),
                                    source,
                                    capture_host,
                                    pin_state,
                                    pin_host,
                                    output,
                                    result_services,
                                )
                                .await
                            }
                        }
                    }
                    worker.state.actions.shutdown();
                    worker.state.shutdown().await.unwrap();
                    on_main(&worker,||{let app=NSApplication::sharedApplication(MainThreadMarker::new().unwrap());app.stop(None);if let Some(wake)=NSEvent::otherEventWithType_location_modifierFlags_timestamp_windowNumber_context_subtype_data1_data2(NSEventType::ApplicationDefined,NSPoint::ZERO,NSEventModifierFlags::empty(),0.,0,None,0,0,0){app.postEvent_atStart(&wake,true);}}).await;
                })
            }));
            if result.is_err() {
                std::process::exit(1);
            }
        });
        app.run();
        app_shell::shutdown_menu_bar();
        infrastructure::system::shortcut::shutdown_shortcuts().unwrap();
        drop(test_app);
        drop(directory);
    }
}

#[cfg(target_os = "macos")]
fn main() {
    fixture::run();
}
#[cfg(not(target_os = "macos"))]
fn main() {}

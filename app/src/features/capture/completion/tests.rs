use super::*;
use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, Mutex,
};
use std::time::Duration;

use image::ImageEncoder;
use snaplingo_core::capture::model::*;
use snaplingo_core::capture::{
    CaptureCursorMover, CaptureImageComposer, CaptureOutput, CaptureOutputHost,
    CaptureOutputSystemPaths, CapturePinOutput, CaptureSessionRuntime, CaptureSessionRuntimeHost,
    CaptureSessionSource, CaptureSessions, CaptureWindowGeometry,
};
use snaplingo_core::library::capacity::FavoriteCapacity;
use snaplingo_core::library::screenshot_favorites::{
    ScreenshotFavoriteCapture, ScreenshotFavoriteCaptureRenderer, ScreenshotFavoriteClipboard,
    ScreenshotFavoriteHost, ScreenshotFavorites,
};
use snaplingo_core::providers::ocr::{
    model::{OcrRequest, OcrResult},
    OcrCoordinator, OcrProvider,
};
use snaplingo_core::providers::Provider;
use snaplingo_core::settings::SettingsConfiguration;
use snaplingo_storage::{
    Database, FilesystemScreenshotFavoriteAssets, SqliteConfigStore,
    SqliteFavoriteCapacityRepository, SqliteScreenshotFavoriteRepository,
};

fn png(width: u32, height: u32, color: [u8; 4]) -> Vec<u8> {
    let mut result = Vec::new();
    image::codecs::png::PngEncoder::new(&mut result)
        .write_image(
            &color.repeat((width * height) as usize),
            width,
            height,
            image::ExtendedColorType::Rgba8,
        )
        .unwrap();
    result
}
fn bounds() -> LogicalRect {
    LogicalRect {
        x: 0.,
        y: 0.,
        width: 8.,
        height: 8.,
    }
}
fn annotation() -> AnnotationCommand {
    AnnotationCommand::Rectangle {
        rect: LogicalRect {
            x: 4.,
            y: 4.,
            width: 3.,
            height: 3.,
        },
        color: [255, 0, 0, 255],
        stroke_width: 1,
        filled: true,
        corner_radius: 0.,
    }
}
struct Source(AtomicUsize);
#[async_trait]
impl CaptureSessionSource for Source {
    async fn capture_monitor_snapshots(&self) -> Result<Vec<MonitorSnapshot>> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(vec![MonitorSnapshot {
            id: "display".into(),
            logical_bounds: bounds(),
            physical_bounds: PhysicalRect {
                x: 0,
                y: 0,
                width: 8,
                height: 8,
            },
            scale_factor: 1.,
            png_data: png(8, 8, [10, 20, 30, 255]),
        }])
    }
    async fn capture_monitor_layouts(&self) -> Result<Vec<MonitorLayout>> {
        Ok(vec![MonitorLayout {
            id: "display".into(),
            logical_bounds: bounds(),
            physical_bounds: PhysicalRect {
                x: 0,
                y: 0,
                width: 8,
                height: 8,
            },
            scale_factor: 1.,
        }])
    }
    async fn capture_cursor(&self, _: &[MonitorSnapshot]) -> Result<Option<CapturedCursor>> {
        Ok(Some(CapturedCursor {
            logical_position: LogicalPoint { x: 1., y: 1. },
            hotspot: LogicalPoint { x: 0., y: 0. },
            image_width: 1,
            image_height: 1,
            scale_factor: 1.,
            png_data: png(1, 1, [0, 255, 0, 255]),
        }))
    }
    async fn capture_region(&self, _: ScreenRegion) -> Result<Vec<u8>> {
        panic!("completion must use frozen pixels")
    }
}

struct Recorder {
    events: Mutex<Vec<&'static str>>,
    copies: Mutex<Vec<Vec<u8>>>,
    writes: Mutex<Vec<(PathBuf, Vec<u8>)>>,
    pins: Mutex<Vec<(Vec<u8>, Option<DesktopPosition>)>>,
    prints: Mutex<Vec<Vec<u8>>>,
    ocr: Mutex<Vec<CaptureOcrTarget>>,
    ocr_ready: tokio::sync::Notify,
    current: AtomicBool,
    stale_in_dialog: AtomicBool,
    fail_copy: AtomicBool,
    print_completed: AtomicBool,
    save_reply: Mutex<std::result::Result<Option<PathBuf>, String>>,
    save_defaults: Mutex<Vec<(PathBuf, String)>>,
}
impl Default for Recorder {
    fn default() -> Self {
        Self {
            events: Default::default(),
            copies: Default::default(),
            writes: Default::default(),
            pins: Default::default(),
            prints: Default::default(),
            ocr: Default::default(),
            ocr_ready: Default::default(),
            current: AtomicBool::new(true),
            stale_in_dialog: AtomicBool::new(false),
            fail_copy: AtomicBool::new(false),
            print_completed: AtomicBool::new(true),
            save_reply: Mutex::new(Ok(None)),
            save_defaults: Default::default(),
        }
    }
}
impl Recorder {
    fn event(&self, event: &'static str) {
        self.events.lock().unwrap().push(event);
    }
}
#[async_trait]
impl Effects for Recorder {
    async fn is_current(&self, _: &CaptureSessionId) -> Result<bool> {
        Ok(self.current.load(Ordering::SeqCst))
    }
    async fn save_path(
        &self,
        _: &CaptureSessionId,
        path: PathBuf,
        format: &str,
    ) -> Result<Option<PathBuf>> {
        self.event("dialog");
        self.save_defaults
            .lock()
            .unwrap()
            .push((path, format.into()));
        if self.stale_in_dialog.load(Ordering::SeqCst) {
            self.current.store(false, Ordering::SeqCst);
        }
        self.save_reply.lock().unwrap().clone().map_err(Into::into)
    }
    async fn print(&self, _: &CaptureSessionId, png: Vec<u8>) -> Result<bool> {
        self.event("print");
        self.prints.lock().unwrap().push(png);
        Ok(self.print_completed.load(Ordering::SeqCst))
    }
    async fn resume(&self, _: &CaptureSessionId) -> Result<()> {
        self.event("resume");
        Ok(())
    }
    async fn close(&self, _: &CaptureSessionId) -> Result<()> {
        self.event("close");
        Ok(())
    }
}
#[async_trait]
impl CaptureSessionRuntimeHost for Recorder {
    async fn begin_capture_presentation(&self) -> Result<()> {
        Ok(())
    }
    async fn end_capture_presentation(&self) -> Result<()> {
        self.event("end");
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
        self.event("hide");
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
    fn remember_capture_selection(&self, _: &LogicalRect) -> Result<()> {
        self.event("remember");
        Ok(())
    }
    async fn publish_capture_ocr(
        &self,
        target: CaptureOcrTarget,
        _: u64,
        _: OcrResult,
        _: Option<String>,
    ) -> Result<()> {
        self.ocr.lock().unwrap().push(target);
        self.ocr_ready.notify_one();
        Ok(())
    }
}
impl CaptureOutputHost for Recorder {
    fn system_paths(&self) -> CaptureOutputSystemPaths {
        CaptureOutputSystemPaths {
            download_dir: None,
            picture_dir: None,
            home_dir: None,
            temp_dir: std::env::temp_dir(),
        }
    }
    fn now(&self) -> chrono::DateTime<chrono::Local> {
        chrono::Local::now()
    }
    fn path_exists(&self, path: &Path) -> bool {
        path.is_dir()
    }
    fn write_file(&self, path: &Path, data: &[u8]) -> Result<()> {
        self.event("save");
        self.writes.lock().unwrap().push((path.into(), data.into()));
        Ok(())
    }
    fn copy_png(&self, data: &[u8]) -> Result<()> {
        if self.fail_copy.load(Ordering::SeqCst) {
            return Err("Clipboard busy".into());
        }
        self.event("copy");
        self.copies.lock().unwrap().push(data.into());
        Ok(())
    }
    fn read_clipboard_png(&self) -> Result<Vec<u8>> {
        panic!("unused")
    }
    fn read_clipboard_text(&self) -> Result<String> {
        panic!("unused")
    }
}
#[async_trait]
impl CapturePinOutput for Recorder {
    async fn pin_png(&self, png: Vec<u8>, position: Option<DesktopPosition>) -> Result<()> {
        self.event("pin");
        self.pins.lock().unwrap().push((png, position));
        Ok(())
    }
}
impl CaptureCursorMover for Recorder {
    fn move_relative(&self, _: i32, _: i32) {
        panic!("unused")
    }
}
impl ScreenshotFavoriteHost for Recorder {
    fn reveal(&self, _: &str) -> Result<()> {
        panic!("unused")
    }
}
#[async_trait]
impl ScreenshotFavoriteClipboard for Recorder {
    async fn copy_png(&self, _: &[u8]) -> Result<()> {
        panic!("unused")
    }
}
struct Renderer(Arc<CaptureSessionRuntime>);
impl ScreenshotFavoriteCaptureRenderer for Renderer {
    fn render_png(
        &self,
        id: &CaptureSessionId,
        rect: &LogicalRect,
        annotations: &[AnnotationCommand],
        cursor: bool,
    ) -> Result<Vec<u8>> {
        self.0.render_png(id, rect, annotations, cursor)
    }
}
struct Recognizer;
impl Provider for Recognizer {
    fn id(&self) -> &str {
        "test"
    }
    fn name(&self) -> &str {
        "Test"
    }
    fn is_configured(&self) -> bool {
        true
    }
    fn requires_api_key(&self) -> bool {
        false
    }
}
#[async_trait]
impl OcrProvider for Recognizer {
    async fn recognize(&self, request: &OcrRequest) -> Result<OcrResult> {
        assert_eq!(
            image::load_from_memory(&request.image_data)
                .unwrap()
                .to_rgba8()
                .get_pixel(1, 1)
                .0,
            [10, 20, 30, 255]
        );
        Ok(OcrResult::from_text("recognized", None))
    }
}
struct Fixture {
    root: tempfile::TempDir,
    database: Arc<Database>,
    runtime: Arc<CaptureSessionRuntime>,
    source: Arc<Source>,
    recorder: Arc<Recorder>,
    _favorites: Arc<ScreenshotFavoriteCapture>,
    services: Services,
    id: CaptureSessionId,
}
impl Fixture {
    async fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let database = Arc::new(Database::open(root.path().join("test.db")).unwrap());
        let store = Arc::new(SqliteConfigStore::new(database.clone()));
        let settings = Arc::new(SettingsConfiguration::with_paths(
            store.clone(),
            None,
            root.path().join("captures"),
        ));
        let source = Arc::new(Source(AtomicUsize::new(0)));
        let recorder = Arc::new(Recorder::default());
        let sessions = Arc::new(CaptureSessions::new(source.clone()));
        let output = Arc::new(CaptureOutput::with_host(recorder.clone()));
        let ocr = Arc::new(OcrCoordinator::new(store));
        ocr.register(Recognizer).unwrap();
        ocr.activate("test").unwrap();
        let runtime = Arc::new(
            CaptureSessionRuntime::with_host(
                sessions.clone(),
                Arc::new(CaptureImageComposer::new()),
                output.clone(),
                ocr,
                recorder.clone(),
                recorder.clone(),
            )
            .with_settings(settings.clone()),
        );
        let capacity = Arc::new(FavoriteCapacity::new(
            Arc::new(SqliteFavoriteCapacityRepository::new(database.clone())),
            settings.clone(),
        ));
        let favorites = Arc::new(ScreenshotFavoriteCapture::new(
            Arc::new(ScreenshotFavorites::new(
                Arc::new(SqliteScreenshotFavoriteRepository::new(database.clone())),
                Arc::new(FilesystemScreenshotFavoriteAssets::new(
                    root.path().join("assets"),
                )),
                recorder.clone(),
                recorder.clone(),
                capacity,
            )),
            Arc::new(Renderer(runtime.clone())),
        ));
        let services = Services {
            runtime: Arc::downgrade(&runtime),
            sessions: sessions.clone(),
            settings,
            output,
            cursor: recorder.clone(),
            favorites: Arc::downgrade(&favorites),
        };
        let id = runtime
            .create_session_from_visible_desktop()
            .await
            .unwrap()
            .id;
        sessions.hydrate_session_snapshots(&id).await.unwrap();
        recorder.events.lock().unwrap().clear();
        Self {
            root,
            database,
            runtime,
            source,
            recorder,
            _favorites: favorites,
            services,
            id,
        }
    }
    async fn complete(&self, command: CaptureCommand, mode: CaptureSelectionMode) -> Result<()> {
        complete(
            &self.services,
            Request {
                id: &self.id,
                mode,
                command,
                rect: &bounds(),
                annotations: &[annotation()],
                include_cursor: true,
            },
            self.recorder.as_ref(),
        )
        .await
    }
    fn expected(&self) -> Vec<u8> {
        self.runtime
            .render_png(&self.id, &bounds(), &[annotation()], true)
            .unwrap()
    }
}

#[tokio::test]
async fn primary_modes_share_the_real_output_and_ocr_handoff() {
    for (mode, target) in [
        (CaptureSelectionMode::Edit, None),
        (CaptureSelectionMode::Copy, None),
        (CaptureSelectionMode::Ocr, Some(CaptureOcrTarget::OcrWindow)),
        (
            CaptureSelectionMode::SilentOcr,
            Some(CaptureOcrTarget::Clipboard),
        ),
        (
            CaptureSelectionMode::Translate,
            Some(CaptureOcrTarget::TranslationWindow),
        ),
    ] {
        let f = Fixture::new().await;
        let expected = f.expected();
        f.complete(CaptureCommand::Primary, mode).await.unwrap();
        assert!(!f.services.sessions.has_session(&f.id));
        if let Some(target) = target {
            tokio::time::timeout(Duration::from_secs(2), f.recorder.ocr_ready.notified())
                .await
                .unwrap();
            assert_eq!(*f.recorder.ocr.lock().unwrap(), vec![target]);
            assert!(f.recorder.copies.lock().unwrap().is_empty());
        } else {
            assert_eq!(*f.recorder.copies.lock().unwrap(), vec![expected]);
        }
        assert_eq!(f.source.0.load(Ordering::SeqCst), 1);
        f.runtime.cancel_background_ocr();
    }
}

#[tokio::test]
async fn failed_copy_retains_frozen_annotations_and_cursor_for_retry_in_an_ocr_mode() {
    let f = Fixture::new().await;
    let expected = f.expected();
    f.recorder.fail_copy.store(true, Ordering::SeqCst);
    assert!(f
        .complete(CaptureCommand::Copy, CaptureSelectionMode::Ocr)
        .await
        .is_err());
    assert!(f.services.sessions.has_session(&f.id));
    assert!(f.recorder.events.lock().unwrap().is_empty());
    f.recorder.fail_copy.store(false, Ordering::SeqCst);
    f.complete(CaptureCommand::Copy, CaptureSelectionMode::Ocr)
        .await
        .unwrap();
    assert_eq!(*f.recorder.copies.lock().unwrap(), vec![expected]);
    assert!(f.recorder.ocr.lock().unwrap().is_empty());
    assert_eq!(
        *f.recorder.events.lock().unwrap(),
        vec!["copy", "remember", "hide", "end"]
    );
    assert_eq!(f.source.0.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn save_cancel_and_error_preserve_the_session_and_an_explicit_extension_wins_on_retry() {
    let f = Fixture::new().await;
    let mut settings = f.services.settings.snapshot().unwrap().screenshot;
    let previous = f.root.path().join("previous");
    std::fs::create_dir(&previous).unwrap();
    settings.format = "png".into();
    settings.auto_copy = true;
    f.services.settings.update_screenshot(settings).unwrap();
    f.services
        .settings
        .record_screenshot_save_choice(&previous.join("old.jpg"), "jpg")
        .unwrap();
    f.complete(CaptureCommand::Save, CaptureSelectionMode::Edit)
        .await
        .unwrap();
    assert!(f.services.sessions.has_session(&f.id));
    assert!(f.recorder.writes.lock().unwrap().is_empty());
    *f.recorder.save_reply.lock().unwrap() = Err("dialog failed".into());
    assert!(f
        .complete(CaptureCommand::Save, CaptureSelectionMode::Edit)
        .await
        .is_err());
    assert!(f.services.sessions.has_session(&f.id));
    assert!(!f.recorder.events.lock().unwrap().contains(&"remember"));
    let selected = f.root.path().join("chosen.webp");
    *f.recorder.save_reply.lock().unwrap() = Ok(Some(selected.clone()));
    let expected = f.expected();
    f.complete(CaptureCommand::Save, CaptureSelectionMode::Edit)
        .await
        .unwrap();
    let writes = f.recorder.writes.lock().unwrap();
    assert_eq!(writes[0].0, selected);
    assert_eq!(
        image::guess_format(&writes[0].1).unwrap(),
        image::ImageFormat::WebP
    );
    assert_eq!(*f.recorder.copies.lock().unwrap(), vec![expected]);
    for (path, format) in f.recorder.save_defaults.lock().unwrap().iter() {
        assert_eq!(path.parent(), Some(previous.as_path()));
        assert_eq!(format, "jpg");
        assert_eq!(path.extension().unwrap(), "jpg");
    }
    let saved = f.services.settings.snapshot().unwrap().screenshot;
    assert_eq!(saved.last_save_format.as_deref(), Some("webp"));
    assert!(!f.services.sessions.has_session(&f.id));
}

#[tokio::test]
async fn quick_save_uses_configured_preferences_and_save_without_extension_uses_last_choice() {
    for quick in [true, false] {
        let f = Fixture::new().await;
        let configured = f.root.path().join("configured");
        let mut settings = f.services.settings.snapshot().unwrap().screenshot;
        settings.save_path = configured.to_string_lossy().into_owned();
        settings.format = "png".into();
        settings.naming_rule = "custom".into();
        settings.custom_file_name = "shot".into();
        settings.auto_copy = false;
        f.services.settings.update_screenshot(settings).unwrap();
        f.services
            .settings
            .record_screenshot_save_choice(&f.root.path().join("missing/old.jpg"), "jpg")
            .unwrap();
        *f.recorder.save_reply.lock().unwrap() = Ok(Some(f.root.path().join("no-extension")));
        f.complete(
            if quick {
                CaptureCommand::QuickSave
            } else {
                CaptureCommand::Save
            },
            CaptureSelectionMode::Edit,
        )
        .await
        .unwrap();
        let writes = f.recorder.writes.lock().unwrap();
        assert_eq!(
            writes[0].0.extension().unwrap(),
            if quick { "png" } else { "jpg" }
        );
        assert_eq!(
            image::guess_format(&writes[0].1).unwrap(),
            if quick {
                image::ImageFormat::Png
            } else {
                image::ImageFormat::Jpeg
            }
        );
        if quick {
            assert_eq!(writes[0].0.parent(), Some(configured.as_path()));
            assert!(f.recorder.save_defaults.lock().unwrap().is_empty());
        } else {
            assert_eq!(
                f.recorder.save_defaults.lock().unwrap()[0].0.parent(),
                Some(configured.as_path())
            );
        }
        assert_eq!(
            f.services
                .settings
                .snapshot()
                .unwrap()
                .screenshot
                .last_save_format
                .as_deref(),
            Some("jpg")
        );
        assert!(f.recorder.copies.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn stale_completion_and_dialog_results_cannot_write_or_close_a_replacement() {
    for stale_in_dialog in [false, true] {
        let f = Fixture::new().await;
        f.recorder.current.store(stale_in_dialog, Ordering::SeqCst);
        f.recorder
            .stale_in_dialog
            .store(stale_in_dialog, Ordering::SeqCst);
        *f.recorder.save_reply.lock().unwrap() = Ok(Some(f.root.path().join("unused.png")));
        f.complete(CaptureCommand::Save, CaptureSelectionMode::Edit)
            .await
            .unwrap();
        assert!(f.services.sessions.has_session(&f.id));
        assert!(f.recorder.writes.lock().unwrap().is_empty());
        assert_eq!(
            *f.recorder.events.lock().unwrap(),
            if stale_in_dialog {
                vec!["dialog"]
            } else {
                vec![]
            }
        );
    }
}

#[tokio::test]
async fn favorite_failure_preserves_the_selection_and_success_closes_only_after_storage() {
    let f = Fixture::new().await;
    f.database.with_connection(|db| {
        db.execute_batch("CREATE TRIGGER reject_favorite BEFORE INSERT ON screenshot_favorites BEGIN SELECT RAISE(FAIL, 'test storage failure'); END;").map_err(snaplingo_core::AppError::database)?; Ok(())
    }).unwrap();
    assert!(f
        .complete(CaptureCommand::Favorite, CaptureSelectionMode::Edit)
        .await
        .is_err());
    assert!(f.services.sessions.has_session(&f.id));
    assert!(f.recorder.events.lock().unwrap().is_empty());
    f.database
        .with_connection(|db| {
            db.execute_batch("DROP TRIGGER reject_favorite;")
                .map_err(snaplingo_core::AppError::database)?;
            Ok(())
        })
        .unwrap();
    f.complete(CaptureCommand::Favorite, CaptureSelectionMode::Edit)
        .await
        .unwrap();
    let count: i64 = f
        .database
        .with_connection(|db| {
            db.query_row("SELECT COUNT(*) FROM screenshot_favorites", [], |row| {
                row.get(0)
            })
            .map_err(snaplingo_core::AppError::database)
        })
        .unwrap();
    assert_eq!(count, 1);
    assert_eq!(
        *f.recorder.events.lock().unwrap(),
        vec!["remember", "close", "end"]
    );
    assert!(!f.services.sessions.has_session(&f.id));
}

#[tokio::test]
async fn print_cancel_resumes_and_completed_print_reuses_the_frozen_annotated_image() {
    let f = Fixture::new().await;
    let expected = f.expected();
    f.recorder.print_completed.store(false, Ordering::SeqCst);
    f.complete(CaptureCommand::Print, CaptureSelectionMode::Edit)
        .await
        .unwrap();
    assert!(f.services.sessions.has_session(&f.id));
    assert_eq!(*f.recorder.events.lock().unwrap(), vec!["print", "resume"]);
    f.recorder.print_completed.store(true, Ordering::SeqCst);
    f.complete(CaptureCommand::Print, CaptureSelectionMode::Edit)
        .await
        .unwrap();
    assert_eq!(
        *f.recorder.prints.lock().unwrap(),
        vec![expected.clone(), expected]
    );
    assert_eq!(
        *f.recorder.events.lock().unwrap(),
        vec!["print", "resume", "print", "close", "end"]
    );
    assert!(!f.services.sessions.has_session(&f.id));
}

#[tokio::test]
async fn pin_output_keeps_annotations_cursor_and_desktop_position() {
    let f = Fixture::new().await;
    let expected = f.expected();
    f.complete(CaptureCommand::Pin, CaptureSelectionMode::Edit)
        .await
        .unwrap();
    assert_eq!(
        *f.recorder.pins.lock().unwrap(),
        vec![(
            expected,
            Some(DesktopPosition::Logical(LogicalPoint { x: 0., y: 0. }))
        )]
    );
    assert_eq!(
        *f.recorder.events.lock().unwrap(),
        vec!["pin", "remember", "hide", "end"]
    );
}

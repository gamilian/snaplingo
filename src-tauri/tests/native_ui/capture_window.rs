use std::io::Cursor;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use async_trait::async_trait;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSEvent, NSEventModifierFlags as Flags, NSEventType};
use objc2_foundation::{NSPoint, NSString};
use tauri::{Listener, Manager};

use crate::application::capture::{
    CaptureOcrEffects, CaptureSelectionMode, CaptureSession, CaptureWindowGeometry,
};
use crate::application::providers::common::Provider;
use crate::application::providers::ocr::OcrProvider;
use crate::application::result_window::{ResultWindowMode, TranslationStatus};
use crate::domain::capture::*;
use crate::domain::config::ScreenshotSettings;
use crate::domain::ocr::{OcrRequest, OcrResult};
use crate::fixture::on_main;
use crate::infrastructure::system::capture_window::editor_toolbar_macos as toolbar;
use crate::infrastructure::system::capture_window::native_view::{
    self as native, Action, Operation,
};
use crate::{AppState, Result};

#[derive(Default)]
pub(crate) struct Services {
    requests: Mutex<Vec<(u32, u32, Option<String>)>>,
    clipboard: Mutex<String>,
    errors: Mutex<Vec<String>>,
    fail: AtomicBool,
}
struct Recognizer(Arc<Services>);
impl Provider for Recognizer {
    fn id(&self) -> &str {
        "native-capture-fixture"
    }
    fn name(&self) -> &str {
        "本地测试 OCR"
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
        let image = image::load_from_memory(&request.image_data).unwrap();
        self.0.requests.lock().unwrap().push((
            image.width(),
            image.height(),
            request.language.clone(),
        ));
        tokio::time::sleep(Duration::from_millis(80)).await;
        if self.0.fail.swap(false, Ordering::SeqCst) {
            return Err("Synthetic capture OCR failure".into());
        }
        let mut result =
            OcrResult::from_text("native capture text", Some(0.95)).with_provider_id(self.id());
        result.detected_language = Some("en".into());
        Ok(result)
    }
}
impl CaptureOcrEffects for Services {
    fn copy_text(&self, text: String) -> Result<()> {
        *self.clipboard.lock().unwrap() = text;
        Ok(())
    }
    fn show_error(&self, error: &str) {
        self.errors.lock().unwrap().push(error.into());
    }
}
pub(crate) fn configure(state: &mut AppState) -> Arc<Services> {
    let services = Arc::new(Services::default());
    state
        .providers
        .ocr
        .register(Recognizer(services.clone()))
        .unwrap();
    state
        .providers
        .ocr
        .activate("native-capture-fixture")
        .unwrap();
    services
}

fn key(id: &str, text: &str, flags: Flags, code: u16) {
    let event = NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(
        NSEventType::KeyDown, NSPoint::new(0.0, 0.0), flags, 0.0, native::test_window_number(id), None,
        &NSString::from_str(text), &NSString::from_str(text), false, code,
    ).unwrap();
    native::test_key(id, &event);
}
fn pointer(id: &str, bounds: &LogicalRect, x: f64, y: f64, phase: &str, flags: Flags) {
    let kind = match phase {
        "down" => NSEventType::LeftMouseDown,
        "up" => NSEventType::LeftMouseUp,
        "right" => NSEventType::RightMouseDown,
        "flags" => NSEventType::FlagsChanged,
        _ => NSEventType::MouseMoved,
    };
    let event = if phase == "flags" {
        NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(
            NSEventType::FlagsChanged, NSPoint::new(0.0, 0.0), flags, 0.0, native::test_window_number(id), None,
            &NSString::new(), &NSString::new(), false, 58,
        )
    } else { NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure(
        kind, NSPoint::new(x - bounds.x, bounds.height - (y - bounds.y)), flags, 0.0, native::test_window_number(id), None, 1, 1, 0.0,
    ) }.unwrap();
    native::test_pointer(id, &event, phase);
}
fn fixture_bounds() -> LogicalRect {
    LogicalRect {
        x: 0.0,
        y: 0.0,
        width: 1280.0,
        height: 720.0,
    }
}
fn snapshot(name: &str, bytes: &[u8]) {
    if let Ok(directory) = std::env::var("SNAPLINGO_NATIVE_UI_SNAPSHOT_DIR") {
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(Path::new(&directory).join(name), bytes).unwrap();
    }
}

fn controls() {
    let pixels = image::RgbaImage::from_fn(800, 800, |x, y| match (x < 400, y < 400) {
        (true, true) => image::Rgba([240, 40, 40, 255]),
        (false, true) => image::Rgba([40, 240, 40, 255]),
        (true, false) => image::Rgba([40, 40, 240, 255]),
        _ => image::Rgba([240, 240, 40, 255]),
    });
    let mut encoded = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(pixels)
        .write_to(&mut encoded, image::ImageFormat::Png)
        .unwrap();
    let secondary = MonitorSnapshot {
        id: "secondary".into(),
        logical_bounds: LogicalRect {
            x: -400.0,
            y: -100.0,
            width: 400.0,
            height: 400.0,
        },
        physical_bounds: PhysicalRect {
            x: -800,
            y: -200,
            width: 800,
            height: 800,
        },
        scale_factor: 2.0,
        png_data: encoded.into_inner(),
    };
    let primary = MonitorSnapshot {
        id: "primary".into(),
        logical_bounds: LogicalRect {
            x: 0.0,
            y: 0.0,
            width: 400.0,
            height: 400.0,
        },
        ..secondary.clone()
    };
    let bounds = secondary.logical_bounds.clone();
    let session = CaptureSession {
        id: CaptureSessionId("native-selection-controls".into()),
        selection_monitor_id: Some("secondary".into()),
        layout_snapshots: Vec::new(),
        snapshots: vec![primary, secondary],
        desktop_scale: None,
        candidates: vec![CaptureCandidateView {
            id: "window".into(),
            kind: "window".into(),
            rect: LogicalRect {
                x: -350.0,
                y: -60.0,
                width: 220.0,
                height: 100.0,
            },
            priority: 10_000,
        }],
        captured_cursor: Some(CapturedCursor {
            logical_position: LogicalPoint { x: -200.0, y: 0.0 },
            hotspot: LogicalPoint { x: 2.0, y: 3.0 },
            image_width: 20,
            image_height: 10,
            scale_factor: 2.0,
            png_data: crate::fixture::png(20, 10, 0),
        }),
        hidden_window_labels: Vec::new(),
        created_at: SystemTime::now(),
    };
    let id = session.id.0.clone();
    let actions = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let captured = actions.clone();
    native::open(
        session,
        CaptureSelectionMode::Copy,
        CaptureWindowGeometry {
            bounds: bounds.clone(),
            desktop_scale: None,
        },
        ScreenshotSettings::default(),
        vec![LogicalRect {
            x: -340.0,
            y: -50.0,
            width: 150.0,
            height: 80.0,
        }],
        move |action| captured.borrow_mut().push(action),
    )
    .unwrap();
    assert_eq!(
        native::test_decoded_images(),
        1,
        "Only the session's selection display should be decoded"
    );
    let frame = native::test_frame(&id);
    assert_eq!(frame.origin.x, -400.0);
    assert_eq!(frame.size.width, 400.0);
    pointer(&id, &bounds, -390.0, -90.0, "move", Flags::empty());
    let bytes = native::test_snapshot(&id);
    snapshot("capture-native-desktop.png", &bytes);
    let image = image::load_from_memory(&bytes).unwrap().to_rgb8();
    for (x, y, expected) in [
        (0.1, 0.1, [240, 40, 40]),
        (0.9, 0.1, [40, 240, 40]),
        (0.1, 0.7, [40, 40, 240]),
        (0.9, 0.7, [240, 240, 40]),
    ] {
        let pixel = image.get_pixel(
            (image.width() as f64 * x) as u32,
            (image.height() as f64 * y) as u32,
        );
        // AppKit encodes the cached display in the monitor's ICC profile.
        // Check orientation here and verify original sRGB sampling independently.
        assert_eq!(
            pixel.0.map(|channel| channel > 150),
            expected.map(|channel| channel > 150)
        );
        pointer(
            &id,
            &bounds,
            bounds.x + bounds.width * x,
            bounds.y + bounds.height * y,
            "move",
            Flags::empty(),
        );
        assert_eq!(native::test_color(&id), Some(expected));
    }
    pointer(&id, &bounds, -320.0, -30.0, "move", Flags::Option);
    pointer(&id, &bounds, -320.0, -30.0, "flags", Flags::Option);
    assert_eq!(native::test_color(&id), Some([240, 40, 40]));
    key(&id, "c", Flags::Option, 8);
    assert!(matches!(&actions.borrow()[0], Action::CopyText(text) if text == "#F02828"));
    snapshot(
        "capture-native-selection-lens.png",
        &native::test_snapshot(&id),
    );
    pointer(&id, &bounds, -320.0, -30.0, "flags", Flags::empty());
    pointer(&id, &bounds, -360.0, -70.0, "down", Flags::empty());
    pointer(&id, &bounds, -180.0, 10.0, "move", Flags::Shift);
    assert_eq!(
        native::test_selection(&id),
        Some(LogicalRect {
            x: -360.0,
            y: -70.0,
            width: 80.0,
            height: 80.0
        })
    );
    pointer(&id, &bounds, -180.0, 10.0, "up", Flags::Shift);
    assert!(
        matches!(&actions.borrow()[1], Action::Complete(Operation::Primary, rect, false) if rect.x == -360.0 && rect.width == 80.0 && rect.height == 80.0)
    );
    native::action_finished(&id, None);
    key(&id, "`", Flags::empty(), 50);
    key(&id, "c", Flags::Command, 8);
    assert!(matches!(
        &actions.borrow()[2],
        Action::Complete(Operation::Copy, _, true)
    ));
    native::action_finished(&id, None);
    native::suspend(&id);
    native::action_finished(&id, None);
    assert!(native::is_current(&id));
    for (text, flags, code, operation) in [
        ("s", Flags::Command, 1, Operation::Save),
        ("s", Flags::Command | Flags::Shift, 1, Operation::QuickSave),
        ("t", Flags::Command, 17, Operation::Pin),
        ("p", Flags::Command, 35, Operation::Print),
    ] {
        key(&id, text, flags, code);
        assert!(
            matches!(actions.borrow().last(), Some(Action::Complete(actual, _, _)) if *actual == operation)
        );
        native::action_finished(&id, None);
    }
    key(&id, "r", Flags::empty(), 15);
    assert!(
        matches!(actions.borrow().last(), Some(Action::Complete(Operation::Primary, rect, _)) if rect.width == 150.0)
    );
    native::action_finished(&id, None);
    key(&id, "\u{1b}", Flags::empty(), 53);
    assert!(matches!(actions.borrow().last(), Some(Action::Cancel)));
    native::close();
    assert_eq!(native::test_decoded_images(), 0);
    let png = crate::fixture::png(300, 180, 0);
    let printed = native::test_print_snapshot(&png);
    let printed_image = image::load_from_memory(&printed).unwrap();
    assert!(printed_image.width() >= 300 && printed_image.height() >= 180);
    snapshot("capture-native-print.png", &printed);
}

async fn until(app: &tauri::AppHandle, condition: impl Fn() -> bool, message: &str) {
    let deadline = Instant::now() + Duration::from_secs(8);
    while !condition() {
        assert!(Instant::now() < deadline, "{message}");
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    assert!(
        app.webview_windows().is_empty(),
        "A native quick capture created a WebView"
    );
}
async fn open(app: &tauri::AppHandle, mode: &str) -> String {
    let previous = on_main(app, native::session_id).await;
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        // The runtime deliberately ignores reentry while OCR delivery/startup
        // owns its lock. Wait for that handoff before starting the next scenario.
        app.state::<AppState>()
            .capture
            .runtime
            .open_capture_window_for_mode(mode)
            .await
            .unwrap();
        if let Some(id) = on_main(app, native::session_id)
            .await
            .filter(|id| Some(id) != previous.as_ref())
        {
            assert!(app.webview_windows().is_empty());
            return id;
        }
        assert!(
            Instant::now() < deadline,
            "Native quick capture did not open: {mode}"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}
async fn at_candidate(app: &tauri::AppHandle, id: &str) {
    let id = id.to_owned();
    on_main(app, move || {
        pointer(&id, &fixture_bounds(), 500.0, 350.0, "move", Flags::empty())
    })
    .await;
}
async fn drag(app: &tauri::AppHandle, id: &str) {
    let id = id.to_owned();
    on_main(app, move || {
        pointer(&id, &fixture_bounds(), 120.0, 140.0, "down", Flags::empty());
        pointer(&id, &fixture_bounds(), 360.0, 300.0, "move", Flags::empty());
        pointer(&id, &fixture_bounds(), 360.0, 300.0, "up", Flags::empty());
    })
    .await;
}
async fn press(app: &tauri::AppHandle, id: &str, text: &'static str, flags: Flags, code: u16) {
    let id = id.to_owned();
    on_main(app, move || key(&id, text, flags, code)).await;
}
async fn released(app: &tauri::AppHandle, id: &str) {
    let state = app.state::<AppState>();
    until(
        app,
        || {
            !state
                .capture
                .sessions
                .has_session(&CaptureSessionId(id.to_owned()))
        },
        "Native capture retained its frozen session after completion",
    )
    .await;
    assert_eq!(on_main(app, native::test_decoded_images).await, 0);
    assert!(!crate::infrastructure::system::capture_window::is_capture_presentation_active());
}

pub(crate) async fn run(app: &tauri::AppHandle, directory: &Path, clipboard: &Mutex<Vec<u8>>) {
    on_main(app, controls).await;
    let state = app.state::<AppState>();
    let services = app.state::<Arc<Services>>();
    let id = open(app, "screenshot-copy").await;
    let expected = state
        .capture
        .runtime
        .render_png(
            &CaptureSessionId(id.clone()),
            &LogicalRect {
                x: 120.0,
                y: 140.0,
                width: 240.0,
                height: 160.0,
            },
            &[],
            false,
        )
        .unwrap();
    drag(app, &id).await;
    released(app, &id).await;
    assert_eq!(*clipboard.lock().unwrap(), expected);
    assert_eq!(
        state
            .capture
            .runtime
            .capture_selection_history(Vec::new())
            .unwrap()[0]
            .width,
        240.0
    );

    let id = open(app, "screenshot-copy").await;
    at_candidate(app, &id).await;
    press(app, &id, "s", Flags::Command | Flags::Shift, 1).await;
    released(app, &id).await;
    let saved = std::fs::read_dir(directory)
        .unwrap()
        .filter_map(|entry| {
            let path = entry.unwrap().path();
            (path.extension().is_some_and(|extension| extension == "png")
                && path.file_name().unwrap() != "longshot.png")
                .then_some(path)
        })
        .collect::<Vec<_>>();
    assert_eq!(saved.len(), 1);
    let saved = image::open(&saved[0]).unwrap();
    assert_eq!((saved.width(), saved.height()), (320, 200));

    let id = open(app, "screenshot-ocr").await;
    at_candidate(app, &id).await;
    state.capture.runtime.pin_from_shortcut().await.unwrap();
    released(app, &id).await;
    assert!(
        services.requests.lock().unwrap().is_empty(),
        "Explicit pin should bypass the OCR mode"
    );
    assert_eq!(
        on_main(app, || NSApplication::sharedApplication(
            MainThreadMarker::new().unwrap()
        )
        .windows()
        .iter()
        .filter(|window| window.title().to_string() == "SnapLingo Pin")
        .count())
        .await,
        1
    );
    state.capture.pinned_images.close_all().await.unwrap();

    let id = open(app, "screenshot-copy").await;
    press(app, &id, "r", Flags::empty(), 15).await;
    released(app, &id).await;
    let copied = image::load_from_memory(&clipboard.lock().unwrap()).unwrap();
    assert_eq!((copied.width(), copied.height()), (320, 200));

    for mode in [
        "screenshot-ocr",
        "silent-screenshot-ocr",
        "screenshot-translate",
    ] {
        state.result_session.close();
        let before = services.requests.lock().unwrap().len();
        let id = open(app, mode).await;
        drag(app, &id).await;
        released(app, &id).await;
        until(
            app,
            || services.requests.lock().unwrap().len() > before,
            "Native selection did not dispatch OCR",
        )
        .await;
        // Existing OCR requests retain two logical pixels of surrounding context.
        assert_eq!(services.requests.lock().unwrap().last().unwrap().0, 244);
        assert_eq!(services.requests.lock().unwrap().last().unwrap().1, 164);
        if mode == "silent-screenshot-ocr" {
            until(
                app,
                || *services.clipboard.lock().unwrap() == "native capture text",
                "Silent OCR did not copy text after closing native selection",
            )
            .await;
            assert!(!state.result_session.snapshot().visible);
        } else {
            until(
                app,
                || {
                    let view = state.result_session.snapshot();
                    view.visible
                        && if mode == "screenshot-ocr" {
                            view.mode == ResultWindowMode::Ocr
                                && view.ocr_text == "native capture text"
                        } else {
                            view.mode == ResultWindowMode::Translation
                                && view.source_text == "native capture text"
                        }
                },
                "OCR did not open the native result window",
            )
            .await;
            let view = state.result_session.snapshot();
            if mode == "screenshot-ocr" {
                assert_eq!(view.mode, ResultWindowMode::Ocr);
                assert_eq!(view.ocr_text, "native capture text");
                assert!(view.ocr_image.is_some());
            } else {
                until(
                    app,
                    || {
                        state
                            .result_session
                            .snapshot()
                            .rows
                            .first()
                            .is_some_and(|row| row.status == TranslationStatus::Success)
                    },
                    "Screenshot translation did not complete natively",
                )
                .await;
                assert_eq!(
                    state.result_session.snapshot().rows[0]
                        .result
                        .translated_text,
                    "原生译文：native capture text"
                );
            }
        }
    }
    state.result_session.close();
    let id = open(app, "screenshot-copy").await;
    at_candidate(app, &id).await;
    press(app, &id, "\t", Flags::empty(), 48).await;
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let query = id.clone();
        if on_main(app, move || {
            native::test_selection(&query)
                .is_some_and(|rect| rect.width == 160.0 && rect.height == 80.0)
        })
        .await
        {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "Native element detection did not update"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    press(app, &id, "\r", Flags::empty(), 36).await;
    released(app, &id).await;
    let copied = image::load_from_memory(&clipboard.lock().unwrap()).unwrap();
    assert_eq!((copied.width(), copied.height()), (160, 80));

    let id = open(app, "screenshot-copy").await;
    press(app, &id, "\u{f708}", Flags::empty(), 96).await;
    let deadline = Instant::now() + Duration::from_secs(5);
    let refreshed = loop {
        if let Some(current) = on_main(app, native::session_id)
            .await
            .filter(|current| current != &id)
        {
            break current;
        }
        assert!(
            Instant::now() < deadline,
            "F5 did not replace the native frozen session"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    };
    until(
        app,
        || {
            !state
                .capture
                .sessions
                .has_session(&CaptureSessionId(id.clone()))
        },
        "F5 kept the old snapshot",
    )
    .await;
    let replacement = open(app, "screenshot-copy").await;
    until(
        app,
        || {
            !state
                .capture
                .sessions
                .has_session(&CaptureSessionId(refreshed.clone()))
        },
        "Repeated screenshot retained the previous native session",
    )
    .await;
    let check = replacement.clone();
    on_main(app, move || {
        pointer(
            &check,
            &fixture_bounds(),
            120.0,
            140.0,
            "down",
            Flags::empty(),
        );
        pointer(
            &check,
            &fixture_bounds(),
            240.0,
            220.0,
            "move",
            Flags::empty(),
        );
        pointer(
            &check,
            &fixture_bounds(),
            240.0,
            220.0,
            "right",
            Flags::empty(),
        );
        assert_eq!(native::test_selection(&check), None);
        pointer(
            &check,
            &fixture_bounds(),
            240.0,
            220.0,
            "right",
            Flags::empty(),
        );
    })
    .await;
    released(app, &replacement).await;

    services.fail.store(true, Ordering::SeqCst);
    let id = open(app, "silent-screenshot-ocr").await;
    drag(app, &id).await;
    released(app, &id).await;
    until(
        app,
        || !services.errors.lock().unwrap().is_empty(),
        "Native OCR errors were lost after selection closed",
    )
    .await;
    assert!(services.errors.lock().unwrap()[0].contains("Synthetic capture OCR failure"));
    assert!(app.webview_windows().is_empty());
    println!("Native quick capture pixels, negative/Retina coordinates, selection, keys, history, copy/save/pin, OCR/translation, control detection, refresh and cleanup passed");
}

async fn wait_for_editor(app: &tauri::AppHandle) -> tauri::WebviewWindow {
    let deadline = Instant::now() + Duration::from_secs(12);
    loop {
        if let Some(window) = app.get_webview_window("capture") {
            if window.is_visible().unwrap() && on_main(app, native::session_id).await.is_none() {
                return window;
            }
        }
        assert!(
            Instant::now() < deadline,
            "Native selection did not reach the annotation editor"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

async fn wait_for_toolbar(
    app: &tauri::AppHandle,
    id: &str,
    tool: Option<&str>,
    undo: bool,
    redo: bool,
    visible: bool,
) {
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        let state = on_main(app, toolbar::test_state).await;
        if state.as_ref().is_some_and(|state| {
            state.session_id == id
                && state.active_tool.as_deref() == tool
                && state.can_undo == undo
                && state.can_redo == redo
                && state.visible == visible
                && state.requested_visible == visible
        }) {
            assert!(
                !state.unwrap().can_become_key,
                "Toolbar must retain the editor's keyboard focus"
            );
            return;
        }
        assert!(
            Instant::now() < deadline,
            "Native toolbar did not synchronize: expected {id} {tool:?} undo={undo} redo={redo} visible={visible}; current {state:?}"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

async fn wait_for_styles(
    app: &tauri::AppHandle,
    ready: impl Fn(&crate::application::capture::CaptureEditorStylePresentation) -> bool,
) -> crate::application::capture::CaptureEditorStylePresentation {
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        let style = on_main(app, || {
            toolbar::test_styles(|controls| controls.test_state())
        })
        .await;
        if ready(&style) {
            return style;
        }
        assert!(
            Instant::now() < deadline,
            "Native style controls did not synchronize: {style:?}"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

async fn editor_probe(
    app: &tauri::AppHandle,
    window: &tauri::WebviewWindow,
    script: &str,
) -> serde_json::Value {
    let (done, pending) = tokio::sync::oneshot::channel();
    app.once("native-ui-editor-probe", move |event| {
        let _ = done.send(event.payload().to_owned());
    });
    window.eval(&format!(r#"(async () => {{
        try {{ window.__TAURI__.event.emit('native-ui-editor-probe', await (async () => {{ {script} }})()); }}
        catch(error) {{ window.__TAURI__.event.emit('native-ui-editor-probe', {{error:String(error)}}); }}
    }})()"#)).unwrap();
    let result = tokio::time::timeout(Duration::from_secs(8), pending)
        .await
        .expect("Editor probe did not respond")
        .unwrap();
    let result: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(
        result.get("error").is_none(),
        "Editor probe failed: {result}"
    );
    result
}

pub(crate) async fn editor_handoff(app: &tauri::AppHandle, clipboard: &Mutex<Vec<u8>>) {
    let state = app.state::<AppState>();
    let selected = LogicalRect {
        x: 120.0,
        y: 140.0,
        width: 240.0,
        height: 160.0,
    };
    let id = open(app, "screenshot").await;
    let expected = state
        .capture
        .runtime
        .render_png(&CaptureSessionId(id.clone()), &selected, &[], false)
        .unwrap();
    assert!(
        app.webview_windows().is_empty(),
        "Ordinary capture selection must be native"
    );
    let snapshot_id = id.clone();
    let bytes = on_main(app, move || native::test_snapshot(&snapshot_id)).await;
    snapshot("capture-native-screenshot-entry.png", &bytes);
    drag(app, &id).await;
    let window = wait_for_editor(app).await;
    wait_for_toolbar(app, &id, Some("rectangle"), false, false, true).await;
    on_main(app, || {
        toolbar::test_styles(|style| style.test_color([8, 16, 24, 200]))
    })
    .await;
    wait_for_styles(app, |style| style.style.color == [8, 16, 24, 200]).await;
    on_main(app, || toolbar::test_styles(|style| style.test_button(5))).await;
    let colors = wait_for_styles(app, |style| style.color_presets.contains(&[8, 16, 24, 200]))
        .await
        .color_presets;
    let added = colors
        .iter()
        .position(|color| *color == [8, 16, 24, 200])
        .unwrap();
    on_main(app, move || {
        toolbar::test_styles(|style| style.test_preset(added))
    })
    .await;
    on_main(app, || {
        toolbar::test_styles(|style| style.test_color([90, 80, 70, 128]))
    })
    .await;
    wait_for_styles(app, |style| style.style.color == [90, 80, 70, 128]).await;
    on_main(app, || toolbar::test_styles(|style| style.test_button(6))).await;
    wait_for_styles(app, |style| {
        style.color_presets.contains(&[90, 80, 70, 128])
            && !style.color_presets.contains(&[8, 16, 24, 200])
    })
    .await;
    on_main(app, || toolbar::test_styles(|style| style.test_button(7))).await;
    wait_for_styles(app, |style| {
        !style.color_presets.contains(&[90, 80, 70, 128])
    })
    .await;
    on_main(app, || {
        toolbar::test_styles(|style| style.test_size(6.0, true))
    })
    .await;
    wait_for_styles(app, |style| style.style.stroke_width == 6).await;
    on_main(app, || {
        toolbar::test_styles(|style| style.test_corner(12.0))
    })
    .await;
    wait_for_styles(app, |style| style.style.corner_radius == 12.0).await;
    let bytes = on_main(app, toolbar::test_snapshot).await;
    snapshot("capture-native-editor-toolbar.png", &bytes);
    let dragged_size = on_main(app, || toolbar::test_styles(|style| style.test_drag_size())).await;
    assert!(dragged_size > 6, "Native slider did not move");
    wait_for_styles(app, |style| style.style.stroke_width == dragged_size).await;
    let deadline = Instant::now() + Duration::from_secs(8);
    while u32::from(
        state
            .settings
            .configuration
            .snapshot()
            .unwrap()
            .screenshot
            .default_stroke_width,
    ) != dragged_size
    {
        assert!(
            Instant::now() < deadline,
            "Native slider did not commit the size default"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    // AppKit animates the knob after tracking; capture its settled presentation.
    tokio::time::sleep(Duration::from_millis(400)).await;
    let bytes = on_main(app, toolbar::test_snapshot).await;
    snapshot("capture-native-editor-slider.png", &bytes);
    on_main(app, || {
        toolbar::test_styles(|style| style.test_size(6.0, true))
    })
    .await;
    wait_for_styles(app, |style| style.style.stroke_width == 6).await;
    assert_eq!(
        on_main(
            app,
            || toolbar::test_styles(|style| style.test_size_value())
        )
        .await,
        (6.0, 1.0, 32.0),
        "Native slider did not reflect the confirmed style"
    );
    let (checked, dom) = tokio::sync::oneshot::channel();
    app.once("native-ui-toolbar-dom", move |event| {
        let _ = checked.send(event.payload().to_owned());
    });
    window.eval(r#"const check = window.setInterval(() => {
        if (!document.querySelector('[aria-label="Copy selection"]') && document.querySelector('[data-capture-toolbar][aria-hidden="true"]')) {
            window.clearInterval(check);
            window.__TAURI__.event.emit('native-ui-toolbar-dom', {
                copy:!!document.querySelector('[aria-label="Copy selection"]'),
                shapes:!!document.querySelector('[aria-label="Shapes"]'),
                style:!!document.querySelector('[aria-label="Annotation stroke width"]')
            });
        }
    }, 25)"#).unwrap();
    let dom = tokio::time::timeout(Duration::from_secs(8), dom)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&dom).unwrap(),
        serde_json::json!({"copy":false,"shapes":false,"style":false})
    );
    on_main(app, || {
        toolbar::test_styles(|style| style.test_open_color())
    })
    .await;
    assert!(
        on_main(app, || toolbar::test_styles(
            |style| style.test_color_active()
        ))
        .await
    );
    assert!(
        state
            .capture
            .runtime
            .capture_editor_owns_focus(&CaptureSessionId(id.clone()))
            .await
            .unwrap(),
        "Native color picker does not own editor focus"
    );
    on_main(app, || {
        toolbar::test_styles(|style| style.close_color_panel())
    })
    .await;
    assert!(
        !on_main(app, || toolbar::test_styles(
            |style| style.test_color_active()
        ))
        .await
    );
    wait_for_toolbar(app, &id, Some("rectangle"), false, false, true).await;
    for tool in [
        "ellipse",
        "arrow",
        "line",
        "pen",
        "text",
        "mosaic",
        "eraser",
        "highlight",
        "region-highlight",
        "blur",
        "spotlight",
        "rectangle",
    ] {
        on_main(app, move || toolbar::test_click(tool)).await;
        wait_for_toolbar(app, &id, Some(tool), false, false, true).await;
        if tool == "text" {
            let fonts = wait_for_styles(app, |style| {
                style.text_mode && !style.font_families.is_empty()
            })
            .await
            .font_families;
            window.set_focus().unwrap();
            let draft = editor_probe(app, &window, r#"
                const bounds = document.querySelector('[data-capture-editor-selection]').getBoundingClientRect();
                for (const type of ['pointerdown','pointerup']) {
                    document.querySelector('[data-capture-editor-selection]').dispatchEvent(new PointerEvent(type, {
                        bubbles:true,cancelable:true,pointerId:1,pointerType:'mouse',isPrimary:true,
                        button:0,buttons:type==='pointerup'?0:1,clientX:bounds.left+40,clientY:bounds.top+60
                    }));
                    await new Promise(resolve => window.setTimeout(resolve, 40));
                }
                const input = document.querySelector('[data-screenshot-text-draft]');
                Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype,'value').set.call(input,'原生样式');
                input.dispatchEvent(new InputEvent('input',{bubbles:true}));
                await new Promise(resolve => window.setTimeout(resolve, 40));
                return {text:document.querySelector('[data-screenshot-text-draft]')?.value};
            "#).await;
            assert_eq!(draft["text"], "原生样式");
            let family = fonts[0].clone();
            let selected_family = family.clone();
            on_main(app, move || {
                toolbar::test_styles(|style| style.test_font(&family))
            })
            .await;
            wait_for_styles(app, |style| style.style.font_family == selected_family).await;
            on_main(app, || toolbar::test_styles(|style| style.test_button(1))).await;
            wait_for_styles(app, |style| style.style.bold).await;
            on_main(app, || toolbar::test_styles(|style| style.test_button(2))).await;
            wait_for_styles(app, |style| style.style.italic).await;
            on_main(app, || {
                toolbar::test_styles(|style| style.test_size(36.0, true))
            })
            .await;
            wait_for_styles(app, |style| style.font_size == 36).await;
            on_main(app, || {
                toolbar::test_styles(|style| style.test_open_color())
            })
            .await;
            assert!(
                on_main(app, || toolbar::test_styles(
                    |style| style.test_color_active()
                ))
                .await
            );
            let draft = editor_probe(app, &window, r#"
                await new Promise(resolve => window.setTimeout(resolve, 75));
                const input = document.querySelector('[data-screenshot-text-draft]');
                return {text:input?.value, fontSize:input?.style.fontSize, family:input?.style.fontFamily,
                    bold:input?.style.fontWeight, italic:input?.style.fontStyle, focused:document.hasFocus()};
            "#).await;
            assert_eq!(
                draft["text"], "原生样式",
                "Native picker committed the text draft: {draft}"
            );
            assert_eq!(draft["fontSize"], "36px");
            assert!(
                draft["family"].as_str().unwrap().contains(&selected_family),
                "Native font family {selected_family:?} was not applied to the draft: {draft}"
            );
            assert_eq!(draft["bold"], "bold");
            assert_eq!(draft["italic"], "italic");
            assert_eq!(draft["focused"], false, "Native picker did not take focus");
            let bytes = on_main(app, toolbar::test_snapshot).await;
            snapshot("capture-native-editor-text-styles.png", &bytes);
            on_main(app, || {
                toolbar::test_styles(|style| style.close_color_panel())
            })
            .await;
            window.set_focus().unwrap();
            let committed = editor_probe(app, &window, r#"
                for (let attempt=0; attempt<80 && !document.hasFocus(); attempt++) {
                    await new Promise(resolve => window.setTimeout(resolve, 25));
                }
                if (!document.hasFocus()) throw new Error('Text input did not regain native focus');
                const input = document.querySelector('[data-screenshot-text-draft]');
                input.focus();
                input.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true,cancelable:true}));
                await new Promise(resolve => window.setTimeout(resolve, 40));
                return {draft:!!document.querySelector('[data-screenshot-text-draft]')};
            "#).await;
            assert_eq!(
                committed["draft"], false,
                "Enter did not commit the styled text"
            );
            wait_for_toolbar(app, &id, Some("text"), true, false, true).await;
            on_main(app, || toolbar::test_click("clear")).await;
            wait_for_toolbar(app, &id, Some("text"), false, false, true).await;
        } else if tool == "blur" {
            on_main(app, || {
                toolbar::test_styles(|style| style.test_size(32.0, false))
            })
            .await;
            wait_for_styles(app, |style| style.style.blur_radius == 32.0).await;
        }
    }
    on_main(app, || toolbar::test_styles(|style| style.test_button(0))).await;
    wait_for_styles(app, |style| style.style.filled).await;
    let (drawn, drawing) = tokio::sync::oneshot::channel();
    app.once("native-ui-toolbar-drawing", move |event| {
        let _ = drawn.send(event.payload().to_owned());
    });
    window.eval(r#"(async () => {
        const errors = [];
        const failed = event => errors.push(event.message);
        window.addEventListener('error', failed);
        try {
            const selection = document.querySelector('[data-capture-editor-selection]');
            const bounds = selection.getBoundingClientRect();
            for (const [type,x,y] of [['pointerdown',0.15,0.2],['pointermove',0.65,0.65],['pointerup',0.65,0.65]]) {
                document.querySelector('[data-capture-editor-selection]').dispatchEvent(new PointerEvent(type,{bubbles:true,cancelable:true,pointerId:1,
                    pointerType:'mouse',isPrimary:true,detail:1,button:0,buttons:type==='pointerup'?0:1,
                    clientX:bounds.left+x*bounds.width,clientY:bounds.top+y*bounds.height}));
                await new Promise(resolve => window.setTimeout(resolve, 40));
            }
            window.__TAURI__.event.emit('native-ui-toolbar-drawing', {
                left:bounds.left,top:bounds.top,width:bounds.width,height:bounds.height,
                errors,handles:document.querySelectorAll('[aria-label^="Resize annotation"]').length});
        } catch (error) {
            window.__TAURI__.event.emit('native-ui-toolbar-drawing', {error:String(error)});
        } finally {
            window.removeEventListener('error', failed);
        }
    })()"#).unwrap();
    let drawing = tokio::time::timeout(Duration::from_secs(8), drawing)
        .await
        .unwrap()
        .unwrap();
    let drawing = serde_json::from_str::<serde_json::Value>(&drawing).unwrap();
    assert!(
        drawing.get("error").is_none(),
        "Editor input failed: {drawing}"
    );
    assert_eq!(
        drawing["errors"],
        serde_json::json!([]),
        "Editor input failed: {drawing}"
    );
    assert!(drawing["width"].as_f64().unwrap() > 0.0 && drawing["height"].as_f64().unwrap() > 0.0);
    wait_for_toolbar(app, &id, Some("rectangle"), true, false, true).await;
    on_main(app, || toolbar::test_click("undo")).await;
    wait_for_toolbar(app, &id, Some("rectangle"), false, true, true).await;
    on_main(app, || toolbar::test_click("redo")).await;
    wait_for_toolbar(app, &id, Some("rectangle"), true, false, true).await;
    on_main(app, || toolbar::test_click("clear")).await;
    wait_for_toolbar(app, &id, Some("rectangle"), false, false, true).await;
    assert_eq!(app.webview_windows().len(), 1);
    assert_eq!(on_main(app, native::test_decoded_images).await, 0);
    assert!(state
        .capture
        .sessions
        .has_session(&CaptureSessionId(id.clone())));
    assert_eq!(
        state
            .capture
            .runtime
            .render_png(&CaptureSessionId(id.clone()), &selected, &[], false)
            .unwrap(),
        expected
    );
    // Native controls use the existing annotation/output workflow and frozen pixels.
    on_main(app, || {
        toolbar::test_styles(|style| style.test_open_color())
    })
    .await;
    on_main(app, || toolbar::test_click("copy")).await;
    let deadline = Instant::now() + Duration::from_secs(8);
    while state
        .capture
        .sessions
        .has_session(&CaptureSessionId(id.clone()))
        || !app.webview_windows().is_empty()
    {
        assert!(
            Instant::now() < deadline,
            "Completed annotation editor was not released"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    assert_eq!(*clipboard.lock().unwrap(), expected);
    assert!(on_main(app, toolbar::test_state).await.is_none());
    assert!(
        !on_main(app, || {
            objc2_app_kit::NSColorPanel::sharedColorPanel(MainThreadMarker::new().unwrap())
                .isVisible()
        })
        .await,
        "Editor completion did not hide its native picker"
    );
    assert!(!crate::infrastructure::system::capture_window::is_capture_presentation_active());

    let id = open(app, "screenshot").await;
    at_candidate(app, &id).await;
    press(app, &id, "\r", Flags::empty(), 36).await;
    released(app, &id).await;
    let copied = image::load_from_memory(&clipboard.lock().unwrap()).unwrap();
    assert_eq!((copied.width(), copied.height()), (320, 200));
    let id = open(app, "screenshot").await;
    drag(app, &id).await;
    let window = wait_for_editor(app).await;
    let native_window = window.ns_window().unwrap() as usize;
    let (refreshed, ready) = tokio::sync::oneshot::channel();
    app.once("native-ui-editor-refreshed", move |event| {
        let _ = refreshed.send(event.payload().to_owned());
    });
    window
        .eval(
            r#"window.dispatchEvent(new KeyboardEvent('keydown', {key:'F5', bubbles:true}));
            const refreshed = window.setInterval(() => {
                const monitor = document.querySelector('[data-capture-monitor]');
                if (document.querySelector('[aria-label="选区快捷键提示"]') && monitor?.naturalWidth && !document.querySelector('[aria-label="Loading capture"]')) {
                    window.clearInterval(refreshed);
                    window.__TAURI__.event.emit('native-ui-editor-refreshed', {width:monitor.naturalWidth,height:monitor.naturalHeight});
                }
            }, 25);"#,
        )
        .unwrap();
    let refreshed = tokio::time::timeout(Duration::from_secs(8), ready)
        .await
        .expect("Editor refresh did not reach its frozen selecting state")
        .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&refreshed).unwrap(),
        serde_json::json!({"width":1280,"height":720})
    );
    // DOM readiness precedes the host's decode/paint and AppKit reveal.
    let deadline = Instant::now() + Duration::from_secs(8);
    while !window.is_visible().unwrap() {
        assert!(
            Instant::now() < deadline,
            "Refreshed editor did not reveal its frozen desktop"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    assert!(!state
        .capture
        .sessions
        .has_session(&CaptureSessionId(id.clone())));
    assert_eq!(app.webview_windows().len(), 1);
    assert_eq!(window.ns_window().unwrap() as usize, native_window);
    assert!(window.is_visible().unwrap());
    assert!(on_main(app, native::session_id).await.is_none());
    assert!(on_main(app, toolbar::test_state).await.is_none());
    window
        .eval("window.dispatchEvent(new KeyboardEvent('keydown', {key:'Escape', bubbles:true}))")
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(8);
    while !app.webview_windows().is_empty()
        || crate::infrastructure::system::capture_window::is_capture_presentation_active()
    {
        assert!(
            Instant::now() < deadline,
            "Refreshed editor was not released"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    println!("Native screenshot entry, annotation toolbar controls, frozen editor handoff, copy, refresh and resource cleanup passed");
}

pub(crate) fn start_benchmark(app: &tauri::AppHandle, renderer: &str) {
    let app = app.clone();
    let is_native = renderer == "native";
    std::thread::spawn(move || {
        tauri::async_runtime::block_on(async {
            app.state::<AppState>()
                .capture
                .runtime
                .open_capture_window_for_mode("screenshot-copy")
                .await
                .unwrap();
            if is_native {
                let id = on_main(&app, native::session_id).await.unwrap();
                at_candidate(&app, &id).await;
                let (width, height, visible) =
                    on_main(&app, move || native::test_ready_size(&id)).await;
                println!(
                    "{}",
                    serde_json::json!({ "captureWidth": width, "captureHeight": height, "selecting": visible, "loading": false, "webviews": app.webview_windows().len() })
                );
            }
        })
    });
}

use std::io::Cursor;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use crate::fixture::NativeTestApp;
use async_trait::async_trait;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSEvent, NSEventModifierFlags as Flags, NSEventType};
use objc2_foundation::{NSPoint, NSString};

use crate::fixture::on_main;
use crate::window::capture::macos::toolbar;
use crate::window::capture::macos::view::{self as native, Action, Operation};
use crate::{AppState, Result};
use snaplingo_core::application::capture::{
    CaptureOcrEffects, CaptureSelectionMode, CaptureSession, CaptureWindowGeometry,
};
use snaplingo_core::application::providers::common::Provider;
use snaplingo_core::application::providers::ocr::OcrProvider;
use snaplingo_core::application::result_window::{ResultWindowMode, TranslationStatus};
use snaplingo_core::domain::capture::*;
use snaplingo_core::domain::config::ScreenshotSettings;
use snaplingo_core::domain::ocr::{OcrRequest, OcrResult};

#[path = "editor_tools.rs"]
mod editor_tools;

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

async fn until(_app: &NativeTestApp, condition: impl Fn() -> bool, message: &str) {
    let deadline = Instant::now() + Duration::from_secs(8);
    while !condition() {
        assert!(Instant::now() < deadline, "{message}");
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}
async fn open(app: &NativeTestApp, mode: &str) -> String {
    let previous = on_main(app, native::session_id).await;
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        // The runtime deliberately ignores reentry while OCR delivery/startup
        // owns its lock. Wait for that handoff before starting the next scenario.
        app.state
            .clone()
            .capture
            .runtime
            .open_capture_window_for_mode(mode)
            .await
            .unwrap();
        if let Some(id) = on_main(app, native::session_id)
            .await
            .filter(|id| Some(id) != previous.as_ref())
        {
            return id;
        }
        assert!(
            Instant::now() < deadline,
            "Native quick capture did not open: {mode}"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}
async fn at_candidate(app: &NativeTestApp, id: &str) {
    let id = id.to_owned();
    on_main(app, move || {
        pointer(&id, &fixture_bounds(), 500.0, 350.0, "move", Flags::empty())
    })
    .await;
}
async fn drag(app: &NativeTestApp, id: &str) {
    let id = id.to_owned();
    on_main(app, move || {
        pointer(&id, &fixture_bounds(), 120.0, 140.0, "down", Flags::empty());
        pointer(&id, &fixture_bounds(), 360.0, 300.0, "move", Flags::empty());
        pointer(&id, &fixture_bounds(), 360.0, 300.0, "up", Flags::empty());
    })
    .await;
}
async fn press(app: &NativeTestApp, id: &str, text: &'static str, flags: Flags, code: u16) {
    let id = id.to_owned();
    on_main(app, move || key(&id, text, flags, code)).await;
}
async fn released(app: &NativeTestApp, id: &str) {
    let state = app.state.clone();
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
    // Removing the application session precedes queued AppKit teardown. Wait
    // for the complete shutdown contract, not that intermediate state alone.
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        let released = on_main(app, || {
            native::test_decoded_images() == 0
                && !crate::window::capture::is_capture_presentation_active()
        })
        .await;
        if released {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "Native capture retained its UI resources after completion"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

pub(crate) async fn run(app: &NativeTestApp, directory: &Path, clipboard: &Mutex<Vec<u8>>) {
    on_main(app, controls).await;
    let state = app.state.clone();
    let services = app.capture_services.clone();
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
    println!("Native quick capture pixels, negative/Retina coordinates, selection, keys, history, copy/save/pin, OCR/translation, control detection, refresh and cleanup passed");
}

async fn wait_for_toolbar(
    app: &NativeTestApp,
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
    app: &NativeTestApp,
    ready: impl Fn(&snaplingo_core::application::capture::CaptureEditorStylePresentation) -> bool,
) -> snaplingo_core::application::capture::CaptureEditorStylePresentation {
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

async fn native_editor_state(
    app: &NativeTestApp,
    id: &str,
) -> crate::window::capture::macos::canvas::TestState {
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        let id = id.to_owned();
        if let Some(state) = on_main(app, move || native::test_editor_state(&id)).await {
            if state.ready {
                return state;
            }
        }
        assert!(
            Instant::now() < deadline,
            "Native annotation editor did not finish rendering"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

pub(crate) async fn editor_handoff(app: &NativeTestApp, clipboard: &Mutex<Vec<u8>>) {
    let state = app.state.clone();
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
    drag(app, &id).await;
    native_editor_state(app, &id).await;
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
    snapshot("capture-native-editor-controls.png", &bytes);
    let geometry = on_main(app, toolbar::test_geometry).await;
    snapshot(
        "capture-native-editor-controls.json",
        serde_json::to_string_pretty(&geometry).unwrap().as_bytes(),
    );
    on_main(app, || {
        toolbar::test_styles(|style| style.test_open_palette())
    })
    .await;
    assert!(
        on_main(app, || toolbar::test_styles(
            |style| style.test_palette_visible()
        ))
        .await
    );
    let bytes = on_main(app, || {
        toolbar::test_styles(|style| style.test_palette_snapshot())
    })
    .await;
    snapshot("capture-native-editor-colors.png", &bytes);
    let geometry = on_main(app, toolbar::test_geometry).await;
    snapshot(
        "capture-native-editor-colors.json",
        serde_json::to_string_pretty(&geometry).unwrap().as_bytes(),
    );
    press(app, &id, "", Flags::empty(), 53).await;
    assert!(
        !on_main(app, || toolbar::test_styles(
            |style| style.test_palette_visible()
        ))
        .await
    );

    assert!(state
        .capture
        .sessions
        .has_session(&CaptureSessionId(id.clone())));
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
            let family = wait_for_styles(app, |style| {
                style.text_mode && !style.font_families.is_empty()
            })
            .await
            .font_families[0]
                .clone();
            let pointer_id = id.clone();
            on_main(app, move || {
                pointer(
                    &pointer_id,
                    &fixture_bounds(),
                    160.0,
                    200.0,
                    "down",
                    Flags::empty(),
                );
                pointer(
                    &pointer_id,
                    &fixture_bounds(),
                    160.0,
                    200.0,
                    "up",
                    Flags::empty(),
                );
                native::test_editor_text(&pointer_id, "原生样式");
            })
            .await;
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
            let geometry = on_main(app, toolbar::test_geometry).await;
            snapshot(
                "capture-native-editor-text-controls.json",
                serde_json::to_string_pretty(&geometry).unwrap().as_bytes(),
            );
            let bytes = on_main(app, toolbar::test_snapshot).await;
            snapshot("capture-native-editor-text-controls.png", &bytes);
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
            let text_state = native_editor_state(app, &id).await;
            assert_eq!(text_state.text.as_deref(), Some("原生样式"));
            assert_eq!(text_state.styling.font_size, 36);
            assert!(text_state.styling.style.bold && text_state.styling.style.italic);
            assert_eq!(text_state.styling.style.font_family, selected_family);
            on_main(app, || {
                toolbar::test_styles(|style| style.close_color_panel())
            })
            .await;
            let text_id = id.clone();
            assert!(on_main(app, move || native::test_editor_commit_text(&text_id)).await);
            wait_for_toolbar(app, &id, Some("text"), true, false, true).await;
            let text_state = native_editor_state(app, &id).await;
            assert!(
                matches!(&text_state.annotations[0], AnnotationCommand::Text { text, font_size: 36, bold: true, italic: true, .. } if text == "原生样式")
            );
            let image_id = id.clone();
            let bytes = on_main(app, move || native::test_snapshot(&image_id)).await;
            snapshot("capture-native-editor-text.png", &bytes);
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
    let pointer_id = id.clone();
    on_main(app, move || {
        pointer(
            &pointer_id,
            &fixture_bounds(),
            150.0,
            170.0,
            "down",
            Flags::empty(),
        );
        pointer(
            &pointer_id,
            &fixture_bounds(),
            270.0,
            245.0,
            "move",
            Flags::empty(),
        );
        pointer(
            &pointer_id,
            &fixture_bounds(),
            270.0,
            245.0,
            "up",
            Flags::empty(),
        );
    })
    .await;
    wait_for_toolbar(app, &id, Some("rectangle"), true, false, true).await;
    let annotation = native_editor_state(app, &id).await;
    assert!(
        matches!(&annotation.annotations[0], AnnotationCommand::Rectangle { rect, stroke_width: 6, filled: true, corner_radius, .. }
        if rect.width == 120.0 && rect.height == 75.0 && *corner_radius == 12.0)
    );
    on_main(app, || toolbar::test_click("undo")).await;
    wait_for_toolbar(app, &id, Some("rectangle"), false, true, true).await;
    on_main(app, || toolbar::test_click("redo")).await;
    wait_for_toolbar(app, &id, Some("rectangle"), true, false, true).await;
    on_main(app, || toolbar::test_click("clear")).await;
    wait_for_toolbar(app, &id, Some("rectangle"), false, false, true).await;
    assert!(native_editor_state(app, &id).await.annotations.is_empty());
    assert_eq!(
        state
            .capture
            .runtime
            .render_png(&CaptureSessionId(id.clone()), &selected, &[], false)
            .unwrap(),
        expected
    );
    on_main(app, || {
        toolbar::test_styles(|style| style.test_open_color())
    })
    .await;
    on_main(app, || toolbar::test_click("copy")).await;
    released(app, &id).await;
    assert_eq!(*clipboard.lock().unwrap(), expected);
    assert!(on_main(app, toolbar::test_state).await.is_none());
    assert!(
        !on_main(app, || objc2_app_kit::NSColorPanel::sharedColorPanel(
            MainThreadMarker::new().unwrap()
        )
        .isVisible())
        .await
    );

    let id = open(app, "screenshot").await;
    drag(app, &id).await;
    native_editor_state(app, &id).await;
    on_main(app, || toolbar::test_click("rectangle")).await;
    let pointer_id = id.clone();
    on_main(app, move || {
        pointer(
            &pointer_id,
            &fixture_bounds(),
            150.0,
            170.0,
            "down",
            Flags::empty(),
        );
        pointer(
            &pointer_id,
            &fixture_bounds(),
            270.0,
            245.0,
            "move",
            Flags::empty(),
        );
        pointer(
            &pointer_id,
            &fixture_bounds(),
            270.0,
            245.0,
            "up",
            Flags::empty(),
        );
    })
    .await;
    let annotations = native_editor_state(app, &id).await.annotations;
    assert_eq!(annotations.len(), 1);
    let annotated = state
        .capture
        .runtime
        .render_png(
            &CaptureSessionId(id.clone()),
            &selected,
            &annotations,
            false,
        )
        .unwrap();
    on_main(app, || toolbar::test_click("copy")).await;
    released(app, &id).await;
    assert_eq!(
        *clipboard.lock().unwrap(),
        annotated,
        "Native output lost the Rust annotation state"
    );

    let id = open(app, "screenshot").await;
    drag(app, &id).await;
    native_editor_state(app, &id).await;
    on_main(app, || toolbar::test_click("select")).await;
    press(app, &id, "", Flags::empty(), 96).await;
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        if on_main(app, native::session_id)
            .await
            .is_some_and(|current| current != id)
        {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "Native editor refresh did not replace its frozen session"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    assert!(!state.capture.sessions.has_session(&CaptureSessionId(id)));
    assert!(on_main(app, toolbar::test_state).await.is_none());
    let id = on_main(app, native::session_id).await.unwrap();
    press(app, &id, "", Flags::empty(), 53).await;
    released(app, &id).await;
    editor_tools::run(app, clipboard).await;
    println!("Native annotation canvas, text input, style controls, undo/redo, frozen output, refresh and cleanup passed without WebViews");
}

pub(crate) async fn editor_output_recovery(
    app: &NativeTestApp,
    clipboard: &Mutex<Vec<u8>>,
    fail_copy: &AtomicBool,
    fail_scrolling_open: &AtomicBool,
) {
    let id = open(app, "screenshot").await;
    drag(app, &id).await;
    native_editor_state(app, &id).await;
    on_main(app, || {
        toolbar::test_click("select");
        toolbar::test_click("rectangle");
    })
    .await;
    editor_tools::draw_annotation(app, &id, (150.0, 170.0), (280.0, 230.0)).await;
    let editor = native_editor_state(app, &id).await;
    assert_eq!(editor.annotations.len(), 1);
    fail_scrolling_open.store(true, Ordering::SeqCst);
    on_main(app, || toolbar::test_click("scrolling")).await;
    wait_for_styles(app, |style| {
        style.error.as_deref() == Some("Synthetic scrolling window open failure")
    })
    .await;
    let restored = native_editor_state(app, &id).await;
    assert_eq!(restored.selection, editor.selection);
    assert_eq!(restored.annotations, editor.annotations);
    assert!(!restored.busy);
    assert!(
        on_main(app, {
            let id = id.clone();
            move || native::test_ready_size(&id).2
        })
        .await
    );
    println!("Failed scrolling window opening restored the native editor, annotations and frozen session without a WebView");
    let expected = app
        .state
        .capture
        .runtime
        .render_png(
            &CaptureSessionId(id.clone()),
            &editor.selection,
            &editor.annotations,
            false,
        )
        .unwrap();
    let before = clipboard.lock().unwrap().clone();
    fail_copy.store(true, Ordering::SeqCst);
    on_main(app, || toolbar::test_click("copy")).await;
    wait_for_styles(app, |style| {
        style.error.as_deref() == Some("Synthetic native editor copy failure")
    })
    .await;
    let recovered = native_editor_state(app, &id).await;
    assert!(!recovered.busy, "Failed native editor output remained busy");
    assert!(app
        .state
        .capture
        .sessions
        .has_session(&CaptureSessionId(id.clone())));
    assert_eq!(*clipboard.lock().unwrap(), before);
    on_main(app, || toolbar::test_click("copy")).await;
    released(app, &id).await;
    assert_eq!(*clipboard.lock().unwrap(), expected);
    println!("Native editor copy failure and retry retained frozen pixels and session ownership");
}

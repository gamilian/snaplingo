// Exercise the production native selection reader with AppKit text and the real Rust canvas.
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
#[path = "../src/infrastructure/mod.rs"]
mod infrastructure;
#[cfg(target_os = "macos")]
#[path = "../src/startup_shortcuts.rs"]
mod startup_shortcuts;
#[cfg(target_os = "macos")]
#[path = "../src/window/mod.rs"]
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
mod native_selection {
    include!("../src/infrastructure/system/selection/macos/self_webview.rs");

    pub(super) fn read_window(
        window: &objc2_app_kit::NSWindow,
    ) -> tokio::sync::oneshot::Receiver<std::result::Result<String, String>> {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        read_window_selection(window, sender);
        receiver
    }
}

#[cfg(target_os = "macos")]
fn main() {
    use std::cell::RefCell;
    use std::sync::mpsc;
    use std::time::Duration;

    use crate::infrastructure::system::settings_window::SharedFeatureView;
    use objc2::{rc::Retained, MainThreadMarker};
    use objc2_app_kit::{
        NSApplication, NSBackingStoreType, NSEvent, NSEventModifierFlags, NSEventType, NSTextView,
        NSView, NSWindow, NSWindowStyleMask,
    };
    use objc2_foundation::{NSPoint, NSRange, NSRect, NSSize, NSString};
    use snaplingo_ui::{
        library::{LibraryInput, LibrarySnapshot, LibraryTarget},
        Event, Page, Snapshot, WindowInput, WindowSnapshot,
    };

    thread_local! {
        static WINDOW: RefCell<Option<Retained<NSWindow>>> = const { RefCell::new(None) };
    }
    let mtm = MainThreadMarker::new().unwrap();
    let app = NSApplication::sharedApplication(mtm);
    let bounds = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(900.0, 650.0));
    let window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            mtm.alloc(),
            bounds,
            NSWindowStyleMask::Titled,
            NSBackingStoreType::Buffered,
            false,
        )
    };
    unsafe { window.setReleasedWhenClosed(false) };
    let text = NSTextView::initWithFrame(mtm.alloc(), bounds);
    text.setString(&NSString::from_str("prefix 中文🦀 \"quoted\"\nend"));
    window.setContentView(Some(&text));
    assert!(window.makeFirstResponder(Some(&text)));
    text.setSelectedRange(NSRange::new(7, 4));
    let native_text = native_selection::read_window(&window);
    text.setSelectedRange(NSRange::new(0, 0));
    let empty_text = native_selection::read_window(&window);
    text.setSelectedRange(NSRange::new(7, 17));
    let multiline_text = native_selection::read_window(&window);

    let canvas = SharedFeatureView::new(mtm, Page::History, Snapshot::default());
    canvas.refresh_window(WindowSnapshot::Library(LibrarySnapshot::default()));
    canvas.setFrame(bounds);
    window.setContentView(Some(&canvas));
    assert!(window.makeFirstResponder(Some(&canvas)));
    canvas.window_input(WindowInput::Library(LibraryInput::Focus(
        LibraryTarget::Search,
    )));
    canvas.input(Event::TextInput("prefix 中文🦀 \"quoted\"".into()));
    canvas.input(Event::TextSelection(7..17));
    let canvas_text = native_selection::read_window(&window);
    canvas.input(Event::ImePreedit {
        text: "候補🙂".into(),
        selection: Some(0..10),
    });
    let composing_text = native_selection::read_window(&window);
    canvas.input(Event::ImeDisabled);
    canvas.input(Event::TextSelection(0..0));
    let empty_canvas = native_selection::read_window(&window);
    canvas.window_input(WindowInput::Library(LibraryInput::FocusLost));
    let inactive_editor = native_selection::read_window(&window);
    window.makeFirstResponder(None);
    let unsupported = native_selection::read_window(&window);
    WINDOW.with_borrow_mut(|slot| *slot = Some(window));

    let (done, finished) = mpsc::channel();
    std::thread::spawn(move || {
        if finished.recv_timeout(Duration::from_secs(20)).is_err() {
            eprintln!("Native selected-text verification failed or stalled");
            std::process::exit(1);
        }
    });
    let worker = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async move {
            assert_eq!(native_text.await.unwrap().unwrap(), "中文🦀");
            assert_eq!(empty_text.await.unwrap().unwrap(), "");
            assert_eq!(multiline_text.await.unwrap().unwrap(), "中文🦀 \"quoted\"\nend");
            assert_eq!(canvas_text.await.unwrap().unwrap(), "中文🦀");
            assert_eq!(composing_text.await.unwrap().unwrap(), "候補🙂");
            assert_eq!(empty_canvas.await.unwrap().unwrap(), "");
            assert_eq!(inactive_editor.await.unwrap().unwrap(), "");
            assert!(unsupported.await.unwrap().is_err());
            infrastructure::ui_thread::run("release selection fixture", || {
                WINDOW.with_borrow_mut(|window| window.take().unwrap().close());
                Ok(())
            }).await.unwrap();
            assert!(native_selection::read_focused_window_selection().await.is_err());
            infrastructure::ui_thread::dispatch(move || {
                let app = NSApplication::sharedApplication(MainThreadMarker::new().unwrap());
                app.stop(None);
                let wake = NSEvent::otherEventWithType_location_modifierFlags_timestamp_windowNumber_context_subtype_data1_data2(
                    NSEventType::ApplicationDefined, NSPoint::new(0.0, 0.0), NSEventModifierFlags::empty(),
                    0.0, 0, None, 0, 0, 0,
                ).unwrap();
                app.postEvent_atStart(&wake, true);
                done.send(()).unwrap();
            });
        });
    });
    app.run();
    worker.join().unwrap();
    println!("Native selection: NSTextView UTF-16, real Rust canvas selection and IME, empty/inactive editor, quotes/newlines, unsupported responder and missing focus passed");
}

#[cfg(not(target_os = "macos"))]
fn main() {}

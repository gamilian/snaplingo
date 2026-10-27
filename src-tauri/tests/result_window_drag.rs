// AppKit requires the process main thread, so this native regression uses its own harness.
#[cfg(target_os = "macos")]
#[path = "../src/infrastructure/system/result_window/drag_macos.rs"]
mod drag_macos;

#[cfg(target_os = "macos")]
fn main() {
    use std::cell::Cell;

    use objc2::{
        runtime::{AnyClass, Sel},
        sel, ClassType, MainThreadMarker, MainThreadOnly,
    };
    use objc2_app_kit::{
        NSApplication, NSBackingStoreType, NSEvent, NSEventModifierFlags, NSEventType, NSWindow,
        NSWindowStyleMask,
    };
    use objc2_foundation::{NSPoint, NSRect, NSSize};

    thread_local! {
        static CURSOR: Cell<NSPoint> = const { Cell::new(NSPoint::new(0.0, 0.0)) };
        static BUTTONS: Cell<usize> = const { Cell::new(1) };
        static CURRENT_EVENT: Cell<*const NSEvent> = const { Cell::new(std::ptr::null()) };
        static CAPTURED: Cell<Option<(NSPoint, isize, NSEventType)>> = const { Cell::new(None) };
    }

    unsafe extern "C-unwind" fn current_event(_: &NSApplication, _: Sel) -> *const NSEvent {
        CURRENT_EVENT.with(Cell::get)
    }

    unsafe extern "C-unwind" fn mouse_location(_: &AnyClass, _: Sel) -> NSPoint {
        CURSOR.with(Cell::get)
    }

    unsafe extern "C-unwind" fn pressed_buttons(_: &AnyClass, _: Sel) -> usize {
        BUTTONS.with(Cell::get)
    }

    unsafe extern "C-unwind" fn capture_drag(_: &NSWindow, _: Sel, event: &NSEvent) {
        CAPTURED.with(|captured| {
            captured.set(Some((
                event.locationInWindow(),
                event.windowNumber(),
                event.r#type(),
            )));
        });
    }

    let main_thread =
        MainThreadMarker::new().expect("native drag test must run on the main thread");
    let _app = NSApplication::sharedApplication(main_thread);
    let wake_up = NSEvent::otherEventWithType_location_modifierFlags_timestamp_windowNumber_context_subtype_data1_data2(
        NSEventType::ApplicationDefined,
        NSPoint::new(0.0, 0.0),
        NSEventModifierFlags::empty(),
        1.0,
        0,
        None,
        0,
        0,
        0,
    ).unwrap();
    CURRENT_EVENT.with(|event| event.set(&*wake_up));

    // Intercept dragging inside this test process: no mouse events are posted and no window moves.
    unsafe {
        NSApplication::class()
            .instance_method(sel!(currentEvent))
            .unwrap()
            .set_implementation(std::mem::transmute(current_event as *const ()));
        NSEvent::class()
            .class_method(sel!(mouseLocation))
            .unwrap()
            .set_implementation(std::mem::transmute(mouse_location as *const ()));
        NSEvent::class()
            .class_method(sel!(pressedMouseButtons))
            .unwrap()
            .set_implementation(std::mem::transmute(pressed_buttons as *const ()));
        NSWindow::class()
            .instance_method(sel!(performWindowDragWithEvent:))
            .unwrap()
            .set_implementation(std::mem::transmute(capture_drag as *const ()));
    }

    for (origin, anchor) in [
        (NSPoint::new(400.0, 240.0), NSPoint::new(180.0, 440.0)),
        (NSPoint::new(-1440.0, 120.0), NSPoint::new(430.5, 300.5)),
        (NSPoint::new(0.0, -900.0), NSPoint::new(20.0, 460.0)),
    ] {
        let window = unsafe {
            let window = NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(main_thread),
                NSRect::new(origin, NSSize::new(660.0, 480.0)),
                NSWindowStyleMask::Borderless,
                NSBackingStoreType::Buffered,
                false,
            );
            window.setReleasedWhenClosed(false);
            window
        };
        CURSOR.with(|cursor| cursor.set(window.convertPointToScreen(anchor)));
        CAPTURED.with(|captured| captured.set(None));

        drag_macos::start_dragging(&window).unwrap();

        assert_eq!(
            CAPTURED.with(Cell::get),
            Some((anchor, window.windowNumber(), NSEventType::LeftMouseDown)),
            "dragging must preserve the pointer's local anchor and target window at {origin:?}",
        );

        BUTTONS.with(|buttons| buttons.set(0));
        CAPTURED.with(|captured| captured.set(None));
        drag_macos::start_dragging(&window).unwrap();
        assert_eq!(
            CAPTURED.with(Cell::get),
            None,
            "ignore an already released mouse press"
        );
        BUTTONS.with(|buttons| buttons.set(1));
    }

    println!(
        "Native result window dragging: anchors and released presses passed at 3 window origins"
    );
}

#[cfg(not(target_os = "macos"))]
fn main() {}

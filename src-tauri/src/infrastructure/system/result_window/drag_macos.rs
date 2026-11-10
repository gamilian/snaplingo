use objc2_app_kit::{NSEvent, NSEventType, NSWindow};
use objc2_foundation::NSProcessInfo;

pub(super) fn start_dragging(window: &NSWindow) -> Result<(), String> {
    if NSEvent::pressedMouseButtons() & 1 == 0 {
        return Ok(());
    }

    // IPC can replace NSApp.currentEvent with an application-defined wake-up.
    // AppKit's drag anchor must belong to this window and use window coordinates.
    let event = NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure(
        NSEventType::LeftMouseDown,
        window.convertPointFromScreen(NSEvent::mouseLocation()),
        NSEvent::modifierFlags_class(),
        NSProcessInfo::processInfo().systemUptime(),
        window.windowNumber(),
        None,
        0,
        1,
        1.0,
    )
    .ok_or("Failed to create result window drag event")?;
    window.performWindowDragWithEvent(&event);
    Ok(())
}

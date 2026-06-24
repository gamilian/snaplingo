use std::sync::atomic::{AtomicUsize, Ordering};

use objc2_app_kit::{
    NSScreenSaverWindowLevel, NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask,
};
use tauri::{AppHandle, WebviewWindow};

static CAPTURE_PRESENTATION_DEPTH: AtomicUsize = AtomicUsize::new(0);

pub(super) fn begin_capture_presentation(app: &AppHandle) -> Result<(), String> {
    let previous_depth = CAPTURE_PRESENTATION_DEPTH.fetch_add(1, Ordering::SeqCst);

    if previous_depth == 0 {
        if let Err(err) = app.set_activation_policy(tauri::ActivationPolicy::Accessory) {
            CAPTURE_PRESENTATION_DEPTH.fetch_sub(1, Ordering::SeqCst);
            return Err(err.to_string());
        }
    }

    Ok(())
}

pub(super) fn end_capture_presentation(app: &AppHandle) -> Result<(), String> {
    let previous_depth = decrement_capture_presentation_depth();

    if previous_depth == 1 {
        app.set_activation_policy(tauri::ActivationPolicy::Regular)
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

pub(super) fn configure_capture_window_for_current_space(
    window: &WebviewWindow,
) -> Result<(), String> {
    let ns_window = window.ns_window().map_err(|e| e.to_string())?;
    if ns_window.is_null() {
        return Err("Capture window has no native NSWindow".to_string());
    }

    let ns_window: &NSWindow = unsafe { &*ns_window.cast() };
    ns_window.setStyleMask(capture_overlay_style_mask(ns_window.styleMask()));
    ns_window.setCollectionBehavior(capture_overlay_collection_behavior(
        ns_window.collectionBehavior(),
    ));
    ns_window.setLevel(NSScreenSaverWindowLevel);
    ns_window.setCanHide(false);
    ns_window.setHidesOnDeactivate(false);

    Ok(())
}

pub(super) fn reveal_capture_window_for_current_space(
    window: &WebviewWindow,
) -> Result<(), String> {
    configure_capture_window_for_current_space(window)?;

    let ns_window = window.ns_window().map_err(|e| e.to_string())?;
    if ns_window.is_null() {
        return Err("Capture window has no native NSWindow".to_string());
    }

    let ns_window: &NSWindow = unsafe { &*ns_window.cast() };
    ns_window.orderFrontRegardless();

    Ok(())
}

fn decrement_capture_presentation_depth() -> usize {
    let mut current_depth = CAPTURE_PRESENTATION_DEPTH.load(Ordering::SeqCst);

    loop {
        if current_depth == 0 {
            return 0;
        }

        match CAPTURE_PRESENTATION_DEPTH.compare_exchange(
            current_depth,
            current_depth - 1,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(previous_depth) => return previous_depth,
            Err(next_depth) => current_depth = next_depth,
        }
    }
}

fn capture_overlay_collection_behavior(
    base: NSWindowCollectionBehavior,
) -> NSWindowCollectionBehavior {
    base | NSWindowCollectionBehavior::CanJoinAllSpaces
        | NSWindowCollectionBehavior::FullScreenAuxiliary
        | NSWindowCollectionBehavior::Stationary
        | NSWindowCollectionBehavior::Transient
        | NSWindowCollectionBehavior::IgnoresCycle
}

fn capture_overlay_style_mask(base: NSWindowStyleMask) -> NSWindowStyleMask {
    base | NSWindowStyleMask::NonactivatingPanel
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_overlay_joins_fullscreen_spaces() {
        let behavior = capture_overlay_collection_behavior(NSWindowCollectionBehavior::Default);

        assert!(behavior.contains(NSWindowCollectionBehavior::CanJoinAllSpaces));
        assert!(behavior.contains(NSWindowCollectionBehavior::FullScreenAuxiliary));
        assert!(behavior.contains(NSWindowCollectionBehavior::Stationary));
        assert!(behavior.contains(NSWindowCollectionBehavior::Transient));
        assert!(behavior.contains(NSWindowCollectionBehavior::IgnoresCycle));
    }

    #[test]
    fn capture_overlay_preserves_existing_collection_behavior() {
        let behavior = capture_overlay_collection_behavior(NSWindowCollectionBehavior::Stationary);

        assert!(behavior.contains(NSWindowCollectionBehavior::Stationary));
        assert!(behavior.contains(NSWindowCollectionBehavior::FullScreenAuxiliary));
    }

    #[test]
    fn capture_overlay_uses_nonactivating_panel_style() {
        let style = capture_overlay_style_mask(NSWindowStyleMask::Borderless);

        assert!(style.contains(NSWindowStyleMask::Borderless));
        assert!(style.contains(NSWindowStyleMask::NonactivatingPanel));
    }
}

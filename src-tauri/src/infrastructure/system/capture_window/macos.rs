use objc2_app_kit::{NSScreenSaverWindowLevel, NSWindow, NSWindowCollectionBehavior};
use tauri::WebviewWindow;

pub(super) fn configure_capture_window_for_current_space(
    window: &WebviewWindow,
) -> Result<(), String> {
    let ns_window = window.ns_window().map_err(|e| e.to_string())?;
    if ns_window.is_null() {
        return Err("Capture window has no native NSWindow".to_string());
    }

    let ns_window: &NSWindow = unsafe { &*ns_window.cast() };
    ns_window.setCollectionBehavior(capture_overlay_collection_behavior(
        ns_window.collectionBehavior(),
    ));
    ns_window.setLevel(NSScreenSaverWindowLevel);

    Ok(())
}

fn capture_overlay_collection_behavior(
    base: NSWindowCollectionBehavior,
) -> NSWindowCollectionBehavior {
    base | NSWindowCollectionBehavior::CanJoinAllSpaces
        | NSWindowCollectionBehavior::FullScreenAuxiliary
        | NSWindowCollectionBehavior::Transient
        | NSWindowCollectionBehavior::IgnoresCycle
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_overlay_joins_fullscreen_spaces() {
        let behavior = capture_overlay_collection_behavior(NSWindowCollectionBehavior::Default);

        assert!(behavior.contains(NSWindowCollectionBehavior::CanJoinAllSpaces));
        assert!(behavior.contains(NSWindowCollectionBehavior::FullScreenAuxiliary));
        assert!(behavior.contains(NSWindowCollectionBehavior::Transient));
        assert!(behavior.contains(NSWindowCollectionBehavior::IgnoresCycle));
    }

    #[test]
    fn capture_overlay_preserves_existing_collection_behavior() {
        let behavior = capture_overlay_collection_behavior(NSWindowCollectionBehavior::Stationary);

        assert!(behavior.contains(NSWindowCollectionBehavior::Stationary));
        assert!(behavior.contains(NSWindowCollectionBehavior::FullScreenAuxiliary));
    }
}

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering};

use objc2_app_kit::{
    NSApplicationActivationOptions, NSRunningApplication, NSScreenSaverWindowLevel, NSWindow,
    NSWindowAnimationBehavior, NSWindowCollectionBehavior, NSWindowStyleMask, NSWorkspace,
};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

use crate::infrastructure::system::shortcut;

use super::backend::CAPTURE_WINDOW_LABEL;

static CAPTURE_PRESENTATION_DEPTH: AtomicUsize = AtomicUsize::new(0);
static CAPTURE_WINDOW_ACTIVATION_SUPPRESSED: AtomicBool = AtomicBool::new(false);
static CAPTURE_CANCEL_SHORTCUT_REGISTERED: AtomicBool = AtomicBool::new(false);
static PREVIOUS_FRONTMOST_APP_PID: AtomicI32 = AtomicI32::new(NO_PREVIOUS_FRONTMOST_APP_PID);

const NO_PREVIOUS_FRONTMOST_APP_PID: i32 = -1;
const CAPTURE_CANCEL_SHORTCUT_ACCELERATOR: &str = "Escape";
const CAPTURE_CANCEL_REQUESTED_EVENT: &str = "capture-cancel-requested";

pub(super) fn begin_capture_presentation(app: &AppHandle) -> Result<(), String> {
    let previous_depth = CAPTURE_PRESENTATION_DEPTH.fetch_add(1, Ordering::SeqCst);

    if previous_depth == 0 {
        remember_previous_frontmost_application();
        if let Err(err) = register_capture_cancel_shortcut(app) {
            log::warn!("Failed to register capture cancel shortcut: {}", err);
        }
    }

    let _ = app;
    Ok(())
}

pub(super) fn end_capture_presentation(app: &AppHandle) -> Result<(), String> {
    let previous_depth = decrement_capture_presentation_depth();

    if previous_depth == 1 {
        let activation_suppressed = take_capture_window_activation_suppressed();
        unregister_capture_cancel_shortcut(app);
        restore_previous_frontmost_application();

        if activation_suppressed {
            app.set_activation_policy(tauri::ActivationPolicy::Regular)
                .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

pub(super) fn suppress_capture_window_activation(app: &AppHandle) -> Result<(), String> {
    if let Some(activation_policy) = capture_presentation_activation_policy(app) {
        app.set_activation_policy(activation_policy)
            .map_err(|e| e.to_string())?;
        mark_capture_window_activation_suppressed();
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
    ns_window.setAcceptsMouseMovedEvents(capture_overlay_accepts_mouse_moved_events());
    if capture_overlay_disables_window_animation() {
        ns_window.setAnimationBehavior(NSWindowAnimationBehavior::None);
    }

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
    if should_make_capture_overlay_key_on_reveal() {
        ns_window.makeKeyAndOrderFront(None);
    }
    ns_window.orderFrontRegardless();

    Ok(())
}

pub(super) fn hide_capture_window_for_current_space(window: &WebviewWindow) -> Result<(), String> {
    let ns_window = window.ns_window().map_err(|e| e.to_string())?;
    if ns_window.is_null() {
        return Err("Capture window has no native NSWindow".to_string());
    }

    let ns_window: &NSWindow = unsafe { &*ns_window.cast() };
    ns_window.orderOut(None);

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
    base
}

fn capture_presentation_activation_policy(app: &AppHandle) -> Option<tauri::ActivationPolicy> {
    capture_presentation_activation_policy_for_state(
        current_application_pid(),
        frontmost_application_pid(),
        has_active_space_capture_subject_app_window(app),
    )
}

fn capture_presentation_activation_policy_for_state(
    current_pid: i32,
    frontmost_pid: Option<i32>,
    has_active_space_app_window: bool,
) -> Option<tauri::ActivationPolicy> {
    if has_active_space_app_window {
        return None;
    }

    capture_presentation_activation_policy_for_frontmost(current_pid, frontmost_pid)
}

fn capture_presentation_activation_policy_for_frontmost(
    current_pid: i32,
    frontmost_pid: Option<i32>,
) -> Option<tauri::ActivationPolicy> {
    match frontmost_pid {
        Some(pid) if pid == current_pid => None,
        _ => Some(tauri::ActivationPolicy::Prohibited),
    }
}

fn has_active_space_capture_subject_app_window(app: &AppHandle) -> bool {
    app.webview_windows().into_iter().any(|(label, window)| {
        label != CAPTURE_WINDOW_LABEL && is_capture_subject_window_on_active_space(&window)
    })
}

fn is_capture_subject_window_on_active_space(window: &WebviewWindow) -> bool {
    let Ok(ns_window) = window.ns_window() else {
        return false;
    };
    if ns_window.is_null() {
        return false;
    }

    let ns_window: &NSWindow = unsafe { &*ns_window.cast() };
    capture_subject_window_is_on_active_space_for_state(
        ns_window.isVisible(),
        ns_window.isOnActiveSpace(),
    )
}

fn capture_subject_window_is_on_active_space_for_state(
    is_visible: bool,
    is_on_active_space: bool,
) -> bool {
    is_visible && is_on_active_space
}

fn mark_capture_window_activation_suppressed() {
    CAPTURE_WINDOW_ACTIVATION_SUPPRESSED.store(true, Ordering::SeqCst);
}

fn take_capture_window_activation_suppressed() -> bool {
    CAPTURE_WINDOW_ACTIVATION_SUPPRESSED.swap(false, Ordering::SeqCst)
}

fn capture_overlay_accepts_mouse_moved_events() -> bool {
    true
}

fn should_make_capture_overlay_key_on_reveal() -> bool {
    should_make_capture_overlay_key_on_reveal_for_activation_suppressed(
        CAPTURE_WINDOW_ACTIVATION_SUPPRESSED.load(Ordering::SeqCst),
    )
}

fn should_make_capture_overlay_key_on_reveal_for_activation_suppressed(
    activation_suppressed: bool,
) -> bool {
    !activation_suppressed
}

fn capture_cancel_shortcut_accelerator() -> &'static str {
    CAPTURE_CANCEL_SHORTCUT_ACCELERATOR
}

fn capture_cancel_requested_event_name() -> &'static str {
    CAPTURE_CANCEL_REQUESTED_EVENT
}

fn capture_overlay_disables_window_animation() -> bool {
    true
}

fn register_capture_cancel_shortcut(app: &AppHandle) -> Result<(), String> {
    if CAPTURE_CANCEL_SHORTCUT_REGISTERED.swap(true, Ordering::SeqCst) {
        return Ok(());
    }

    let app_clone = app.clone();
    shortcut::register_shortcut(app, capture_cancel_shortcut_accelerator(), move || {
        emit_capture_cancel_requested(&app_clone);
    })
    .map_err(|e| {
        CAPTURE_CANCEL_SHORTCUT_REGISTERED.store(false, Ordering::SeqCst);
        e.to_string()
    })
}

fn unregister_capture_cancel_shortcut(app: &AppHandle) {
    if !CAPTURE_CANCEL_SHORTCUT_REGISTERED.swap(false, Ordering::SeqCst) {
        return;
    }

    if let Err(err) = shortcut::unregister_shortcut(app, capture_cancel_shortcut_accelerator()) {
        log::warn!("Failed to unregister capture cancel shortcut: {}", err);
    }
}

fn emit_capture_cancel_requested(app: &AppHandle) {
    let Some(window) = app.get_webview_window(CAPTURE_WINDOW_LABEL) else {
        return;
    };

    if let Err(err) = window.emit(capture_cancel_requested_event_name(), ()) {
        log::warn!("Failed to emit capture cancel request: {}", err);
    }
}

fn remember_previous_frontmost_application() {
    let previous_pid = frontmost_application_pid().and_then(|frontmost_pid| {
        previous_frontmost_pid_to_restore(Some(frontmost_pid), current_application_pid())
    });
    PREVIOUS_FRONTMOST_APP_PID.store(
        previous_pid.unwrap_or(NO_PREVIOUS_FRONTMOST_APP_PID),
        Ordering::SeqCst,
    );
}

fn restore_previous_frontmost_application() {
    let pid = PREVIOUS_FRONTMOST_APP_PID.swap(NO_PREVIOUS_FRONTMOST_APP_PID, Ordering::SeqCst);
    if !should_restore_previous_frontmost_app(
        Some(pid),
        current_application_pid(),
        frontmost_application_pid(),
    ) {
        return;
    }

    if let Some(application) = NSRunningApplication::runningApplicationWithProcessIdentifier(pid) {
        application.activateWithOptions(NSApplicationActivationOptions::empty());
    }
}

fn should_restore_previous_frontmost_app(
    previous_pid: Option<i32>,
    current_pid: i32,
    current_frontmost_pid: Option<i32>,
) -> bool {
    match (previous_pid, current_frontmost_pid) {
        (Some(previous), Some(frontmost)) if previous > 0 && frontmost == current_pid => true,
        _ => false,
    }
}

fn frontmost_application_pid() -> Option<i32> {
    let workspace = NSWorkspace::sharedWorkspace();
    let frontmost_application = workspace.frontmostApplication()?;
    let pid = frontmost_application.processIdentifier();
    i32::try_from(pid).ok()
}

fn current_application_pid() -> i32 {
    i32::try_from(NSRunningApplication::currentApplication().processIdentifier())
        .unwrap_or(NO_PREVIOUS_FRONTMOST_APP_PID)
}

fn previous_frontmost_pid_to_restore(frontmost_pid: Option<i32>, current_pid: i32) -> Option<i32> {
    match frontmost_pid {
        Some(pid) if pid > 0 && pid != current_pid => Some(pid),
        _ => None,
    }
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
    fn capture_overlay_does_not_add_unsupported_nonactivating_panel_style() {
        let style = capture_overlay_style_mask(NSWindowStyleMask::Borderless);

        assert!(style.contains(NSWindowStyleMask::Borderless));
        assert!(!style.contains(NSWindowStyleMask::NonactivatingPanel));
    }

    #[test]
    fn capture_overlay_preserves_existing_style_mask() {
        let style = capture_overlay_style_mask(
            NSWindowStyleMask::Borderless | NSWindowStyleMask::Resizable,
        );

        assert!(style.contains(NSWindowStyleMask::Borderless));
        assert!(style.contains(NSWindowStyleMask::Resizable));
    }

    #[test]
    fn capture_presentation_suppresses_activation_for_other_frontmost_apps() {
        assert!(matches!(
            capture_presentation_activation_policy_for_frontmost(9000, Some(4242)),
            Some(tauri::ActivationPolicy::Prohibited)
        ));
    }

    #[test]
    fn capture_presentation_keeps_activation_when_snaplingo_is_frontmost() {
        assert!(matches!(
            capture_presentation_activation_policy_for_frontmost(9000, Some(9000)),
            None
        ));
    }

    #[test]
    fn capture_presentation_keeps_activation_for_active_space_snaplingo_windows() {
        assert!(matches!(
            capture_presentation_activation_policy_for_state(9000, Some(4242), true),
            None
        ));
    }

    #[test]
    fn capture_presentation_keeps_activation_when_snaplingo_is_frontmost_with_active_window() {
        assert!(matches!(
            capture_presentation_activation_policy_for_state(9000, Some(9000), true),
            None
        ));
    }

    #[test]
    fn capture_presentation_suppresses_activation_without_active_space_snaplingo_windows() {
        assert!(matches!(
            capture_presentation_activation_policy_for_state(9000, Some(4242), false),
            Some(tauri::ActivationPolicy::Prohibited)
        ));
    }

    #[test]
    fn capture_subject_window_must_be_visible_on_active_space() {
        assert!(capture_subject_window_is_on_active_space_for_state(
            true, true
        ));
        assert!(!capture_subject_window_is_on_active_space_for_state(
            true, false
        ));
        assert!(!capture_subject_window_is_on_active_space_for_state(
            false, true
        ));
    }

    #[test]
    fn capture_activation_policy_restores_only_after_suppression() {
        assert!(!take_capture_window_activation_suppressed());

        mark_capture_window_activation_suppressed();
        assert!(take_capture_window_activation_suppressed());
        assert!(!take_capture_window_activation_suppressed());
    }

    #[test]
    fn capture_overlay_accepts_mouse_tracking_before_first_click() {
        assert!(capture_overlay_accepts_mouse_moved_events());
    }

    #[test]
    fn capture_overlay_becomes_key_when_activation_is_not_suppressed() {
        assert!(should_make_capture_overlay_key_on_reveal_for_activation_suppressed(false));
    }

    #[test]
    fn capture_overlay_does_not_become_key_when_activation_is_suppressed() {
        assert!(!should_make_capture_overlay_key_on_reveal_for_activation_suppressed(true));
    }

    #[test]
    fn capture_overlay_disables_appkit_window_animation() {
        assert!(capture_overlay_disables_window_animation());
    }

    #[test]
    fn capture_cancel_shortcut_uses_escape() {
        assert_eq!(capture_cancel_shortcut_accelerator(), "Escape");
    }

    #[test]
    fn capture_cancel_event_matches_frontend_listener() {
        assert_eq!(
            capture_cancel_requested_event_name(),
            "capture-cancel-requested"
        );
    }

    #[test]
    fn remembers_previous_frontmost_app_only_when_it_is_not_snaplingo() {
        assert_eq!(
            previous_frontmost_pid_to_restore(Some(4242), 9000),
            Some(4242)
        );
        assert_eq!(previous_frontmost_pid_to_restore(Some(9000), 9000), None);
        assert_eq!(previous_frontmost_pid_to_restore(None, 9000), None);
        assert_eq!(previous_frontmost_pid_to_restore(Some(-1), 9000), None);
    }

    #[test]
    fn restores_previous_frontmost_app_only_if_snaplingo_became_frontmost() {
        assert!(should_restore_previous_frontmost_app(
            Some(4242),
            9000,
            Some(9000),
        ));
        assert!(!should_restore_previous_frontmost_app(
            Some(4242),
            9000,
            Some(4242),
        ));
        assert!(!should_restore_previous_frontmost_app(
            Some(4242),
            9000,
            Some(7777),
        ));
        assert!(!should_restore_previous_frontmost_app(
            None,
            9000,
            Some(9000),
        ));
        assert!(!should_restore_previous_frontmost_app(
            Some(-1),
            9000,
            Some(9000),
        ));
    }
}

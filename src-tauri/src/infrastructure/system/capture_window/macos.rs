use std::sync::atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering};

use objc2_app_kit::{
    NSApplicationActivationOptions, NSCursor, NSRunningApplication, NSScreenSaverWindowLevel,
    NSView, NSWindow, NSWindowAnimationBehavior, NSWindowCollectionBehavior, NSWindowStyleMask,
    NSWorkspace,
};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

use crate::infrastructure::system::shortcut;

use super::backend::CAPTURE_WINDOW_LABEL;

static CAPTURE_PRESENTATION_DEPTH: AtomicUsize = AtomicUsize::new(0);
static CAPTURE_WINDOW_ACTIVATION_SUPPRESSED: AtomicBool = AtomicBool::new(false);
static CAPTURE_CANCEL_SHORTCUT_REGISTERED: AtomicBool = AtomicBool::new(false);
static CAPTURE_COPY_SHORTCUT_REGISTERED: AtomicBool = AtomicBool::new(false);
static CAPTURE_CROSSHAIR_CURSOR_PUSHED: AtomicBool = AtomicBool::new(false);
static PREVIOUS_FRONTMOST_APP_PID: AtomicI32 = AtomicI32::new(NO_PREVIOUS_FRONTMOST_APP_PID);

const NO_PREVIOUS_FRONTMOST_APP_PID: i32 = -1;
const CAPTURE_CANCEL_SHORTCUT_ACCELERATOR: &str = "Escape";
const CAPTURE_COPY_SHORTCUT_ACCELERATOR: &str = "CmdOrCtrl+KeyC";
const CAPTURE_CANCEL_REQUESTED_EVENT: &str = "capture-cancel-requested";
const CAPTURE_COPY_REQUESTED_EVENT: &str = "capture-copy-requested";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RestorePreviousFrontmostDisposition {
    Keep,
    Clear,
}

pub(super) fn begin_capture_presentation(app: &AppHandle) -> Result<(), String> {
    let previous_depth = CAPTURE_PRESENTATION_DEPTH.fetch_add(1, Ordering::SeqCst);

    if previous_depth == 0 {
        remember_previous_frontmost_application();
        if let Some(activation_policy) = capture_presentation_activation_policy() {
            if let Err(err) = app.set_activation_policy(activation_policy) {
                CAPTURE_PRESENTATION_DEPTH.fetch_sub(1, Ordering::SeqCst);
                return Err(err.to_string());
            }
        }
        if let Err(err) = register_capture_cancel_shortcut(app) {
            log::warn!("Failed to register capture cancel shortcut: {}", err);
        }
        if let Err(err) = register_capture_copy_shortcut(app) {
            log::warn!("Failed to register capture copy shortcut: {}", err);
        }
    }

    Ok(())
}

pub(super) fn end_capture_presentation(app: &AppHandle) -> Result<(), String> {
    let previous_depth = decrement_capture_presentation_depth();

    if previous_depth == 1 {
        let activation_suppressed = take_capture_window_activation_suppressed();
        restore_native_crosshair_cursor();
        unregister_capture_cancel_shortcut(app);
        unregister_capture_copy_shortcut(app);
        let activation_policy_result = if activation_suppressed {
            crate::app_shell::apply_resting_activation_policy(app)
        } else {
            Ok(())
        };
        restore_previous_frontmost_application(RestorePreviousFrontmostDisposition::Clear);
        activation_policy_result?;
    }

    Ok(())
}

pub(super) fn is_capture_presentation_active() -> bool {
    CAPTURE_PRESENTATION_DEPTH.load(Ordering::SeqCst) > 0
}

pub(super) fn suppress_capture_window_activation(app: &AppHandle) -> Result<(), String> {
    if let Some(activation_policy) = capture_window_activation_suppression_policy(app) {
        app.set_activation_policy(activation_policy)
            .map_err(|e| e.to_string())?;
        mark_capture_window_activation_suppressed();
    }

    Ok(())
}

pub(super) fn restore_capture_window_activation() {
    if !should_restore_capture_window_activation(
        CAPTURE_WINDOW_ACTIVATION_SUPPRESSED.load(Ordering::SeqCst),
        Some(PREVIOUS_FRONTMOST_APP_PID.load(Ordering::SeqCst)),
        current_application_pid(),
        frontmost_application_pid(),
    ) {
        return;
    }

    restore_previous_frontmost_application(RestorePreviousFrontmostDisposition::Keep);
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
    install_native_crosshair_cursor_rect(window, ns_window)?;

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
    ns_window.setAlphaValue(1.0);
    if should_make_capture_overlay_key_on_reveal() {
        ns_window.makeKeyAndOrderFront(None);
    } else if capture_overlay_uses_order_front_regardless() {
        ns_window.orderFrontRegardless();
    } else {
        ns_window.orderFront(None);
    }
    push_native_crosshair_cursor();

    Ok(())
}

pub(super) fn prepare_capture_window_for_reveal(window: &WebviewWindow) -> Result<(), String> {
    configure_capture_window_for_current_space(window)?;

    let ns_window = window.ns_window().map_err(|e| e.to_string())?;
    if ns_window.is_null() {
        return Err("Capture window has no native NSWindow".to_string());
    }

    let ns_window: &NSWindow = unsafe { &*ns_window.cast() };
    ns_window.setAlphaValue(0.0);

    Ok(())
}

pub(super) fn hide_capture_window_for_current_space(window: &WebviewWindow) -> Result<(), String> {
    let ns_window = window.ns_window().map_err(|e| e.to_string())?;
    if ns_window.is_null() {
        return Err("Capture window has no native NSWindow".to_string());
    }

    let ns_window: &NSWindow = unsafe { &*ns_window.cast() };
    ns_window.setAlphaValue(1.0);
    ns_window.orderOut(None);
    restore_native_crosshair_cursor();

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
    let base = base
        & !NSWindowCollectionBehavior::MoveToActiveSpace
        & !NSWindowCollectionBehavior::Managed
        & !NSWindowCollectionBehavior::Transient
        & !NSWindowCollectionBehavior::ParticipatesInCycle
        & !NSWindowCollectionBehavior::FullScreenPrimary
        & !NSWindowCollectionBehavior::FullScreenNone;

    base | NSWindowCollectionBehavior::CanJoinAllSpaces
        | NSWindowCollectionBehavior::FullScreenAuxiliary
        | NSWindowCollectionBehavior::Stationary
        | NSWindowCollectionBehavior::IgnoresCycle
}

fn capture_overlay_style_mask(base: NSWindowStyleMask) -> NSWindowStyleMask {
    base | NSWindowStyleMask::NonactivatingPanel
}

fn capture_presentation_activation_policy() -> Option<tauri::ActivationPolicy> {
    None
}

fn capture_window_activation_suppression_policy(
    app: &AppHandle,
) -> Option<tauri::ActivationPolicy> {
    capture_window_activation_suppression_policy_for_state(
        current_application_pid(),
        frontmost_application_pid(),
        has_visible_non_capture_window_on_active_space(app),
    )
}

fn capture_window_activation_suppression_policy_for_state(
    current_pid: i32,
    frontmost_pid: Option<i32>,
    has_active_space_app_window: bool,
) -> Option<tauri::ActivationPolicy> {
    should_suppress_capture_window_activation_for_state(
        current_pid,
        frontmost_pid,
        has_active_space_app_window,
    )
    .then_some(tauri::ActivationPolicy::Accessory)
}

fn has_visible_non_capture_window_on_active_space(app: &AppHandle) -> bool {
    app.webview_windows()
        .into_iter()
        .filter(|(label, _)| label.as_str() != CAPTURE_WINDOW_LABEL)
        .any(|(_, window)| window_is_visible_on_active_space(&window))
}

fn window_is_visible_on_active_space(window: &WebviewWindow) -> bool {
    let Ok(ns_window) = window.ns_window() else {
        return false;
    };
    if ns_window.is_null() {
        return false;
    }

    let ns_window: &NSWindow = unsafe { &*ns_window.cast() };
    ns_window.isVisible() && ns_window.isOnActiveSpace()
}

fn should_suppress_capture_window_activation_for_state(
    current_pid: i32,
    frontmost_pid: Option<i32>,
    _has_active_space_app_window: bool,
) -> bool {
    frontmost_pid != Some(current_pid)
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

fn capture_overlay_uses_native_crosshair_cursor() -> bool {
    true
}

fn capture_overlay_uses_native_crosshair_cursor_rect() -> bool {
    true
}

fn should_restore_native_crosshair_cursor(cursor_pushed: bool) -> bool {
    cursor_pushed
}

fn install_native_crosshair_cursor_rect(
    window: &WebviewWindow,
    ns_window: &NSWindow,
) -> Result<(), String> {
    if !capture_overlay_uses_native_crosshair_cursor_rect() {
        return Ok(());
    }

    ns_window.resetCursorRects();
    let cursor = NSCursor::crosshairCursor();

    if let Some(content_view) = ns_window.contentView() {
        content_view.discardCursorRects();
        content_view.addCursorRect_cursor(content_view.bounds(), &cursor);
    }

    let ns_view = window.ns_view().map_err(|e| e.to_string())?;
    if ns_view.is_null() {
        return Err("Capture window has no native NSView".to_string());
    }

    let ns_view: &NSView = unsafe { &*ns_view.cast() };
    ns_view.discardCursorRects();
    ns_view.addCursorRect_cursor(ns_view.bounds(), &cursor);

    Ok(())
}

fn push_native_crosshair_cursor() {
    if !capture_overlay_uses_native_crosshair_cursor() {
        return;
    }

    if CAPTURE_CROSSHAIR_CURSOR_PUSHED.swap(true, Ordering::SeqCst) {
        return;
    }

    let cursor = NSCursor::crosshairCursor();
    cursor.push();
    cursor.set();
}

fn restore_native_crosshair_cursor() {
    let was_pushed = CAPTURE_CROSSHAIR_CURSOR_PUSHED.swap(false, Ordering::SeqCst);
    if should_restore_native_crosshair_cursor(was_pushed) {
        NSCursor::pop_class();
    }
}

fn should_make_capture_overlay_key_on_reveal() -> bool {
    should_make_capture_overlay_key_on_reveal_for_activation_suppressed(
        CAPTURE_WINDOW_ACTIVATION_SUPPRESSED.load(Ordering::SeqCst),
    )
}

fn should_make_capture_overlay_key_on_reveal_for_activation_suppressed(
    _activation_suppressed: bool,
) -> bool {
    false
}

fn capture_cancel_shortcut_accelerator() -> &'static str {
    CAPTURE_CANCEL_SHORTCUT_ACCELERATOR
}

fn capture_copy_shortcut_accelerator() -> &'static str {
    CAPTURE_COPY_SHORTCUT_ACCELERATOR
}

fn capture_cancel_requested_event_name() -> &'static str {
    CAPTURE_CANCEL_REQUESTED_EVENT
}

fn capture_copy_requested_event_name() -> &'static str {
    CAPTURE_COPY_REQUESTED_EVENT
}

fn capture_overlay_disables_window_animation() -> bool {
    true
}

fn capture_overlay_uses_order_front_regardless() -> bool {
    false
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

fn register_capture_copy_shortcut(app: &AppHandle) -> Result<(), String> {
    if CAPTURE_COPY_SHORTCUT_REGISTERED.swap(true, Ordering::SeqCst) {
        return Ok(());
    }

    let app_clone = app.clone();
    shortcut::register_shortcut(app, capture_copy_shortcut_accelerator(), move || {
        emit_capture_copy_requested(&app_clone);
    })
    .map_err(|e| {
        CAPTURE_COPY_SHORTCUT_REGISTERED.store(false, Ordering::SeqCst);
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

fn unregister_capture_copy_shortcut(app: &AppHandle) {
    if !CAPTURE_COPY_SHORTCUT_REGISTERED.swap(false, Ordering::SeqCst) {
        return;
    }

    if let Err(err) = shortcut::unregister_shortcut(app, capture_copy_shortcut_accelerator()) {
        log::warn!("Failed to unregister capture copy shortcut: {}", err);
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

fn emit_capture_copy_requested(app: &AppHandle) {
    let Some(window) = app.get_webview_window(CAPTURE_WINDOW_LABEL) else {
        return;
    };

    if let Err(err) = window.emit(capture_copy_requested_event_name(), ()) {
        log::warn!("Failed to emit capture copy request: {}", err);
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

fn restore_previous_frontmost_application(disposition: RestorePreviousFrontmostDisposition) {
    let pid = PREVIOUS_FRONTMOST_APP_PID.load(Ordering::SeqCst);
    let current_pid = current_application_pid();
    let current_frontmost_pid = frontmost_application_pid();
    let next_pid = previous_frontmost_pid_after_restore_attempt(
        Some(pid),
        current_pid,
        current_frontmost_pid,
        disposition,
    )
    .unwrap_or(NO_PREVIOUS_FRONTMOST_APP_PID);
    PREVIOUS_FRONTMOST_APP_PID.store(next_pid, Ordering::SeqCst);

    if !should_restore_previous_frontmost_app(Some(pid), current_pid, current_frontmost_pid) {
        return;
    }

    if let Some(application) = NSRunningApplication::runningApplicationWithProcessIdentifier(pid) {
        application.activateWithOptions(NSApplicationActivationOptions::empty());
    }
}

fn should_restore_capture_window_activation(
    activation_suppressed: bool,
    previous_pid: Option<i32>,
    current_pid: i32,
    current_frontmost_pid: Option<i32>,
) -> bool {
    (activation_suppressed || current_frontmost_pid == Some(current_pid))
        && should_restore_previous_frontmost_app(previous_pid, current_pid, current_frontmost_pid)
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

fn previous_frontmost_pid_after_restore_attempt(
    previous_pid: Option<i32>,
    _current_pid: i32,
    _current_frontmost_pid: Option<i32>,
    disposition: RestorePreviousFrontmostDisposition,
) -> Option<i32> {
    match disposition {
        RestorePreviousFrontmostDisposition::Keep => previous_pid.filter(|pid| *pid > 0),
        RestorePreviousFrontmostDisposition::Clear => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_overlay_joins_fullscreen_spaces_without_moving_between_spaces() {
        let behavior = capture_overlay_collection_behavior(NSWindowCollectionBehavior::Default);

        assert!(behavior.contains(NSWindowCollectionBehavior::CanJoinAllSpaces));
        assert!(!behavior.contains(NSWindowCollectionBehavior::MoveToActiveSpace));
        assert!(behavior.contains(NSWindowCollectionBehavior::FullScreenAuxiliary));
        assert!(behavior.contains(NSWindowCollectionBehavior::Stationary));
        assert!(!behavior.contains(NSWindowCollectionBehavior::Transient));
        assert!(behavior.contains(NSWindowCollectionBehavior::IgnoresCycle));
    }

    #[test]
    fn capture_overlay_preserves_existing_collection_behavior() {
        let behavior = capture_overlay_collection_behavior(NSWindowCollectionBehavior::Stationary);

        assert!(behavior.contains(NSWindowCollectionBehavior::Stationary));
        assert!(behavior.contains(NSWindowCollectionBehavior::FullScreenAuxiliary));
    }

    #[test]
    fn capture_overlay_uses_nonactivating_panel_style_for_fullscreen_spaces() {
        let style = capture_overlay_style_mask(NSWindowStyleMask::Borderless);

        assert!(style.contains(NSWindowStyleMask::Borderless));
        assert!(style.contains(NSWindowStyleMask::NonactivatingPanel));
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
    fn capture_presentation_does_not_change_activation_policy() {
        assert!(capture_presentation_activation_policy().is_none());
    }

    #[test]
    fn capture_window_activation_suppression_avoids_raising_visible_app_windows() {
        assert!(should_suppress_capture_window_activation_for_state(
            9000,
            Some(4242),
            true,
        ));
    }

    #[test]
    fn capture_window_activation_suppression_uses_accessory_policy() {
        assert!(matches!(
            capture_window_activation_suppression_policy_for_state(9000, Some(4242), true),
            Some(tauri::ActivationPolicy::Accessory)
        ));
    }

    #[test]
    fn capture_window_activation_suppression_prevents_switching_to_hidden_app_space() {
        assert!(should_suppress_capture_window_activation_for_state(
            9000,
            Some(4242),
            false,
        ));
    }

    #[test]
    fn capture_window_activation_suppression_preserves_snaplingo_frontmost_sessions() {
        assert!(!should_suppress_capture_window_activation_for_state(
            9000,
            Some(9000),
            false,
        ));
    }

    #[test]
    fn capture_suppression_flag_clears_only_after_suppression() {
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
    fn capture_overlay_declares_native_crosshair_cursor() {
        assert!(capture_overlay_uses_native_crosshair_cursor());
    }

    #[test]
    fn capture_overlay_declares_native_crosshair_cursor_rect() {
        assert!(capture_overlay_uses_native_crosshair_cursor_rect());
    }

    #[test]
    fn capture_overlay_restores_native_crosshair_cursor_only_when_pushed() {
        assert!(should_restore_native_crosshair_cursor(true));
        assert!(!should_restore_native_crosshair_cursor(false));
    }

    #[test]
    fn capture_overlay_does_not_become_key_on_reveal() {
        assert!(!should_make_capture_overlay_key_on_reveal_for_activation_suppressed(false));
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
    fn capture_overlay_avoids_order_front_regardless() {
        assert!(!capture_overlay_uses_order_front_regardless());
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
    fn capture_copy_shortcut_uses_primary_copy_accelerator() {
        assert_eq!(capture_copy_shortcut_accelerator(), "CmdOrCtrl+KeyC");
    }

    #[test]
    fn capture_copy_event_matches_frontend_listener() {
        assert_eq!(
            capture_copy_requested_event_name(),
            "capture-copy-requested"
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

    #[test]
    fn restores_capture_activation_when_window_creation_displaced_previous_app() {
        assert!(should_restore_capture_window_activation(
            true,
            Some(4242),
            9000,
            Some(9000),
        ));
        assert!(should_restore_capture_window_activation(
            false,
            Some(4242),
            9000,
            Some(9000),
        ));
        assert!(!should_restore_capture_window_activation(
            true,
            Some(4242),
            9000,
            Some(4242),
        ));
    }

    #[test]
    fn mid_capture_restore_keeps_previous_frontmost_pid_for_final_exit() {
        assert_eq!(
            previous_frontmost_pid_after_restore_attempt(
                Some(4242),
                9000,
                Some(9000),
                RestorePreviousFrontmostDisposition::Keep,
            ),
            Some(4242)
        );
    }

    #[test]
    fn final_capture_restore_clears_previous_frontmost_pid_after_exit() {
        assert_eq!(
            previous_frontmost_pid_after_restore_attempt(
                Some(4242),
                9000,
                Some(9000),
                RestorePreviousFrontmostDisposition::Clear,
            ),
            None
        );
    }
}

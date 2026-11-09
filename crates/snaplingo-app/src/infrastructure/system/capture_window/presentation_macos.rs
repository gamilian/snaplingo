use objc2_app_kit::NSUserInterfaceItemIdentification;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering};

use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSApplication, NSApplicationActivationOptions, NSApplicationActivationPolicy, NSCursor,
    NSRunningApplication, NSWindow, NSWorkspace,
};

use super::backend::CAPTURE_WINDOW_LABEL;

static CAPTURE_PRESENTATION_DEPTH: AtomicUsize = AtomicUsize::new(0);
static CAPTURE_WINDOW_ACTIVATION_SUPPRESSED: AtomicBool = AtomicBool::new(false);
static CAPTURE_CROSSHAIR_CURSOR_PUSHED: AtomicBool = AtomicBool::new(false);
static PREVIOUS_FRONTMOST_APP_PID: AtomicI32 = AtomicI32::new(NO_PREVIOUS_FRONTMOST_APP_PID);
const NO_PREVIOUS_FRONTMOST_APP_PID: i32 = -1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RestorePreviousFrontmostDisposition {
    Keep,
    Clear,
}

fn application() -> objc2::rc::Retained<NSApplication> {
    NSApplication::sharedApplication(
        MainThreadMarker::new().expect("capture lifecycle runs on the main thread"),
    )
}

#[cfg(test)]
pub(super) fn activation_suppressed() -> bool {
    CAPTURE_WINDOW_ACTIVATION_SUPPRESSED.load(Ordering::SeqCst)
}

fn set_activation_policy(policy: NSApplicationActivationPolicy) -> Result<(), String> {
    let app = application();
    if app.activationPolicy() == policy || app.setActivationPolicy(policy) {
        Ok(())
    } else {
        log::warn!(
            "Native capture activation policy was rejected: requested={policy:?} current={:?}",
            app.activationPolicy()
        );
        Err("Failed to apply native capture activation policy".into())
    }
}

pub fn begin_capture_presentation() -> Result<(), String> {
    let previous_depth = CAPTURE_PRESENTATION_DEPTH.fetch_add(1, Ordering::SeqCst);

    if previous_depth == 0 {
        // Capture editing keys stay local to the focused view.
        remember_previous_frontmost_application();
        if let Some(activation_policy) = capture_presentation_activation_policy() {
            if let Err(err) = set_activation_policy(activation_policy) {
                CAPTURE_PRESENTATION_DEPTH.fetch_sub(1, Ordering::SeqCst);
                return Err(err.to_string());
            }
        }
    }

    Ok(())
}

pub fn end_capture_presentation() -> Result<(), String> {
    let previous_depth = decrement_capture_presentation_depth();

    if previous_depth == 1 {
        let has_key_non_capture_window = has_key_non_capture_window();
        let activation_suppressed = take_capture_window_activation_suppressed();
        restore_native_crosshair_cursor();
        let activation_policy_result = if activation_suppressed {
            set_activation_policy(NSApplicationActivationPolicy::Accessory)
        } else {
            Ok(())
        };
        if should_restore_previous_frontmost_after_capture(has_key_non_capture_window) {
            restore_previous_frontmost_application(RestorePreviousFrontmostDisposition::Clear);
        } else {
            clear_previous_frontmost_application();
        }
        activation_policy_result?;
    }

    Ok(())
}

pub fn is_capture_presentation_active() -> bool {
    CAPTURE_PRESENTATION_DEPTH.load(Ordering::SeqCst) > 0
}

pub(super) fn suppress_capture_window_activation() -> Result<(), String> {
    if let Some(activation_policy) = capture_window_activation_suppression_policy() {
        set_activation_policy(activation_policy).map_err(|e| e.to_string())?;
        mark_capture_window_activation_suppressed();
    }

    Ok(())
}

pub(crate) fn restore_capture_window_activation() {
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

fn capture_presentation_activation_policy() -> Option<NSApplicationActivationPolicy> {
    None
}

fn capture_window_activation_suppression_policy() -> Option<NSApplicationActivationPolicy> {
    capture_window_activation_suppression_policy_for_state(
        current_application_pid(),
        frontmost_application_pid(),
        has_visible_non_capture_window_on_active_space(),
    )
}

fn capture_window_activation_suppression_policy_for_state(
    current_pid: i32,
    frontmost_pid: Option<i32>,
    has_active_space_app_window: bool,
) -> Option<NSApplicationActivationPolicy> {
    should_suppress_capture_window_activation_for_state(
        current_pid,
        frontmost_pid,
        has_active_space_app_window,
    )
    .then_some(NSApplicationActivationPolicy::Accessory)
}

fn is_capture_window(window: &NSWindow) -> bool {
    window
        .identifier()
        .is_some_and(|id| id.to_string() == CAPTURE_WINDOW_LABEL)
}

fn has_visible_non_capture_window_on_active_space() -> bool {
    application()
        .windows()
        .iter()
        .any(|window| !is_capture_window(&window) && window.isVisible() && window.isOnActiveSpace())
}

fn has_key_non_capture_window() -> bool {
    application()
        .windows()
        .iter()
        .any(|window| !is_capture_window(&window) && window.isKeyWindow())
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

fn capture_overlay_uses_native_crosshair_cursor() -> bool {
    false
}

fn should_restore_native_crosshair_cursor(cursor_pushed: bool) -> bool {
    cursor_pushed
}

pub(super) fn push_native_crosshair_cursor() {
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

pub(super) fn restore_native_crosshair_cursor() {
    let was_pushed = CAPTURE_CROSSHAIR_CURSOR_PUSHED.swap(false, Ordering::SeqCst);
    if should_restore_native_crosshair_cursor(was_pushed) {
        NSCursor::pop_class();
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

fn clear_previous_frontmost_application() {
    PREVIOUS_FRONTMOST_APP_PID.store(NO_PREVIOUS_FRONTMOST_APP_PID, Ordering::SeqCst);
}

fn should_restore_previous_frontmost_after_capture(has_key_non_capture_window: bool) -> bool {
    !has_key_non_capture_window
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
            Some(NSApplicationActivationPolicy::Accessory)
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
    fn capture_overlay_leaves_cursor_state_to_the_webview() {
        assert!(!capture_overlay_uses_native_crosshair_cursor());
    }

    #[test]
    fn capture_overlay_restores_native_crosshair_cursor_only_when_pushed() {
        assert!(should_restore_native_crosshair_cursor(true));
        assert!(!should_restore_native_crosshair_cursor(false));
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

    #[test]
    fn result_window_focus_survives_capture_presentation_cleanup() {
        assert!(!should_restore_previous_frontmost_after_capture(true));
        assert!(should_restore_previous_frontmost_after_capture(false));
    }
}

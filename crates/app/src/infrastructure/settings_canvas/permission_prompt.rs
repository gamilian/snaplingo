//! One Settings lifetime's guide state. Reading grants never requests them.
use crate::application::{RequiredPermissions, SystemPermission};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub(crate) struct Projection {
    pub visible: bool,
    pub needs_installation: bool,
    pub busy: bool,
    pub attempted: bool,
    pub error: Option<String>,
}
#[derive(Default)]
struct State {
    enabled: bool,
    dismissed: bool,
    busy: bool,
    attempted: bool,
    error: Option<String>,
    generation: u64,
}
pub(crate) struct PermissionPromptSession {
    permissions: Arc<RequiredPermissions>,
    state: Mutex<State>,
}
impl PermissionPromptSession {
    pub(crate) fn new(permissions: Arc<RequiredPermissions>) -> Self {
        Self {
            permissions,
            state: Mutex::new(State::default()),
        }
    }
    pub(crate) fn set_visible(&self, visible: bool) {
        let mut state = self.state.lock().unwrap();
        if state.enabled != visible {
            state.generation = state.generation.wrapping_add(1);
            state.busy = false;
        }
        state.enabled = visible;
    }
    pub(crate) fn projection(&self) -> Projection {
        let status = self.permissions.status();
        let context = self.permissions.context();
        let state = self.state.lock().unwrap();
        Projection {
            visible: state.enabled
                && !state.dismissed
                && context.platform == "macos"
                && !status.screen_recording,
            needs_installation: context.needs_installation,
            busy: state.busy,
            attempted: state.attempted,
            error: state.error.clone(),
        }
    }
    pub(crate) fn dismiss(&self) {
        let mut state = self.state.lock().unwrap();
        state.dismissed = true;
        state.busy = false;
        state.generation = state.generation.wrapping_add(1);
    }
    pub(crate) fn begin_authorize(&self) -> Option<u64> {
        let projection = self.projection();
        if !projection.visible || projection.needs_installation {
            return None;
        }
        let mut state = self.state.lock().unwrap();
        if !state.enabled || state.dismissed || state.busy {
            return None;
        }
        state.busy = true;
        state.attempted = true;
        state.error = None;
        Some(state.generation)
    }
    pub(crate) fn authorize(&self, generation: u64) {
        {
            let state = self.state.lock().unwrap();
            if state.generation != generation || !state.enabled || state.dismissed || !state.busy {
                return;
            }
        }
        let result = if self.permissions.context().needs_installation {
            Ok(())
        } else {
            self.permissions
                .request(SystemPermission::ScreenRecording)
                .map(|_| ())
        };
        let mut state = self.state.lock().unwrap();
        if state.generation == generation {
            state.busy = false;
            state.error = result.err().map(|e| e.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{
        RequiredPermissionsContext, RequiredPermissionsHost, RequiredPermissionsStatus,
    };
    use std::sync::atomic::{AtomicBool, Ordering};
    struct Host {
        screen: AtomicBool,
        temporary: AtomicBool,
        fail: AtomicBool,
        calls: Mutex<Vec<SystemPermission>>,
    }
    impl RequiredPermissionsHost for Host {
        fn status(&self) -> RequiredPermissionsStatus {
            RequiredPermissionsStatus {
                screen_recording: self.screen.load(Ordering::SeqCst),
                accessibility: false,
            }
        }
        fn context(&self) -> RequiredPermissionsContext {
            RequiredPermissionsContext {
                platform: "macos".into(),
                app_path: None,
                needs_installation: self.temporary.load(Ordering::SeqCst),
                can_reset: !self.temporary.load(Ordering::SeqCst),
            }
        }
        fn request(&self, p: SystemPermission) -> crate::Result<()> {
            self.calls.lock().unwrap().push(p);
            if self.fail.load(Ordering::SeqCst) {
                Err("request failed".into())
            } else {
                Ok(())
            }
        }
        fn reset(&self, _: SystemPermission) -> crate::Result<()> {
            panic!("First-use guide never resets")
        }
        fn restart(&self) -> crate::Result<()> {
            panic!("First-use guide never restarts")
        }
    }
    fn fixture() -> (Arc<Host>, PermissionPromptSession) {
        let host = Arc::new(Host {
            screen: AtomicBool::new(false),
            temporary: AtomicBool::new(false),
            fail: AtomicBool::new(false),
            calls: Mutex::new(vec![]),
        });
        let session =
            PermissionPromptSession::new(Arc::new(RequiredPermissions::new(host.clone())));
        (host, session)
    }
    #[test]
    fn showing_polling_and_returning_from_system_settings_only_read_grants() {
        let (host, s) = fixture();
        assert!(!s.projection().visible);
        s.set_visible(true);
        assert!(s.projection().visible);
        assert!(s.projection().visible);
        assert!(host.calls.lock().unwrap().is_empty());
        host.screen.store(true, Ordering::SeqCst);
        assert!(
            !s.projection().visible,
            "Optional accessibility does not block the guide"
        );
        host.screen.store(false, Ordering::SeqCst);
        assert!(s.projection().visible, "Revocation is observed on refresh");
        s.set_visible(false);
        assert!(!s.projection().visible);
        s.set_visible(true);
        assert!(s.projection().visible);
    }
    #[test]
    fn explicit_authorization_is_serial_and_targets_only_screen_recording() {
        let (host, s) = fixture();
        s.set_visible(true);
        let ticket = s.begin_authorize().unwrap();
        assert!(s.begin_authorize().is_none());
        assert!(s.projection().busy);
        s.authorize(ticket);
        assert_eq!(
            *host.calls.lock().unwrap(),
            vec![SystemPermission::ScreenRecording]
        );
        assert!(!s.projection().busy);
        assert!(s.projection().attempted);
        assert!(s.projection().visible);
    }
    #[test]
    fn later_and_temporary_installations_never_authorize_or_reopen_within_settings_lifetime() {
        let (host, s) = fixture();
        s.set_visible(true);
        host.temporary.store(true, Ordering::SeqCst);
        assert!(s.projection().needs_installation);
        assert!(s.begin_authorize().is_none());
        host.temporary.store(false, Ordering::SeqCst);
        let ticket = s.begin_authorize().unwrap();
        s.dismiss();
        s.authorize(ticket);
        s.set_visible(false);
        s.set_visible(true);
        assert!(!s.projection().visible);
        assert!(host.calls.lock().unwrap().is_empty());
    }
    #[test]
    fn changed_grant_skips_request_and_late_cancelled_work_cannot_poison_new_session() {
        let (host, s) = fixture();
        s.set_visible(true);
        let old = s.begin_authorize().unwrap();
        s.set_visible(false);
        s.set_visible(true);
        let current = s.begin_authorize().unwrap();
        s.authorize(old);
        assert!(s.projection().busy);
        assert!(host.calls.lock().unwrap().is_empty());
        host.screen.store(true, Ordering::SeqCst);
        s.authorize(current);
        assert!(host.calls.lock().unwrap().is_empty());
        assert!(!s.projection().visible);
        assert!(!s.projection().busy);
    }
    #[test]
    fn error_preserves_guide_and_retry_while_current_copy_becoming_temporary_blocks_request() {
        let (host, s) = fixture();
        s.set_visible(true);
        host.fail.store(true, Ordering::SeqCst);
        s.authorize(s.begin_authorize().unwrap());
        assert_eq!(s.projection().error.as_deref(), Some("request failed"));
        host.fail.store(false, Ordering::SeqCst);
        let retry = s.begin_authorize().unwrap();
        assert!(s.projection().error.is_none());
        host.temporary.store(true, Ordering::SeqCst);
        s.authorize(retry);
        assert_eq!(host.calls.lock().unwrap().len(), 1);
        assert!(s.projection().needs_installation);
        assert!(!s.projection().busy);
    }
}

use std::sync::Arc;

use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RequiredPermissionsStatus {
    pub screen_recording: bool,
    pub accessibility: bool,
}

impl RequiredPermissionsStatus {
    pub fn all_granted(self) -> bool {
        self.screen_recording && self.accessibility
    }
}

pub trait RequiredPermissionsHost: Send + Sync {
    fn status(&self) -> RequiredPermissionsStatus;
    fn request_screen_recording(&self);
    fn request_accessibility(&self);
}

pub struct RequiredPermissions {
    host: Arc<dyn RequiredPermissionsHost>,
}

impl RequiredPermissions {
    pub fn new(host: Arc<dyn RequiredPermissionsHost>) -> Self {
        Self { host }
    }

    pub fn status(&self) -> RequiredPermissionsStatus {
        self.host.status()
    }

    pub fn request_next_missing(&self) -> RequiredPermissionsStatus {
        let status = self.host.status();
        if !status.screen_recording {
            log::info!("[permissions] requesting screen recording");
            self.host.request_screen_recording();
        } else if !status.accessibility {
            log::info!("[permissions] requesting accessibility");
            self.host.request_accessibility();
        }
        let updated = self.host.status();
        log::info!(
            "[permissions] status after request: screen_recording={}, accessibility={}",
            updated.screen_recording,
            updated.accessibility,
        );
        updated
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct FakeHost {
        status: RequiredPermissionsStatus,
        requests: Mutex<Vec<&'static str>>,
    }

    impl RequiredPermissionsHost for FakeHost {
        fn status(&self) -> RequiredPermissionsStatus {
            self.status
        }
        fn request_screen_recording(&self) {
            self.requests.lock().unwrap().push("screenRecording");
        }
        fn request_accessibility(&self) {
            self.requests.lock().unwrap().push("accessibility");
        }
    }

    #[test]
    fn requests_screen_recording_before_accessibility() {
        let host = Arc::new(FakeHost {
            status: RequiredPermissionsStatus {
                screen_recording: false,
                accessibility: false,
            },
            requests: Mutex::new(Vec::new()),
        });
        RequiredPermissions::new(host.clone()).request_next_missing();
        assert_eq!(*host.requests.lock().unwrap(), vec!["screenRecording"]);
    }

    #[test]
    fn requests_accessibility_after_screen_recording_is_granted() {
        let host = Arc::new(FakeHost {
            status: RequiredPermissionsStatus {
                screen_recording: true,
                accessibility: false,
            },
            requests: Mutex::new(Vec::new()),
        });
        RequiredPermissions::new(host.clone()).request_next_missing();
        assert_eq!(*host.requests.lock().unwrap(), vec!["accessibility"]);
    }
}

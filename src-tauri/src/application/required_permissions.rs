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

    pub fn request_missing(&self) -> RequiredPermissionsStatus {
        let status = self.host.status();
        if !status.screen_recording {
            self.host.request_screen_recording();
        }
        if !status.accessibility {
            self.host.request_accessibility();
        }
        self.host.status()
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
    fn requests_every_missing_required_permission() {
        let host = Arc::new(FakeHost {
            status: RequiredPermissionsStatus {
                screen_recording: false,
                accessibility: false,
            },
            requests: Mutex::new(Vec::new()),
        });
        RequiredPermissions::new(host.clone()).request_missing();
        assert_eq!(
            *host.requests.lock().unwrap(),
            vec!["screenRecording", "accessibility"]
        );
    }
}

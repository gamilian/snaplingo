use std::sync::Arc;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SystemPermission {
    ScreenRecording,
    Accessibility,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RequiredPermissionsStatus {
    pub screen_recording: bool,
    pub accessibility: bool,
}

impl RequiredPermissionsStatus {
    fn granted(self, permission: SystemPermission) -> bool {
        match permission {
            SystemPermission::ScreenRecording => self.screen_recording,
            SystemPermission::Accessibility => self.accessibility,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequiredPermissionsContext {
    pub platform: String,
    pub app_path: Option<String>,
    pub needs_installation: bool,
    pub can_reset: bool,
}

pub trait RequiredPermissionsHost: Send + Sync {
    fn status(&self) -> RequiredPermissionsStatus;
    fn context(&self) -> RequiredPermissionsContext;
    fn request(&self, permission: SystemPermission) -> crate::Result<()>;
    fn reset(&self, permission: SystemPermission) -> crate::Result<()>;
    fn restart(&self) -> crate::Result<()>;
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

    pub fn context(&self) -> RequiredPermissionsContext {
        self.host.context()
    }

    pub fn request(
        &self,
        permission: SystemPermission,
    ) -> crate::Result<RequiredPermissionsStatus> {
        if !self.host.status().granted(permission) {
            self.host.request(permission)?;
        }
        Ok(self.host.status())
    }

    pub fn reset(&self, permission: SystemPermission) -> crate::Result<RequiredPermissionsStatus> {
        if !self.host.context().can_reset {
            return Err("请先将 SnapLingo 安装到应用程序，再修复授权。".into());
        }
        self.host.reset(permission)?;
        self.host.request(permission)?;
        Ok(self.host.status())
    }

    pub fn restart(&self) -> crate::Result<()> {
        self.host.restart()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct FakeHost {
        status: RequiredPermissionsStatus,
        requests: Mutex<Vec<SystemPermission>>,
        resets: Mutex<Vec<SystemPermission>>,
        reset_fails: bool,
    }

    impl RequiredPermissionsHost for FakeHost {
        fn status(&self) -> RequiredPermissionsStatus {
            self.status
        }
        fn context(&self) -> RequiredPermissionsContext {
            RequiredPermissionsContext {
                platform: "macos".into(),
                app_path: Some("/Applications/SnapLingo.app".into()),
                needs_installation: false,
                can_reset: true,
            }
        }
        fn request(&self, permission: SystemPermission) -> crate::Result<()> {
            self.requests.lock().unwrap().push(permission);
            Ok(())
        }
        fn reset(&self, permission: SystemPermission) -> crate::Result<()> {
            self.resets.lock().unwrap().push(permission);
            if self.reset_fails {
                return Err("reset failed".into());
            }
            Ok(())
        }
        fn restart(&self) -> crate::Result<()> {
            Ok(())
        }
    }

    fn host(screen_recording: bool, accessibility: bool) -> FakeHost {
        FakeHost {
            status: RequiredPermissionsStatus {
                screen_recording,
                accessibility,
            },
            requests: Mutex::new(Vec::new()),
            resets: Mutex::new(Vec::new()),
            reset_fails: false,
        }
    }

    #[test]
    fn requests_only_the_permission_the_user_selected() {
        let host = Arc::new(host(false, false));
        RequiredPermissions::new(host.clone())
            .request(SystemPermission::Accessibility)
            .unwrap();
        assert_eq!(
            *host.requests.lock().unwrap(),
            vec![SystemPermission::Accessibility]
        );
        assert!(host.resets.lock().unwrap().is_empty());
    }

    #[test]
    fn existing_screen_grant_does_not_request_optional_accessibility() {
        let host = Arc::new(host(true, false));
        let permissions = RequiredPermissions::new(host.clone());
        assert!(permissions.status().screen_recording);
        permissions
            .request(SystemPermission::ScreenRecording)
            .unwrap();
        assert!(host.requests.lock().unwrap().is_empty());
        assert!(host.resets.lock().unwrap().is_empty());
    }

    #[test]
    fn repair_resets_and_requests_only_the_selected_permission() {
        let host = Arc::new(host(false, true));
        RequiredPermissions::new(host.clone())
            .reset(SystemPermission::ScreenRecording)
            .unwrap();
        assert_eq!(
            *host.resets.lock().unwrap(),
            vec![SystemPermission::ScreenRecording]
        );
        assert_eq!(
            *host.requests.lock().unwrap(),
            vec![SystemPermission::ScreenRecording]
        );
        assert!(host.status().accessibility);
    }

    #[test]
    fn failed_repair_is_reported_without_requesting_again() {
        let host = Arc::new(FakeHost {
            reset_fails: true,
            ..host(false, true)
        });
        assert!(RequiredPermissions::new(host.clone())
            .reset(SystemPermission::ScreenRecording)
            .is_err());
        assert!(host.requests.lock().unwrap().is_empty());
    }
}

use crate::application::{RequiredPermissionsHost, RequiredPermissionsStatus};

pub struct SystemRequiredPermissions;

#[cfg(target_os = "macos")]
impl RequiredPermissionsHost for SystemRequiredPermissions {
    fn status(&self) -> RequiredPermissionsStatus {
        RequiredPermissionsStatus {
            screen_recording: unsafe { CGPreflightScreenCaptureAccess() },
            accessibility: crate::infrastructure::system::selection::macos::context::accessibility_permission_granted(false),
        }
    }

    fn request_screen_recording(&self) {
        unsafe {
            CGRequestScreenCaptureAccess();
        }
    }
    fn request_accessibility(&self) {
        crate::infrastructure::system::selection::macos::context::request_accessibility_permission(
        );
    }
}

#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;
}

#[cfg(not(target_os = "macos"))]
impl RequiredPermissionsHost for SystemRequiredPermissions {
    fn status(&self) -> RequiredPermissionsStatus {
        RequiredPermissionsStatus {
            screen_recording: true,
            accessibility: true,
        }
    }
    fn request_screen_recording(&self) {}
    fn request_accessibility(&self) {}
}

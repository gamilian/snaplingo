use crate::application::{RequiredPermissionsHost, RequiredPermissionsStatus};

#[cfg(target_os = "macos")]
const SCREEN_RECORDING_SETTINGS_URL: &str =
    "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture";
#[cfg(target_os = "macos")]
const ACCESSIBILITY_SETTINGS_URL: &str =
    "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility";

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
        open_privacy_settings(SCREEN_RECORDING_SETTINGS_URL);
    }
    fn request_accessibility(&self) {
        crate::infrastructure::system::selection::macos::context::request_accessibility_permission(
        );
        open_privacy_settings(ACCESSIBILITY_SETTINGS_URL);
    }
}

#[cfg(target_os = "macos")]
fn open_privacy_settings(url: &str) {
    match std::process::Command::new("/usr/bin/open")
        .arg(url)
        .status()
    {
        Ok(status) if status.success() => {}
        Ok(status) => log::warn!(
            "Failed to open macOS privacy settings: open exited with {}",
            status
        ),
        Err(error) => log::warn!("Failed to open macOS privacy settings: {}", error),
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

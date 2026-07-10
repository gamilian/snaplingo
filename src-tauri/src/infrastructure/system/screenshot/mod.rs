mod geometry;
mod image;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
mod xcap_common;

use std::sync::Arc;

use crate::application::CaptureSessionSource;

pub fn get_capture_session_source() -> Arc<dyn CaptureSessionSource> {
    #[cfg(target_os = "macos")]
    {
        Arc::new(macos::MacOSCaptureSessionSource::new())
    }

    #[cfg(target_os = "windows")]
    {
        Arc::new(windows::WindowsCaptureSessionSource::new())
    }

    #[cfg(target_os = "linux")]
    {
        Arc::new(linux::LinuxCaptureSessionSource::new())
    }
}

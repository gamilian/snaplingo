mod backend;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "linux")]
mod linux;

pub use backend::{ScreenshotBackend, ScreenRegion};

use std::sync::Arc;

/// Get the platform-specific screenshot backend
pub fn get_screenshot_backend() -> Arc<dyn ScreenshotBackend> {
    #[cfg(target_os = "macos")]
    {
        Arc::new(macos::MacOSScreenshotBackend::new())
    }

    #[cfg(target_os = "windows")]
    {
        Arc::new(windows::WindowsScreenshotBackend::new())
    }

    #[cfg(target_os = "linux")]
    {
        Arc::new(linux::LinuxScreenshotBackend::new())
    }
}

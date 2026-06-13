mod backend;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "linux")]
mod linux;

pub use backend::{HotkeyBackend, HotkeyId};

use std::sync::Arc;

/// Get the platform-specific hotkey backend
pub fn get_hotkey_backend() -> Arc<dyn HotkeyBackend> {
    #[cfg(target_os = "macos")]
    {
        Arc::new(macos::MacOSHotkeyBackend::new())
    }

    #[cfg(target_os = "windows")]
    {
        Arc::new(windows::WindowsHotkeyBackend::new())
    }

    #[cfg(target_os = "linux")]
    {
        Arc::new(linux::LinuxHotkeyBackend::new())
    }
}

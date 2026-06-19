mod backend;

// Old hotkey system - replaced by tauri-plugin-global-shortcut
// Keeping files for reference but not compiling
// #[cfg(target_os = "macos")]
// pub mod macos;
// #[cfg(target_os = "windows")]
// mod windows;
// #[cfg(target_os = "linux")]
// mod linux;

pub use backend::{HotkeyBackend, HotkeyId};

use std::sync::Arc;

/// Get the platform-specific hotkey backend
/// NOTE: This is deprecated - use tauri-plugin-global-shortcut instead
#[deprecated(note = "Use tauri-plugin-global-shortcut instead")]
pub fn get_hotkey_backend() -> Arc<dyn HotkeyBackend> {
    // Return a dummy implementation since this is no longer used
    panic!("Legacy hotkey backend is deprecated. Use tauri-plugin-global-shortcut instead.");
}

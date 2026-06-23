pub mod backend;
pub mod common;
pub mod registry;

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;

pub use backend::{SelectionContextProvider, SelectionMethod, SystemSelectionProvider};
pub use registry::SelectionMethodRegistry;

#[cfg(target_os = "linux")]
pub use linux::platform_selection_provider;
#[cfg(target_os = "macos")]
pub use macos::platform_selection_provider;
#[cfg(target_os = "windows")]
pub use windows::platform_selection_provider;

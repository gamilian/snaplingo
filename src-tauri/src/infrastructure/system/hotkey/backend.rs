use crate::error::AppError;

/// Opaque identifier for registered hotkeys
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HotkeyId(pub(super) u32);

/// Platform-agnostic hotkey backend trait
#[async_trait::async_trait]
pub trait HotkeyBackend: Send + Sync {
    /// Register a global hotkey
    /// Returns a HotkeyId that can be used to unregister
    async fn register(&self, accelerator: &str) -> Result<HotkeyId, AppError>;

    /// Unregister a previously registered hotkey
    async fn unregister(&self, id: HotkeyId) -> Result<(), AppError>;

    /// Check if a hotkey is currently registered
    async fn is_registered(&self, id: HotkeyId) -> bool;
}

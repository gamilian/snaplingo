pub mod capture;
pub mod favorites;
pub mod history;
pub mod hotkeys;
pub mod pinned_image;
pub mod providers;
pub mod result_window;
pub mod screenshot_favorites;
pub mod selected_text;
pub mod settings;

pub use capture::{CaptureOutput, CaptureSessionRuntime, CaptureSessionSource, CaptureSessions};
pub use favorites::Favorites;
pub use history::{History, OcrHistoryReplay};
pub use hotkeys::{HotkeyConfiguration, HotkeyRuntime, HotkeyUpdateOutcome};
pub use pinned_image::PinnedImageRuntime;
pub use providers::Provider;
pub use screenshot_favorites::{ScreenshotFavoriteCapture, ScreenshotFavorites};
pub use selected_text::{
    SelectedTextAcquirer, SelectionContextProvider, SelectionMethod, SelectionScheme,
    SystemSelectionProvider,
};
pub use settings::SettingsConfiguration;

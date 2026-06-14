use serde::{Deserialize, Serialize};

/// Actions that can be triggered by hotkeys.
///
/// Each variant represents a distinct user-facing workflow that can be
/// initiated via a global keyboard shortcut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HotkeyAction {
    /// Capture screenshot and show image (no OCR/translation)
    Screenshot,

    /// Capture screenshot, perform OCR, and display recognized text
    ScreenshotOcr,

    /// Capture screenshot, perform OCR, translate, and show results
    ScreenshotTranslate,

    /// Translate currently selected text
    SelectionTranslate,

    /// Show input window for manual text translation
    InputTranslate,
}

impl HotkeyAction {
    /// Returns a human-readable name for this action.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Screenshot => "Screenshot",
            Self::ScreenshotOcr => "Screenshot OCR",
            Self::ScreenshotTranslate => "Screenshot Translate",
            Self::SelectionTranslate => "Selection Translate",
            Self::InputTranslate => "Input Translate",
        }
    }

    /// Returns a description of what this action does.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Screenshot => "Capture a screenshot",
            Self::ScreenshotOcr => "Capture screenshot and recognize text",
            Self::ScreenshotTranslate => "Capture screenshot, OCR, and translate",
            Self::SelectionTranslate => "Translate selected text",
            Self::InputTranslate => "Open input window to translate text",
        }
    }
}

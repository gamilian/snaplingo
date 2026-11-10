//! Shared Settings state, interaction, layout and window projection.
pub(crate) mod canvas;
pub mod capacity;
mod draw;
mod field;
mod general;
mod general_view;
mod model;
mod scene;
mod screenshot;
mod screenshot_edit;
mod screenshot_view;
mod state;
#[cfg(test)]
mod tests;
mod translation;
#[cfg(test)]
mod translation_tests;
mod window;

use crate::foundation::input::chord;
use crate::theme::{palette, Palette};
use crate::*;
use draw::{r, Paint};
use field::{fields, Field};
pub use general::{GeneralCommand, GeneralField, GeneralPreferences, GeneralRuntime};
pub use model::{Action, Capacity, Event, Hotkey, OcrPreferences, Snapshot, Value};
pub use scene::{
    GeometryKind, GeometryNode, HitRegion, OcrField, Page, Section, SettingsFrame, Target,
};
pub use screenshot::{ScreenshotField, ScreenshotPreferences, ANNOTATION_SHORTCUT_LABELS};
use screenshot_edit::Editing;
use state::Menu;
pub use state::SettingsUi;
pub use translation::{TranslationField, TranslationPreferences};
pub use window::{
    WindowControls, WindowFrame, WindowInput, WindowIntent, WindowSnapshot, WindowUi,
};

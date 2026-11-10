//! Portable product views and shared UI foundations.
//! Native hosts provide input/effects and consume scenes or presentation models.
#[cfg(feature = "accessibility")]
pub mod accessibility;
pub use settings::capacity;
pub mod capture;
pub mod foundation;
pub mod library;
pub mod notices;
pub mod permission_prompt;
pub mod pinned;
#[cfg(feature = "rendering")]
pub mod render;
pub mod result;
pub mod scrolling;
pub mod services;
pub mod settings;

pub use foundation::theme;
pub use foundation::{
    Color, DrawCommand, FontFamily, Key, KeyStyle, Modifiers, Movement, Point, RasterImage, Rect,
    Size, TextAlign, TextEdit, TextLayout, TextMeasurement, TextMeasurer, Theme,
};
pub use settings::{
    Action, Capacity, Event, GeneralCommand, GeneralField, GeneralPreferences, GeneralRuntime,
    GeometryKind, GeometryNode, HitRegion, Hotkey, OcrField, OcrPreferences, Page, Scene,
    ScreenshotField, ScreenshotPreferences, Section, SettingsUi, Snapshot, Target,
    TranslationField, TranslationPreferences, Value, WindowControls, WindowFrame, WindowInput,
    WindowIntent, WindowSnapshot, WindowUi, ANNOTATION_SHORTCUT_LABELS,
};

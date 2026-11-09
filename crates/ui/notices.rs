//! Searchable component inventory and selectable full license text.
mod draw;
mod state;
use crate::{Key, Modifiers, Point, Rect, Scene};
pub use state::NoticesUi;
#[derive(Clone, Debug)]
pub struct NoticeComponent {
    pub name: String,
    pub version: String,
    pub license: String,
    pub source: String,
    pub text: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum NoticesTarget {
    Close,
    Search,
    ClearSearch,
    Component(usize),
    Back,
    Source,
    License,
}
#[derive(Clone, Debug)]
pub struct NoticesControl {
    pub target: NoticesTarget,
    pub rect: Rect,
    pub hit_rect: Rect,
    pub label: String,
    pub value: Option<String>,
}
pub struct NoticesFrame {
    pub scene: Scene,
    pub controls: Vec<NoticesControl>,
}
pub enum NoticesInput {
    Activate(NoticesTarget),
    Focus(NoticesTarget),
    SetSearch(String),
    PointerDown(Point),
    PointerMove(Point),
    PointerUp,
    Scroll(f32),
    Key(Key, Modifiers),
    Text(String),
    Preedit(String, Option<std::ops::Range<usize>>),
    Commit(String),
    ImeDisabled,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NoticesIntent {
    Close,
    OpenSource(String),
    Copy(String),
    ReadClipboard,
}
#[cfg(test)]
mod tests;

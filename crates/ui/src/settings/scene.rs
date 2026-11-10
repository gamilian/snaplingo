use crate::{Point, Rect, Scene};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    General,
    Screenshot,
    Translation,
    Ocr,
    Services,
    Favorites,
    History,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    Hotkeys,
    Recognition,
    Window,
    Language,
    Behavior,
    Output,
    CaptureInterface,
    Interface,
    Permissions,
    Network,
    Maintenance,
    Experimental,
    About,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OcrField {
    GpuAcceleration,
    RecognitionLanguage,
    PreserveFormatting,
    RemoveChineseSpaces,
    ShowConfidence,
    WindowPosition,
    HideSilentStatus,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    General(crate::GeneralField),
    GeneralOption(crate::GeneralField, usize),
    GeneralCommand(crate::GeneralCommand),
    GeneralTheme(usize),
    GeneralStep(crate::GeneralField, bool),
    GeneralDisclosure(bool),
    GeneralUnavailable,
    Screenshot(crate::ScreenshotField),
    ScreenshotStep(bool),
    ScreenshotOption(crate::ScreenshotField, usize),
    ScreenshotColor(crate::ScreenshotField, usize),
    BrowseScreenshotDirectory,
    PickScreenshotColor(crate::ScreenshotField),
    Translation(crate::TranslationField),
    TranslationOption(crate::TranslationField, usize),
    Navigate(Page),
    Section(Section),
    Ocr(OcrField),
    Option(OcrField, usize),
    Hotkey(usize),
    ResetHotkey(usize),
    ClearHotkey(usize),
    ResetHotkeys,
    CheckConflicts,
    Capacity,
}
#[derive(Clone, Debug, PartialEq)]
pub struct HitRegion {
    pub rect: Rect,
    pub target: Target,
    pub label: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeometryKind {
    Card(Section),
    Control(Target),
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeometryNode {
    pub kind: GeometryKind,
    pub rect: Rect,
}
#[derive(Clone, Debug, PartialEq)]
pub struct SettingsFrame {
    pub scene: Scene,
    pub ime_cursor: Option<Rect>,
    /// Page body, including its section directory, excluding the main navigation.
    /// An embedded host may crop this frame and offset input by its origin.
    pub content: Rect,
    /// Already intersected with the corresponding drawing clip, in paint order.
    pub hits: Vec<HitRegion>,
    /// Unclipped layout frames, including disabled controls, for visual parity QA.
    pub geometry: Vec<GeometryNode>,
    pub content_height: f32,
    pub viewport: Rect,
}
impl SettingsFrame {
    pub fn scaled(mut self, scale: f32) -> Self {
        self.scene = self.scene.scaled(scale);
        self.ime_cursor = self.ime_cursor.map(|rect| rect.scaled(scale));
        self.content = self.content.scaled(scale);
        self.viewport = self.viewport.scaled(scale);
        self.content_height *= scale;
        for hit in &mut self.hits {
            hit.rect = hit.rect.scaled(scale);
        }
        for node in &mut self.geometry {
            node.rect = node.rect.scaled(scale);
        }
        self
    }
    pub fn hit_test(&self, p: Point) -> Option<Target> {
        self.hits
            .iter()
            .rev()
            .find(|hit| hit.rect.contains(p))
            .map(|hit| hit.target)
    }
}

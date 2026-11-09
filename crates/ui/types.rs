//! Renderer-independent geometry. Coordinates are window logical DIP, including
//! the user's interface scale; the renderer applies the device pixel ratio.
pub type Color = [u8; 4];

/// Immutable premultiplied RGBA pixels. Hosts decode image assets once and
/// reuse this value in snapshots; drawing never reads files or URLs.
#[derive(Clone, PartialEq)]
pub struct RasterImage {
    width: u32,
    height: u32,
    rgba: std::sync::Arc<[u8]>,
}
impl std::fmt::Debug for RasterImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RasterImage")
            .field("width", &self.width)
            .field("height", &self.height)
            .finish_non_exhaustive()
    }
}
impl RasterImage {
    pub fn from_rgba(width: u32, height: u32, mut rgba: Vec<u8>) -> Option<Self> {
        let length = (width as usize)
            .checked_mul(height as usize)?
            .checked_mul(4)?;
        if width == 0 || height == 0 || rgba.len() != length {
            return None;
        }
        for pixel in rgba.chunks_exact_mut(4) {
            let alpha = u16::from(pixel[3]);
            for channel in &mut pixel[..3] {
                *channel = ((u16::from(*channel) * alpha + 127) / 255) as u8;
            }
        }
        Some(Self {
            width,
            height,
            rgba: rgba.into(),
        })
    }
    pub fn width(&self) -> u32 {
        self.width
    }
    pub fn height(&self) -> u32 {
        self.height
    }
    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
impl Rect {
    pub fn contains(self, p: Point) -> bool {
        p.x >= self.x && p.y >= self.y && p.x < self.x + self.width && p.y < self.y + self.height
    }
    pub fn intersect(self, other: Self) -> Self {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        Self {
            x,
            y,
            width: (self.x + self.width).min(other.x + other.width).max(x) - x,
            height: (self.y + self.height).min(other.y + other.height).max(y) - y,
        }
    }
    pub(crate) fn scaled(self, s: f32) -> Self {
        Self {
            x: self.x * s,
            y: self.y * s,
            width: self.width * s,
            height: self.height * s,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontFamily {
    Sans,
    /// Retained Windows UI stack; non-Windows reference rendering uses Arial.
    WindowsUi,
    Serif,
    Mono,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}
/// Shaped caret positions in unscaled logical DIP, at grapheme boundaries.
/// RTL text may have non-monotonic x values. Byte offsets address UTF-8 text.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextMeasurement {
    pub caret_stops: Vec<(usize, f32)>,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextLayout {
    pub height: f32,
    pub caret_stops: Vec<(usize, Point)>,
}
pub trait TextMeasurer {
    fn measure(&mut self, text: &str, size: f32, weight: u16, font: FontFamily) -> TextMeasurement;
    fn height(&mut self, text: &str, width: f32, size: f32, weight: u16, font: FontFamily) -> f32 {
        text.split('\n')
            .map(|line| {
                let measured = self.measure(line, size, weight, font);
                let span = measured
                    .caret_stops
                    .iter()
                    .map(|(_, x)| *x)
                    .fold(0., f32::max);
                (span / width.max(1.)).ceil().max(1.) * size * 1.35
            })
            .sum()
    }
    fn line_layout(
        &mut self,
        text: &str,
        width: f32,
        size: f32,
        weight: u16,
        font: FontFamily,
        line_height: f32,
    ) -> TextLayout {
        use unicode_segmentation::UnicodeSegmentation;
        let mut layout = TextLayout::default();
        let mut point = Point::default();
        for (byte, grapheme) in text.grapheme_indices(true) {
            if grapheme == "\n" || grapheme == "\r\n" {
                layout.caret_stops.push((byte, point));
                point.x = 0.;
                point.y += line_height;
                continue;
            }
            let advance = self
                .measure(grapheme, size, weight, font)
                .caret_stops
                .iter()
                .map(|(_, x)| *x)
                .fold(0., f32::max);
            if point.x > 0. && point.x + advance > width.max(1.) {
                point.x = 0.;
                point.y += line_height;
            }
            layout.caret_stops.push((byte, point));
            point.x += advance;
        }
        layout.caret_stops.push((text.len(), point));
        layout.height = point.y + line_height;
        layout
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum DrawCommand {
    Image {
        rect: Rect,
        image: RasterImage,
    },
    FillRect {
        rect: Rect,
        radius: f32,
        color: Color,
    },
    StrokeRect {
        rect: Rect,
        radius: f32,
        width: f32,
        color: Color,
    },
    Line {
        from: Point,
        to: Point,
        width: f32,
        color: Color,
    },
    /// Text wraps inside `rect`; overflowing glyphs are clipped to that box.
    Text {
        rect: Rect,
        text: String,
        size: f32,
        weight: u16,
        color: Color,
        font: FontFamily,
        align: TextAlign,
    },
    TextBlock {
        rect: Rect,
        text: String,
        size: f32,
        weight: u16,
        color: Color,
        font: FontFamily,
        line_height: f32,
    },
    PushClip(Rect),
    PopClip,
}
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
pub struct Scene {
    pub size: Size,
    pub ime_cursor: Option<Rect>,
    /// Page body, including its section directory, excluding the main navigation.
    /// An embedded host may crop this frame and offset input by its origin.
    pub content: Rect,
    pub commands: Vec<DrawCommand>,
    /// Already intersected with the corresponding drawing clip, in paint order.
    pub hits: Vec<HitRegion>,
    /// Unclipped layout frames, including disabled controls, for visual parity QA.
    pub geometry: Vec<GeometryNode>,
    pub content_height: f32,
    pub viewport: Rect,
}
impl Scene {
    pub fn scaled(mut self, scale: f32) -> Self {
        self.size.width *= scale;
        self.size.height *= scale;
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
        for command in &mut self.commands {
            match command {
                DrawCommand::Image { rect, .. } | DrawCommand::PushClip(rect) => {
                    *rect = rect.scaled(scale)
                }
                DrawCommand::FillRect { rect, radius, .. } => {
                    *rect = rect.scaled(scale);
                    *radius *= scale;
                }
                DrawCommand::StrokeRect {
                    rect,
                    radius,
                    width,
                    ..
                } => {
                    *rect = rect.scaled(scale);
                    *radius *= scale;
                    *width *= scale;
                }
                DrawCommand::Line {
                    from, to, width, ..
                } => {
                    from.x *= scale;
                    from.y *= scale;
                    to.x *= scale;
                    to.y *= scale;
                    *width *= scale;
                }
                DrawCommand::Text { rect, size, .. } => {
                    *rect = rect.scaled(scale);
                    *size *= scale;
                }
                DrawCommand::TextBlock {
                    rect,
                    size,
                    line_height,
                    ..
                } => {
                    *rect = rect.scaled(scale);
                    *size *= scale;
                    *line_height *= scale;
                }
                DrawCommand::PopClip => {}
            }
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

//! Shared pinned-image presentation and input policy. Native adapters own pixels,
//! windows, menus and clipboard/file effects.
mod presentation;
#[cfg(test)]
mod tests;
use crate::{
    Color, DrawCommand, FontFamily, Key, Modifiers, Point, RasterImage, Rect, Scene, Size,
    TextAlign,
};
pub use presentation::{ImageSize, Presentation};

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    Copy,
    Save,
    QuickSave,
    Close,
    Destroy,
    HideGroup,
    DestroyGroup,
    CloseAll,
    MoveGroup,
    Preferences,
    Replace,
    ToggleBorder,
    CopyText(String),
}
#[derive(Clone, Debug)]
pub struct Preferences {
    pub opacity: f64,
    pub shadow: bool,
    pub border: bool,
    pub border_shortcut: String,
}
#[derive(Clone, Debug)]
pub struct State {
    pub presentation: Presentation,
    pub preferences: Preferences,
    pub width: u32,
    pub height: u32,
    pub source_text: Option<String>,
    pub hovered: bool,
    pub forced_toolbar: bool,
    pub cursor: Point,
    pub modifiers: Modifiers,
    pub rgb: bool,
}
impl State {
    pub fn new(
        width: u32,
        height: u32,
        source_text: Option<String>,
        preferences: Preferences,
    ) -> Self {
        Self {
            presentation: Presentation {
                opacity: preferences.opacity,
                ..Presentation::default()
            },
            preferences,
            width,
            height,
            source_text,
            hovered: false,
            forced_toolbar: false,
            cursor: Point::default(),
            modifiers: Modifiers::default(),
            rgb: false,
        }
    }
    pub fn dimensions(&self) -> (f64, f64) {
        self.presentation.window_size(self.width, self.height)
    }
    pub fn inspecting(&self) -> bool {
        self.modifiers.alt && !self.modifiers.control && !self.modifiers.meta
    }
    pub fn modifiers(&mut self, modifiers: Modifiers) {
        if modifiers.alt
            && !modifiers.control
            && !modifiers.meta
            && modifiers.shift
            && !self.modifiers.shift
        {
            self.rgb = !self.rgb;
        }
        self.modifiers = modifiers;
    }
    pub fn key(
        &mut self,
        key: Key,
        repeat: bool,
        sampled_color: Option<[u8; 3]>,
    ) -> Option<Action> {
        let m = self.modifiers;
        let command = m.control || m.meta;
        if !repeat
            && shortcut(key, m) == self.preferences.border_shortcut
            && !self.preferences.border_shortcut.is_empty()
        {
            return Some(Action::ToggleBorder);
        }
        if self.inspecting() && matches!(key, Key::Character('c' | 'C')) {
            return sampled_color.map(|color| Action::CopyText(color_text(color, self.rgb)));
        }
        if command && !m.alt {
            let action = match (key, m.shift) {
                (Key::Character('c' | 'C'), false) => Some(Action::Copy),
                (Key::Character('s' | 'S'), false) => Some(Action::Save),
                (Key::Character('s' | 'S'), true) => Some(Action::QuickSave),
                (Key::Character('p' | 'P'), true) => Some(Action::Preferences),
                (Key::Character('v' | 'V'), false) => Some(Action::Replace),
                (Key::Character('w' | 'W'), false) => Some(Action::Close),
                (Key::Character('c' | 'C'), true) => self.source_text.clone().map(Action::CopyText),
                _ => None,
            };
            if action.is_some() {
                return action;
            }
            if !m.shift && matches!(key, Key::Character('+' | '=' | '-')) {
                self.presentation.opacity_by(if key == Key::Character('-') {
                    -24.
                } else {
                    24.
                });
                return None;
            }
        }
        if key == Key::Escape {
            if m.shift && !command && !m.alt {
                return Some(Action::Destroy);
            }
            if self.forced_toolbar && m == Modifiers::default() {
                self.forced_toolbar = false;
                return None;
            }
            return Some(Action::Close);
        }
        if !command {
            match key {
                Key::Character('+' | '=') => self.presentation.zoom_by(0.1),
                Key::Character('-') => self.presentation.zoom_by(-0.1),
                Key::Character('0') => {
                    self.presentation.zoom = 1.;
                    self.presentation.thumbnail = false;
                }
                _ => {}
            }
        }
        if !command && !m.alt && !m.shift {
            match key {
                Key::Space => self.forced_toolbar = !self.forced_toolbar,
                Key::Character('1') => {
                    self.presentation.rotation = (self.presentation.rotation + 90) % 360
                }
                Key::Character('2') => {
                    self.presentation.rotation = (self.presentation.rotation + 270) % 360
                }
                Key::Character('3') => self.presentation.flip_x = !self.presentation.flip_x,
                Key::Character('4') => self.presentation.flip_y = !self.presentation.flip_y,
                Key::Character('5') => self.presentation.grayscale = !self.presentation.grayscale,
                Key::Character('6') => self.presentation.inverted = !self.presentation.inverted,
                _ => {}
            }
        }
        None
    }
    pub fn movement(&self, key: Key) -> Option<(f64, f64)> {
        let m = self.modifiers;
        if m.control || m.meta || m.alt {
            return None;
        }
        let step = if m.shift { 5. } else { 1. };
        match key {
            Key::ArrowLeft => Some((-step, 0.)),
            Key::ArrowRight => Some((step, 0.)),
            Key::ArrowUp => Some((0., -step)),
            Key::ArrowDown => Some((0., step)),
            _ => None,
        }
    }
    pub fn menu(&mut self, tag: u32) -> Option<Action> {
        match tag {
            1 => {
                self.presentation.zoom = 1.;
                self.presentation.thumbnail = false;
                None
            }
            2..=4 => {
                self.presentation.opacity = match tag {
                    2 => 1.,
                    3 => 0.75,
                    _ => 0.5,
                };
                None
            }
            5 => Some(Action::MoveGroup),
            6 => Some(Action::ToggleBorder),
            7 => Some(Action::Close),
            8 => Some(Action::HideGroup),
            9 => Some(Action::Copy),
            10 => Some(Action::Save),
            11 => Some(Action::QuickSave),
            12 => Some(Action::Destroy),
            13 => Some(Action::DestroyGroup),
            14 => Some(Action::CloseAll),
            _ => None,
        }
    }
    pub fn scroll(&mut self, delta: f64) -> bool {
        if self.modifiers.alt || delta == 0. {
            return false;
        }
        if self.modifiers.control || self.modifiers.meta {
            self.presentation.opacity_by(delta.signum() * 24.);
        } else {
            self.presentation.zoom_by(delta.signum() * 0.1);
        }
        true
    }
    pub fn toolbar_rect(&self) -> Rect {
        let (width, _) = self.dimensions();
        Rect {
            x: (width as f32 - 140.).max(0.),
            y: 4.,
            width: 136.,
            height: 30.,
        }
    }
    pub fn toolbar(&self) -> Vec<(Action, Rect, &'static str)> {
        if !self.hovered && !self.forced_toolbar {
            return vec![];
        }
        self.toolbar_layout()
    }
    pub fn toolbar_layout(&self) -> Vec<(Action, Rect, &'static str)> {
        let panel = self.toolbar_rect();
        [
            (Action::Copy, "Copy", 2., 50.),
            (Action::Save, "Save", 52., 50.),
            (Action::Close, "X", 102., 30.),
        ]
        .into_iter()
        .map(|(action, label, x, width)| {
            (
                action,
                Rect {
                    x: panel.x + x,
                    y: panel.y + 2.,
                    width,
                    height: 26.,
                },
                label,
            )
        })
        .collect()
    }
    pub fn lens_rect(&self) -> Rect {
        let (width, height) = self.dimensions();
        let x = self.cursor.x;
        let y = self.cursor.y;
        let left = if x + 174. <= width as f32 {
            x + 14.
        } else {
            x - 174.
        };
        let top = if y + 126. <= height as f32 {
            y + 14.
        } else {
            y - 126.
        };
        Rect {
            x: left.clamp(0., (width as f32 - 160.).max(0.)),
            y: top.clamp(0., (height as f32 - 112.).max(0.)),
            width: 160.,
            height: 112.,
        }
    }
}

pub fn color_text(color: [u8; 3], rgb: bool) -> String {
    if rgb {
        format!("rgb({}, {}, {})", color[0], color[1], color[2])
    } else {
        format!("#{:02X}{:02X}{:02X}", color[0], color[1], color[2])
    }
}
fn shortcut(key: Key, m: Modifiers) -> String {
    let mut parts = Vec::new();
    if m.control {
        parts.push("ctrl".into());
    }
    if m.meta {
        parts.push("meta".into());
    }
    if m.alt {
        parts.push("alt".into());
    }
    if m.shift {
        parts.push("shift".into());
    }
    parts.push(match key {
        Key::Character(c) => c.to_ascii_lowercase().to_string(),
        Key::Function(n) => format!("f{n}"),
        Key::Escape => "escape".into(),
        Key::Space => "space".into(),
        _ => format!("{key:?}").to_ascii_lowercase(),
    });
    parts.join("+")
}

impl State {
    pub fn scene(&self, display: RasterImage, inspection: Option<(RasterImage, [u8; 3])>) -> Scene {
        let (width, height) = self.dimensions();
        let full = Rect {
            x: 0.,
            y: 0.,
            width: width as f32,
            height: height as f32,
        };
        let mut scene = Scene {
            size: Size {
                width: width as f32,
                height: height as f32,
            },
            commands: vec![DrawCommand::Image {
                rect: full,
                image: display,
            }],
        };
        if self.preferences.border {
            scene.commands.push(DrawCommand::StrokeRect {
                rect: Rect {
                    x: 0.5,
                    y: 0.5,
                    width: (width as f32 - 1.).max(0.),
                    height: (height as f32 - 1.).max(0.),
                },
                radius: 0.,
                width: 1.,
                color: [59, 130, 245, 255],
            });
        }
        if !self.toolbar().is_empty() {
            scene.commands.push(DrawCommand::FillRect {
                rect: self.toolbar_rect(),
                radius: 0.,
                color: [236, 236, 236, 255],
            });
        }
        for (_, rect, label) in self.toolbar() {
            scene.commands.push(DrawCommand::FillRect {
                rect,
                radius: 5.,
                color: [244, 246, 249, 240],
            });
            scene.commands.push(text(rect, label, [35, 45, 60, 255]));
        }
        if let Some((image, color)) = inspection.filter(|_| self.inspecting() && self.hovered) {
            let rect = self.lens_rect();
            let (left, top) = (rect.x, rect.y);
            scene.commands.push(DrawCommand::FillRect {
                rect,
                radius: 0.,
                color: [0, 0, 0, 255],
            });
            scene.commands.push(DrawCommand::Image { rect, image });
            scene.commands.push(DrawCommand::Line {
                from: Point {
                    x: left + 80.,
                    y: top,
                },
                to: Point {
                    x: left + 80.,
                    y: top + 112.,
                },
                width: 1.,
                color: [255, 255, 255, 255],
            });
            scene.commands.push(DrawCommand::Line {
                from: Point {
                    x: left,
                    y: top + 56.,
                },
                to: Point {
                    x: left + 160.,
                    y: top + 56.,
                },
                width: 1.,
                color: [255, 255, 255, 255],
            });
            let label = Rect {
                x: left + 4.,
                y: top + 90.,
                width: 152.,
                height: 20.,
            };
            scene.commands.push(DrawCommand::FillRect {
                rect: label,
                radius: 0.,
                color: [0, 0, 0, 255],
            });
            scene.commands.push(text(
                label,
                color_text(color, self.rgb),
                [255, 255, 255, 255],
            ));
        }
        scene
    }
}
fn text(rect: Rect, value: impl Into<String>, color: Color) -> DrawCommand {
    DrawCommand::Text {
        rect,
        text: value.into(),
        size: 11.,
        weight: 400,
        color,
        font: FontFamily::Sans,
        align: TextAlign::Center,
    }
}

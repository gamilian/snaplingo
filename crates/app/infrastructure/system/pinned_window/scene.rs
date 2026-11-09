//! Portable pin rendering and keyboard behavior; native hosts execute effects.
use super::presentation::Presentation;
use snaplingo_ui::{
    Color, DrawCommand, FontFamily, Key, Modifiers, Point, RasterImage, Rect, Scene, Size,
    TextAlign,
};
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Action {
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
pub(super) struct Preferences {
    pub opacity: f64,
    pub shadow: bool,
    pub border: bool,
    pub border_shortcut: String,
}
pub(super) struct PinUi {
    pub presentation: Presentation,
    pub preferences: Preferences,
    pub original: image::RgbaImage,
    display: RasterImage,
    pub source_text: Option<String>,
    pub hovered: bool,
    pub forced_toolbar: bool,
    pub cursor: Point,
    pub modifiers: Modifiers,
    pub rgb: bool,
}
impl PinUi {
    pub fn new(
        bytes: &[u8],
        source_text: Option<String>,
        preferences: Preferences,
    ) -> Result<Self, String> {
        let original = image::load_from_memory(bytes)
            .map_err(|e| e.to_string())?
            .into_rgba8();
        let presentation = Presentation {
            opacity: preferences.opacity,
            ..Presentation::default()
        };
        let display = transform(&original, &presentation)?;
        Ok(Self {
            presentation,
            preferences,
            original,
            display,
            source_text,
            hovered: false,
            forced_toolbar: false,
            cursor: Point::default(),
            modifiers: Modifiers::default(),
            rgb: false,
        })
    }
    pub fn changed(&mut self) -> Result<(), String> {
        self.display = transform(&self.original, &self.presentation)?;
        Ok(())
    }
    pub fn dimensions(&self) -> (f64, f64) {
        self.presentation
            .window_size(self.original.width(), self.original.height())
    }
    pub fn sample(&self) -> ([u8; 3], (u32, u32)) {
        let point = self.presentation.sample_point(
            (self.cursor.x as f64, self.cursor.y as f64),
            self.original.width(),
            self.original.height(),
        );
        let pixel = self.original.get_pixel(point.0, point.1);
        ([pixel[0], pixel[1], pixel[2]], point)
    }
    pub fn color_text(&self) -> String {
        let (color, _) = self.sample();
        if self.rgb {
            format!("rgb({}, {}, {})", color[0], color[1], color[2])
        } else {
            format!("#{:02X}{:02X}{:02X}", color[0], color[1], color[2])
        }
    }
    pub fn modifiers(&mut self, modifiers: Modifiers) {
        if modifiers.alt && modifiers.shift && !self.modifiers.shift {
            self.rgb = !self.rgb;
        }
        self.modifiers = modifiers;
    }
    pub fn key(&mut self, key: Key, repeat: bool) -> Result<Option<Action>, String> {
        let m = self.modifiers;
        let command = m.control || m.meta;
        if !repeat
            && shortcut(key, m) == self.preferences.border_shortcut
            && !self.preferences.border_shortcut.is_empty()
        {
            return Ok(Some(Action::ToggleBorder));
        }
        if m.alt && !command && matches!(key, Key::Character('c' | 'C')) {
            return Ok(Some(Action::CopyText(self.color_text())));
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
                return Ok(action);
            }
            if !m.shift && matches!(key, Key::Character('+' | '=' | '-')) {
                self.presentation.opacity_by(if key == Key::Character('-') {
                    -24.
                } else {
                    24.
                });
                self.changed()?;
                return Ok(None);
            }
        }
        if key == Key::Escape {
            if m.shift && !command && !m.alt {
                return Ok(Some(Action::Destroy));
            }
            if self.forced_toolbar && !command && !m.alt {
                self.forced_toolbar = false;
                return Ok(None);
            }
            return Ok(Some(Action::Close));
        }
        if !command && !m.alt {
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
        self.changed()?;
        Ok(None)
    }
    pub fn toolbar(&self) -> Vec<(Action, Rect, &'static str)> {
        if !self.hovered && !self.forced_toolbar {
            return vec![];
        }
        let (width, _) = self.dimensions();
        let left = (width - 140.).max(0.) as f32;
        [
            (Action::Copy, "复制"),
            (Action::Save, "保存"),
            (Action::Close, "收起"),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, (action, label))| {
            (
                action,
                Rect {
                    x: left + i as f32 * 46.,
                    y: 4.,
                    width: 44.,
                    height: 30.,
                },
                label,
            )
        })
        .collect()
    }
    pub fn scene(&self) -> Scene {
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
            ime_cursor: None,
            content: full,
            commands: vec![DrawCommand::Image {
                rect: full,
                image: self.display.clone(),
            }],
            hits: vec![],
            geometry: vec![],
            content_height: height as f32,
            viewport: full,
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
                color: [80, 90, 108, 255],
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
        if self.modifiers.alt && !self.modifiers.control && !self.modifiers.meta && self.hovered {
            let (_, point) = self.sample();
            let lens_width = 160f32.min(width as f32);
            let lens_height = 112f32.min(height as f32);
            let left = (self.cursor.x + 14.).min(width as f32 - lens_width).max(0.);
            let top = (self.cursor.y + 14.)
                .min(height as f32 - lens_height)
                .max(0.);
            let rect = Rect {
                x: left,
                y: top,
                width: lens_width,
                height: lens_height,
            };
            scene.commands.push(DrawCommand::FillRect {
                rect,
                radius: 5.,
                color: [244, 246, 249, 255],
            });
            let crop = image::imageops::crop_imm(
                &self.original,
                point.0.saturating_sub(10),
                point.1.saturating_sub(7),
                20.min(self.original.width() - point.0.saturating_sub(10)),
                14.min(self.original.height() - point.1.saturating_sub(7)),
            )
            .to_image();
            if let Some(image) =
                RasterImage::from_rgba(crop.width(), crop.height(), crop.into_raw())
            {
                scene.commands.push(DrawCommand::Image {
                    rect: Rect {
                        x: left,
                        y: top,
                        width: lens_width,
                        height: (lens_height - 24.).max(1.),
                    },
                    image,
                });
            }
            scene.commands.push(DrawCommand::Line {
                from: Point {
                    x: left + lens_width / 2.,
                    y: top,
                },
                to: Point {
                    x: left + lens_width / 2.,
                    y: top + lens_height - 24.,
                },
                width: 1.,
                color: [255, 0, 0, 180],
            });
            scene.commands.push(DrawCommand::Line {
                from: Point {
                    x: left,
                    y: top + (lens_height - 24.) / 2.,
                },
                to: Point {
                    x: left + lens_width,
                    y: top + (lens_height - 24.) / 2.,
                },
                width: 1.,
                color: [255, 0, 0, 180],
            });
            scene.commands.push(text(
                Rect {
                    x: left,
                    y: top + lens_height - 24.,
                    width: lens_width,
                    height: 24.,
                },
                self.color_text(),
                [30, 40, 50, 255],
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
fn transform(original: &image::RgbaImage, p: &Presentation) -> Result<RasterImage, String> {
    let mut pixels = original.clone();
    for pixel in pixels.pixels_mut() {
        if p.grayscale {
            let gray =
                (u32::from(pixel[0]) * 299 + u32::from(pixel[1]) * 587 + u32::from(pixel[2]) * 114)
                    / 1000;
            pixel[0] = gray as u8;
            pixel[1] = gray as u8;
            pixel[2] = gray as u8;
        }
        if p.inverted {
            for channel in &mut pixel.0[..3] {
                *channel = 255 - *channel;
            }
        }
        pixel[3] = (f64::from(pixel[3]) * p.opacity).round() as u8;
    }
    if p.flip_x {
        image::imageops::flip_horizontal_in_place(&mut pixels);
    }
    if p.flip_y {
        image::imageops::flip_vertical_in_place(&mut pixels);
    }
    pixels = match p.rotation {
        90 => image::imageops::rotate90(&pixels),
        180 => image::imageops::rotate180(&pixels),
        270 => image::imageops::rotate270(&pixels),
        _ => pixels,
    };
    RasterImage::from_rgba(pixels.width(), pixels.height(), pixels.into_raw())
        .ok_or_else(|| "Invalid pinned image pixels".into())
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
#[cfg(test)]
mod tests {
    use super::*;
    fn ui() -> PinUi {
        let mut bytes = std::io::Cursor::new(Vec::new());
        let image = image::RgbaImage::from_fn(2, 1, |x, _| {
            image::Rgba(if x == 0 {
                [10, 20, 30, 128]
            } else {
                [100, 150, 200, 255]
            })
        });
        image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
        PinUi::new(
            bytes.get_ref(),
            Some("OCR source".into()),
            Preferences {
                opacity: 1.,
                shadow: true,
                border: false,
                border_shortcut: "f8".into(),
            },
        )
        .unwrap()
    }
    #[test]
    fn transforms_preserve_original_sampling_alpha_and_export_size() {
        let mut ui = ui();
        ui.key(Key::Character('1'), false).unwrap();
        assert_eq!(ui.dimensions(), (1., 2.));
        ui.key(Key::Character('3'), false).unwrap();
        ui.key(Key::Character('6'), false).unwrap();
        assert_eq!(ui.original.get_pixel(0, 0).0, [10, 20, 30, 128]);
        assert_eq!(ui.display.rgba()[3], 255);
        ui.presentation.opacity = 0.5;
        ui.changed().unwrap();
        assert_eq!(ui.display.rgba()[3], 128);
        assert_eq!(ui.presentation.export_size(2, 1).width, 2);
    }
    #[test]
    fn shortcuts_keep_operations_and_display_changes_separate() {
        let mut ui = ui();
        assert_eq!(
            ui.key(Key::Function(8), false).unwrap(),
            Some(Action::ToggleBorder)
        );
        ui.modifiers(Modifiers {
            control: true,
            shift: true,
            ..Modifiers::default()
        });
        assert_eq!(
            ui.key(Key::Character('c'), false).unwrap(),
            Some(Action::CopyText("OCR source".into()))
        );
        assert_eq!(
            ui.key(Key::Character('s'), false).unwrap(),
            Some(Action::QuickSave)
        );
        ui.modifiers(Modifiers::default());
        ui.key(Key::Space, false).unwrap();
        assert!(ui.forced_toolbar);
        assert_eq!(ui.key(Key::Escape, false).unwrap(), None);
        assert!(!ui.forced_toolbar);
        assert_eq!(ui.key(Key::Escape, false).unwrap(), Some(Action::Close));
    }
}

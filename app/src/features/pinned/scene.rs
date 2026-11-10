//! Decode and prepare pinned-image pixels outside the pure UI module.
pub(super) use snaplingo_ui::pinned::{Action, Preferences};
use snaplingo_ui::pinned::{Presentation, State};
use snaplingo_ui::{Key, RasterImage, Scene};

pub(super) struct PinUi {
    state: State,
    pub original: image::RgbaImage,
    display: RasterImage,
}
impl std::ops::Deref for PinUi {
    type Target = State;
    fn deref(&self) -> &State {
        &self.state
    }
}
impl std::ops::DerefMut for PinUi {
    fn deref_mut(&mut self) -> &mut State {
        &mut self.state
    }
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
        let state = State::new(
            original.width(),
            original.height(),
            source_text,
            preferences,
        );
        let display = transform(&original, &state.presentation)?;
        Ok(Self {
            state,
            original,
            display,
        })
    }
    pub fn changed(&mut self) -> Result<(), String> {
        self.display = transform(&self.original, &self.presentation)?;
        Ok(())
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
    pub fn key(&mut self, key: Key, repeat: bool) -> Result<Option<Action>, String> {
        let color = self.hovered.then(|| self.sample().0);
        let action = self.state.key(key, repeat, color);
        self.changed()?;
        Ok(action)
    }
    pub fn scene(&self) -> Scene {
        let inspection = if self.inspecting() && self.hovered {
            let (color, point) = self.sample();
            let mut crop = image::RgbaImage::new(20, 14);
            for y in 0..14 {
                for x in 0..20 {
                    let source_x = i64::from(point.0) + i64::from(x) - 10;
                    let source_y = i64::from(point.1) + i64::from(y) - 7;
                    if source_x >= 0
                        && source_y >= 0
                        && source_x < i64::from(self.original.width())
                        && source_y < i64::from(self.original.height())
                    {
                        crop.put_pixel(
                            x,
                            y,
                            *self.original.get_pixel(source_x as u32, source_y as u32),
                        );
                    }
                }
            }
            RasterImage::from_rgba(crop.width(), crop.height(), crop.into_raw())
                .map(|image| (image, color))
        } else {
            None
        };
        self.state.scene(self.display.clone(), inspection)
    }
}
fn transform(original: &image::RgbaImage, p: &Presentation) -> Result<RasterImage, String> {
    let mut pixels = original.clone();
    for pixel in pixels.pixels_mut() {
        if p.grayscale {
            let gray = (0.2126 * f64::from(pixel[0])
                + 0.7152 * f64::from(pixel[1])
                + 0.0722 * f64::from(pixel[2]))
            .round() as u8;
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
}

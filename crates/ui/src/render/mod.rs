//! Shared CPU rendering. Hosts present RGBA pixels and own their event loop;
//! neither layout nor rasterization depends on an operating-system toolkit.
use std::collections::HashMap;
mod text_metrics;
pub use text_metrics::RendererTextMetrics;

use crate::{Color, DrawCommand, FontFamily, Rect, Scene, TextAlign};
use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping, SwashCache, Weight};
use tiny_skia::{
    FillRule, FilterQuality, Mask, Paint, Path, PathBuilder, Pixmap, PixmapPaint, PixmapRef,
    Stroke, Transform,
};

#[derive(Clone, Hash, PartialEq, Eq)]
struct TextKey {
    text: String,
    width: u32,
    height: u32,
    size: u32,
    line_height: u32,
    weight: u16,
    family: u8,
    align: u8,
}

/// Fonts, shaped text and glyph bitmaps survive redraws. A host requests a new
/// frame only after an input, service result, exposure, resize or DPI change.
pub struct Renderer {
    fonts: FontSystem,
    glyphs: SwashCache,
    text: HashMap<TextKey, Buffer>,
}

impl Default for Renderer {
    fn default() -> Self {
        let fonts = system_fonts();
        Self {
            fonts,
            glyphs: SwashCache::new(),
            text: HashMap::new(),
        }
    }
}

fn system_fonts() -> FontSystem {
    static DATABASE: std::sync::LazyLock<(String, cosmic_text::fontdb::Database)> =
        std::sync::LazyLock::new(|| {
            let mut fonts = FontSystem::new();
            fonts
                .db_mut()
                .load_font_source(cosmic_text::fontdb::Source::Binary(std::sync::Arc::new(
                    snaplingo_assets::NOTO_SANS_CJK_SC,
                )));
            // Match the retained CSS's system UI, Paper serif and code stacks.
            // cosmic-text's generic defaults name fonts that may not be installed.
            let candidates = if cfg!(target_os = "macos") {
                [
                    vec![".AppleSystemUIFont", "SF Pro Text", "Helvetica Neue"],
                    vec!["Songti SC", "Georgia"],
                    vec!["SF Mono", "Menlo"],
                ]
            } else if cfg!(target_os = "windows") {
                [
                    vec!["Segoe UI", "Arial"],
                    vec!["Georgia", "Times New Roman"],
                    vec!["Consolas", "Courier New"],
                ]
            } else {
                [
                    vec!["Noto Sans", "DejaVu Sans"],
                    vec!["Noto Serif", "DejaVu Serif"],
                    vec!["Noto Sans Mono", "DejaVu Sans Mono"],
                ]
            };
            let selected: Vec<_> = candidates
                .iter()
                .map(|names| {
                    names
                        .iter()
                        .find(|name| {
                            fonts.db().faces().any(|face| {
                                face.families.iter().any(|(family, _)| family == **name)
                            })
                        })
                        .copied()
                })
                .collect();
            if let Some(name) = selected[0] {
                fonts.db_mut().set_sans_serif_family(name);
            }
            if let Some(name) = selected[1] {
                fonts.db_mut().set_serif_family(name);
            }
            if let Some(name) = selected[2] {
                fonts.db_mut().set_monospace_family(name);
            }
            (fonts.locale().to_owned(), fonts.db().clone())
        });
    // Scan installed fonts once; per-view glyph/layout caches remain local.
    FontSystem::new_with_locale_and_db(DATABASE.0.clone(), DATABASE.1.clone())
}

impl Renderer {
    /// Returns premultiplied RGBA, including transparency for platform alpha
    /// presenters. `device_scale` is independent of the user's UI scale.
    pub fn render(&mut self, scene: &Scene, device_scale: f32) -> Result<Frame, String> {
        self.render_region(
            scene,
            Rect {
                x: 0.0,
                y: 0.0,
                width: scene.size.width,
                height: scene.size.height,
            },
            device_scale,
        )
    }

    /// Embedded native views render a page from the full scene. The same origin
    /// maps their local input back to the shared layout's window coordinates.
    pub fn render_region(
        &mut self,
        scene: &Scene,
        region: Rect,
        device_scale: f32,
    ) -> Result<Frame, String> {
        if !device_scale.is_finite() || device_scale <= 0.0 {
            return Err("Invalid device scale".into());
        }
        if ![region.x, region.y, region.width, region.height]
            .into_iter()
            .all(f32::is_finite)
            || region.x < 0.0
            || region.y < 0.0
            || region.x + region.width > scene.size.width + 0.01
            || region.y + region.height > scene.size.height + 0.01
        {
            return Err("Drawing region lies outside the scene".into());
        }
        // DIP is often a repeating f32 at fractional DPI. Round back to the
        // physical pixel instead of allocating a spurious extra row/column.
        let width = (region.width * device_scale).round();
        let height = (region.height * device_scale).round();
        if !width.is_finite() || !height.is_finite() || width < 1.0 || height < 1.0 {
            return Err("Invalid scene dimensions".into());
        }
        let mut pixels =
            Pixmap::new(width as u32, height as u32).ok_or("Could not allocate UI frame")?;
        let transform = Transform::from_row(
            device_scale,
            0.0,
            0.0,
            device_scale,
            -region.x * device_scale,
            -region.y * device_scale,
        );
        let mut clips = vec![region];
        let mut mask = None;
        for command in &scene.commands {
            match command {
                DrawCommand::Image { rect, image } => {
                    if rect.width > 0. && rect.height > 0. {
                        let source =
                            PixmapRef::from_bytes(image.rgba(), image.width(), image.height())
                                .ok_or("Invalid image pixels")?;
                        pixels.draw_pixmap(
                            0,
                            0,
                            source,
                            &PixmapPaint {
                                quality: FilterQuality::Bilinear,
                                ..PixmapPaint::default()
                            },
                            Transform::from_row(
                                rect.width / image.width() as f32 * device_scale,
                                0.,
                                0.,
                                rect.height / image.height() as f32 * device_scale,
                                (rect.x - region.x) * device_scale,
                                (rect.y - region.y) * device_scale,
                            ),
                            mask.as_ref(),
                        );
                    }
                }
                DrawCommand::PushClip(rect) => {
                    clips.push(clips.last().unwrap().intersect(*rect));
                    mask = Some(clip_mask(&pixels, *clips.last().unwrap(), transform));
                }
                DrawCommand::PopClip => {
                    if clips.len() <= 1 {
                        return Err("Unbalanced drawing clip".into());
                    }
                    clips.pop();
                    mask = if clips.len() == 1 {
                        None
                    } else {
                        Some(clip_mask(&pixels, *clips.last().unwrap(), transform))
                    };
                }
                DrawCommand::FillRect {
                    rect,
                    radius,
                    color,
                } => {
                    if let Some(path) = rounded_rect(*rect, *radius) {
                        pixels.fill_path(
                            &path,
                            &paint(*color),
                            FillRule::Winding,
                            transform,
                            mask.as_ref(),
                        );
                    }
                }
                DrawCommand::StrokeRect {
                    rect,
                    radius,
                    width,
                    color,
                } => {
                    if let Some(path) = rounded_rect(*rect, *radius) {
                        pixels.stroke_path(
                            &path,
                            &paint(*color),
                            &Stroke {
                                width: *width,
                                ..Stroke::default()
                            },
                            transform,
                            mask.as_ref(),
                        );
                    }
                }
                DrawCommand::Line {
                    from,
                    to,
                    width,
                    color,
                } => {
                    let mut path = PathBuilder::new();
                    path.move_to(from.x, from.y);
                    path.line_to(to.x, to.y);
                    if let Some(path) = path.finish() {
                        pixels.stroke_path(
                            &path,
                            &paint(*color),
                            &Stroke {
                                width: *width,
                                ..Stroke::default()
                            },
                            transform,
                            mask.as_ref(),
                        );
                    }
                }
                DrawCommand::Text {
                    rect,
                    text,
                    size,
                    weight,
                    color,
                    font,
                    align,
                } => {
                    self.draw_text(
                        &mut pixels,
                        *rect,
                        text,
                        *size,
                        *size * 1.35,
                        *weight,
                        *color,
                        *font,
                        *align,
                        device_scale,
                        *clips.last().unwrap(),
                        region,
                    );
                }
                DrawCommand::TextBlock {
                    rect,
                    text,
                    size,
                    weight,
                    color,
                    font,
                    line_height,
                } => self.draw_text(
                    &mut pixels,
                    *rect,
                    text,
                    *size,
                    *line_height,
                    *weight,
                    *color,
                    *font,
                    TextAlign::Left,
                    device_scale,
                    *clips.last().unwrap(),
                    region,
                ),
            }
        }
        if clips.len() != 1 {
            return Err("Unbalanced drawing clip".into());
        }
        Ok(Frame { pixels })
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_text(
        &mut self,
        pixels: &mut Pixmap,
        rect: Rect,
        text: &str,
        size: f32,
        line_height: f32,
        weight: u16,
        color: Color,
        family: FontFamily,
        align: TextAlign,
        scale: f32,
        clip: Rect,
        region: Rect,
    ) {
        if text.is_empty() || rect.width <= 0.0 || rect.height <= 0.0 || size <= 0.0 {
            return;
        }
        let key = TextKey {
            text: text.into(),
            width: (rect.width * scale).to_bits(),
            height: (rect.height * scale).to_bits(),
            size: (size * scale).to_bits(),
            line_height: (line_height * scale).to_bits(),
            weight,
            family: match family {
                FontFamily::Sans => 0,
                FontFamily::WindowsUi => 3,
                FontFamily::Serif => 1,
                FontFamily::Mono => 2,
            },
            align: match align {
                TextAlign::Left => 0,
                TextAlign::Center => 1,
                TextAlign::Right => 2,
            },
        };
        if !self.text.contains_key(&key) {
            // Bound retained layouts when text changes repeatedly (search/input).
            if self.text.len() >= 512 {
                self.text.clear();
            }
            let mut buffer = Buffer::new(
                &mut self.fonts,
                Metrics::new(size * scale, line_height * scale),
            );
            buffer.set_size(
                &mut self.fonts,
                Some(rect.width * scale),
                Some(rect.height * scale),
            );
            let family = match family {
                FontFamily::Sans => Family::SansSerif,
                FontFamily::WindowsUi => Family::Name(if cfg!(target_os = "windows") {
                    "Segoe UI"
                } else {
                    "Arial"
                }),
                FontFamily::Serif => Family::Serif,
                FontFamily::Mono => Family::Monospace,
            };
            buffer.set_text(
                &mut self.fonts,
                text,
                &Attrs::new().family(family).weight(Weight(weight)),
                Shaping::Advanced,
            );
            let align = match align {
                TextAlign::Left => cosmic_text::Align::Left,
                TextAlign::Center => cosmic_text::Align::Center,
                TextAlign::Right => cosmic_text::Align::Right,
            };
            for line in &mut buffer.lines {
                line.set_align(Some(align));
            }
            buffer.shape_until_scroll(&mut self.fonts, false);
            self.text.insert(key.clone(), buffer);
        }
        let clip = rect.intersect(clip);
        let width = pixels.width() as i32;
        let height = pixels.height() as i32;
        let origin = (
            ((rect.x - region.x) * scale).round() as i32,
            ((rect.y - region.y) * scale).round() as i32,
        );
        let left = ((clip.x - region.x) * scale).ceil().max(0.0) as i32;
        let top = ((clip.y - region.y) * scale).ceil().max(0.0) as i32;
        let right = ((clip.x - region.x + clip.width) * scale)
            .floor()
            .min(width as f32) as i32;
        let bottom = ((clip.y - region.y + clip.height) * scale)
            .floor()
            .min(height as f32) as i32;
        let data = pixels.data_mut();
        self.text[&key].draw(
            &mut self.fonts,
            &mut self.glyphs,
            cosmic_text::Color::rgba(color[0], color[1], color[2], color[3]),
            |x, y, _, _, glyph_color| {
                let (x, y) = (x + origin.0, y + origin.1);
                if x < left || x >= right || y < top || y >= bottom {
                    return;
                }
                let offset = ((y * width + x) * 4) as usize;
                let [r, g, b, a] = glyph_color.as_rgba();
                // cosmic-text 0.14 emits glyph coverage without the base
                // color's alpha; disabled text must retain the scene opacity.
                let a = (u32::from(a) * u32::from(color[3]) + 127) / 255;
                for (channel, value) in [r, g, b].into_iter().enumerate() {
                    data[offset + channel] = ((u32::from(value) * a
                        + u32::from(data[offset + channel]) * (255 - a)
                        + 127)
                        / 255) as u8;
                }
                data[offset + 3] =
                    (a + (u32::from(data[offset + 3]) * (255 - a) + 127) / 255) as u8;
            },
        );
    }
}

pub struct Frame {
    pixels: Pixmap,
}
impl Frame {
    pub fn width(&self) -> u32 {
        self.pixels.width()
    }
    pub fn height(&self) -> u32 {
        self.pixels.height()
    }
    pub fn rgba(&self) -> &[u8] {
        self.pixels.data()
    }
    pub fn save_png(&self, path: impl AsRef<std::path::Path>) -> Result<(), String> {
        self.pixels
            .save_png(path)
            .map_err(|error| error.to_string())
    }
    /// softbuffer's opaque 0x00RRGGBB format; alpha windows use RGBA above.
    pub fn write_opaque_rgb(&self, output: &mut [u32]) -> Result<(), String> {
        if output.len() != self.pixels.width() as usize * self.pixels.height() as usize {
            return Err("Presenter dimensions differ from the rendered frame".into());
        }
        for (target, source) in output.iter_mut().zip(self.pixels.data().chunks_exact(4)) {
            if source[3] != 255 {
                return Err("Transparent frame requires a platform alpha presenter".into());
            }
            *target = u32::from(source[0]) << 16 | u32::from(source[1]) << 8 | u32::from(source[2]);
        }
        Ok(())
    }
}

fn paint(color: Color) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color_rgba8(color[0], color[1], color[2], color[3]);
    paint
}
fn clip_mask(pixels: &Pixmap, rect: Rect, transform: Transform) -> Mask {
    let mut mask = Mask::new(pixels.width(), pixels.height()).unwrap();
    if let Some(path) = rounded_rect(rect, 0.0) {
        mask.fill_path(&path, FillRule::Winding, false, transform);
    }
    mask
}
fn rounded_rect(rect: Rect, radius: f32) -> Option<Path> {
    if rect.width <= 0.0 || rect.height <= 0.0 {
        return None;
    }
    let mut p = PathBuilder::new();
    let (x, y, w, h) = (rect.x, rect.y, rect.width, rect.height);
    let r = radius.max(0.0).min(w * 0.5).min(h * 0.5);
    let k = r * 0.552_284_8;
    p.move_to(x + r, y);
    p.line_to(x + w - r, y);
    p.cubic_to(x + w - r + k, y, x + w, y + r - k, x + w, y + r);
    p.line_to(x + w, y + h - r);
    p.cubic_to(x + w, y + h - r + k, x + w - r + k, y + h, x + w - r, y + h);
    p.line_to(x + r, y + h);
    p.cubic_to(x + r - k, y + h, x, y + h - r + k, x, y + h - r);
    p.line_to(x, y + r);
    p.cubic_to(x, y + r - k, x + r - k, y, x + r, y);
    p.close();
    p.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_font_coverage_preserves_text_opacity_with_a_reused_layout() {
        let mut renderer = Renderer::default();
        let picture = |alpha| {
            let mut scene = scene(vec![DrawCommand::Text {
                rect: Rect {
                    x: 0.,
                    y: 0.,
                    width: 180.,
                    height: 40.,
                },
                text: "Disabled text 文字".into(),
                size: 18.,
                weight: 400,
                color: [40, 60, 80, alpha],
                font: FontFamily::Sans,
                align: TextAlign::Left,
            }]);
            scene.size = Size {
                width: 180.,
                height: 40.,
            };
            scene.viewport = Rect {
                x: 0.,
                y: 0.,
                width: 180.,
                height: 40.,
            };
            scene
        };
        let opaque = renderer.render(&picture(255), 1.).unwrap();
        let disabled = renderer.render(&picture(96), 1.).unwrap();
        let hidden = renderer.render(&picture(0), 1.).unwrap();
        assert!(opaque.rgba().chunks_exact(4).any(|pixel| pixel[3] > 200));
        for (opaque, disabled) in opaque
            .rgba()
            .chunks_exact(4)
            .zip(disabled.rgba().chunks_exact(4))
        {
            let expected = (u16::from(opaque[3]) * 96 + 127) / 255;
            assert!((i32::from(disabled[3]) - i32::from(expected)).abs() <= 1);
            assert!(disabled[..3].iter().all(|channel| *channel <= disabled[3]));
        }
        assert!(hidden.rgba().iter().all(|channel| *channel == 0));
    }
    #[test]
    fn real_rgba_image_respects_alpha_crop_and_interface_scaling() {
        let image = crate::RasterImage::from_rgba(
            2,
            2,
            vec![
                255, 0, 0, 128, 255, 0, 0, 128, 255, 0, 0, 128, 255, 0, 0, 128,
            ],
        )
        .unwrap();
        assert_eq!(&image.rgba()[..4], &[128, 0, 0, 128]);
        assert!(crate::RasterImage::from_rgba(2, 2, vec![0; 15]).is_none());
        let mut picture = scene(vec![
            DrawCommand::PushClip(Rect {
                x: 3.,
                y: 2.,
                width: 2.,
                height: 4.,
            }),
            DrawCommand::Image {
                rect: Rect {
                    x: 2.,
                    y: 2.,
                    width: 4.,
                    height: 4.,
                },
                image,
            },
            DrawCommand::PopClip,
        ]);
        picture.size = crate::Size {
            width: 8.,
            height: 8.,
        };
        let frame = Renderer::default()
            .render(&picture.scaled(1.5), 2.)
            .unwrap();
        assert_eq!((frame.width(), frame.height()), (24, 24));
        let pixel = |x: usize, y: usize| &frame.rgba()[(y * 24 + x) * 4..(y * 24 + x) * 4 + 4];
        assert_eq!(pixel(12, 12), &[128, 0, 0, 128]);
        assert_eq!(pixel(7, 12), &[0, 0, 0, 0]);
        assert_eq!(pixel(17, 12), &[0, 0, 0, 0]);
    }
    use crate::Size;
    fn scene(commands: Vec<DrawCommand>) -> Scene {
        Scene {
            size: Size {
                width: 20.0,
                height: 20.0,
            },
            commands,
            geometry: vec![],
            ime_cursor: None,
            content: Rect {
                x: 0.0,
                y: 0.0,
                width: 20.0,
                height: 20.0,
            },
            hits: vec![],
            content_height: 20.0,
            viewport: Rect {
                x: 0.0,
                y: 0.0,
                width: 20.0,
                height: 20.0,
            },
        }
    }
    #[test]
    fn nested_clipping_dpi_and_alpha_survive_rasterization() {
        let r = Rect {
            x: 5.0,
            y: 5.0,
            width: 10.0,
            height: 10.0,
        };
        let mut renderer = Renderer::default();
        let frame = renderer
            .render(
                &scene(vec![
                    DrawCommand::PushClip(r),
                    DrawCommand::FillRect {
                        rect: Rect {
                            x: 0.0,
                            y: 0.0,
                            width: 20.0,
                            height: 20.0,
                        },
                        radius: 0.0,
                        color: [255, 0, 0, 128],
                    },
                    DrawCommand::PopClip,
                ]),
                2.0,
            )
            .unwrap();
        assert_eq!((frame.width(), frame.height()), (40, 40));
        assert_eq!(&frame.rgba()[0..4], &[0, 0, 0, 0]);
        let middle = (20 * 40 + 20) * 4;
        assert_eq!(&frame.rgba()[middle..middle + 4], &[128, 0, 0, 128]);
        assert!(frame.write_opaque_rgb(&mut vec![0; 1600]).is_err());
    }
    #[test]
    fn opaque_presenter_requires_exact_dimensions_and_zero_high_byte() {
        let mut renderer = Renderer::default();
        let frame = renderer
            .render(
                &scene(vec![DrawCommand::FillRect {
                    rect: Rect {
                        x: 0.0,
                        y: 0.0,
                        width: 20.0,
                        height: 20.0,
                    },
                    radius: 0.0,
                    color: [12, 34, 56, 255],
                }]),
                1.0,
            )
            .unwrap();
        let mut output = vec![0; 400];
        frame.write_opaque_rgb(&mut output).unwrap();
        assert!(output.iter().all(|pixel| *pixel == 0x000c2238));
        assert!(frame.write_opaque_rgb(&mut []).is_err());
    }
    #[test]
    fn fractional_windows_dpi_round_trips_to_the_exact_presenter_size() {
        let mut renderer = Renderer::default();
        let mut scene = scene(vec![]);
        for scale in [1.2, 1.25, 1.5, 1.75, 2.0] {
            scene.size = crate::Size {
                width: 898.0 / scale,
                height: 650.0 / scale,
            };
            let frame = renderer.render(&scene, scale).unwrap();
            assert_eq!((frame.width(), frame.height()), (898, 650));
        }
    }
    #[test]
    fn embedded_region_matches_the_full_frame_crop() {
        let scene = scene(vec![
            DrawCommand::FillRect {
                rect: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 20.0,
                    height: 20.0,
                },
                radius: 0.0,
                color: [255, 255, 255, 255],
            },
            DrawCommand::PushClip(Rect {
                x: 4.0,
                y: 2.0,
                width: 12.0,
                height: 14.0,
            }),
            DrawCommand::FillRect {
                rect: Rect {
                    x: 3.0,
                    y: 4.0,
                    width: 10.0,
                    height: 10.0,
                },
                radius: 2.0,
                color: [20, 40, 80, 255],
            },
            DrawCommand::PopClip,
        ]);
        let mut renderer = Renderer::default();
        let full = renderer.render(&scene, 2.0).unwrap();
        let region = renderer
            .render_region(
                &scene,
                Rect {
                    x: 5.0,
                    y: 3.0,
                    width: 10.0,
                    height: 12.0,
                },
                2.0,
            )
            .unwrap();
        for row in 0..region.height() as usize {
            let source = ((row + 6) * full.width() as usize + 10) * 4;
            let destination = row * region.width() as usize * 4;
            assert_eq!(
                &region.rgba()[destination..destination + region.width() as usize * 4],
                &full.rgba()[source..source + region.width() as usize * 4]
            );
        }
    }
}

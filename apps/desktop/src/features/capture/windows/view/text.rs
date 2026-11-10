use super::super::super::preview::text_geometry;
use super::*;
use snaplingo_ui::DrawCommand;

impl View {
    pub(super) fn text_layout(&self) -> Option<(Rect, snaplingo_ui::TextLayout)> {
        text_geometry(
            self.editor.as_ref()?,
            self.text.as_ref()?,
            &self.fonts,
            self.scale(),
            &self.geometry.bounds,
        )
    }
    pub(super) fn text_hit(&mut self, extend: bool) -> bool {
        let Some((rect, layout)) = self.text_layout() else {
            return false;
        };
        let text = self.text.as_mut().unwrap();
        let point = Point {
            x: self.cursor.x - rect.x,
            y: self.cursor.y - rect.y,
        };
        let line = self.editor.as_ref().unwrap().styling.font_size as f32 * 1.2;
        let Some((byte, _)) = layout.caret_stops.iter().min_by(|(_, a), (_, b)| {
            let distance = |p: &Point| {
                ((p.y + line / 2. - point.y).abs() / line).floor() * 100000. + (p.x - point.x).abs()
            };
            distance(a).total_cmp(&distance(b))
        }) else {
            return false;
        };
        text.move_to(*byte, extend);
        self.refresh();
        true
    }
    pub(super) fn text_vertical(&mut self, down: bool, extend: bool) {
        let Some((_, layout)) = self.text_layout() else {
            return;
        };
        let text = self.text.as_mut().unwrap();
        let Some((_, at)) = layout
            .caret_stops
            .iter()
            .find(|(byte, _)| *byte == text.caret())
        else {
            return;
        };
        let line = self.editor.as_ref().unwrap().styling.font_size as f32 * 1.2;
        let y = at.y + if down { line } else { -line };
        let byte = layout
            .caret_stops
            .iter()
            .min_by(|(_, a), (_, b)| {
                let distance = |p: &Point| (p.y - y).abs() * 10000. + (p.x - at.x).abs();
                distance(a).total_cmp(&distance(b))
            })
            .map(|(byte, _)| *byte);
        if let Some(byte) = byte {
            text.move_to(byte, extend);
        }
        self.refresh();
    }
    pub(super) fn decorate_text(&mut self) {
        let Some((rect, layout)) = self.text_layout() else {
            self.window.set_ime_allowed(false);
            return;
        };
        self.window.set_ime_allowed(true);
        let text = self.text.as_ref().unwrap();
        let line = self.editor.as_ref().unwrap().styling.font_size as f32 * 1.2;
        let clip = self.snapshot.selection.unwrap_or(rect);
        if let Some(selection) = text.display_selection() {
            for stops in layout.caret_stops.windows(2) {
                let (start, a) = stops[0];
                let (end, b) = stops[1];
                if start >= selection.start && end <= selection.end && start < end {
                    let width = if a.y == b.y {
                        (b.x - a.x).abs()
                    } else {
                        line / 3.
                    };
                    let r = Rect {
                        x: rect.x + a.x,
                        y: rect.y + a.y,
                        width: width.max(2.),
                        height: line,
                    }
                    .intersect(clip);
                    self.frame.scene.commands.push(DrawCommand::FillRect {
                        rect: r,
                        radius: 0.,
                        color: [57, 117, 220, 64],
                    });
                }
            }
        }
        if let Some(caret) = text.display_caret().and_then(|byte| {
            layout
                .caret_stops
                .iter()
                .find(|(offset, _)| *offset == byte)
                .map(|(_, p)| *p)
        }) {
            let rect = Rect {
                x: rect.x + caret.x,
                y: rect.y + caret.y,
                width: 1.5 / self.scale() as f32,
                height: line,
            }
            .intersect(clip);
            self.frame.scene.commands.push(DrawCommand::FillRect {
                rect,
                radius: 0.,
                color: self.snapshot.style.color,
            });
            self.frame.scene.ime_cursor = Some(rect);
            self.window.set_ime_cursor_area(
                PhysicalPosition::new(rect.x as f64 * self.scale(), rect.y as f64 * self.scale()),
                PhysicalSize::new(
                    1u32,
                    (rect.height as f64 * self.scale()).round().max(1.) as u32,
                ),
            );
        }
    }
    pub(super) fn decorate_snapshot(&self, snapshot: &mut CaptureSnapshot) {
        if self.editor.is_some() || self.busy || !self.magnifier {
            return;
        }
        let Some(image) = self
            .images
            .iter()
            .find(|image| image.rect.contains(self.cursor))
        else {
            return;
        };
        let x = ((self.cursor.x - image.rect.x) * image.image.width() as f32 / image.rect.width)
            .floor()
            .clamp(0., image.image.width() as f32 - 1.) as u32;
        let y = ((self.cursor.y - image.rect.y) * image.image.height() as f32 / image.rect.height)
            .floor()
            .clamp(0., image.image.height() as f32 - 1.) as u32;
        let pixels = image.image.rgba();
        let i = ((y * image.image.width() + x) * 4) as usize;
        let color = [pixels[i], pixels[i + 1], pixels[i + 2]];
        snapshot.sampled_color = Some(presentation::color_text(color, self.rgb));
        let point = self.desktop_point();
        snapshot.magnifier_label = presentation::magnifier_label(
            Point {
                x: point.x as f32,
                y: point.y as f32,
            },
            color,
            self.rgb,
        );
        let layout = presentation::MagnifierPresentation::new(snapshot.ui_scale);
        let frame = layout.frame(self.cursor, snapshot.size);
        let sample = presentation::magnifier_sample(
            Point {
                x: x as f32,
                y: y as f32,
            },
            Size {
                width: image.image.width() as f32,
                height: image.image.height() as f32,
            },
            self.preferences.screenshot.magnifier_zoom,
            snapshot.ui_scale,
        );
        let crop_width = sample.source.width as u32;
        let crop_height = sample.source.height as u32;
        let mut crop = Vec::with_capacity((crop_width * crop_height * 4) as usize);
        for cy in 0..crop_height {
            let row = ((sample.source.y as u32 + cy) * image.image.width() + sample.source.x as u32)
                as usize
                * 4;
            crop.extend_from_slice(&pixels[row..row + crop_width as usize * 4]);
        }
        let crop = image::RgbaImage::from_raw(crop_width, crop_height, crop)
            .expect("Magnifier sample has four channels");
        let device_scale = self.scale() as f32;
        let upscaled = image::imageops::resize(
            &crop,
            (sample.destination.width * device_scale).round().max(1.) as u32,
            (sample.destination.height * device_scale).round().max(1.) as u32,
            image::imageops::FilterType::Nearest,
        );
        // Preserve the native lens's exact integer zoom and transparent edge
        // padding instead of stretching a rounded crop to fill the image area.
        let mut lens = image::RgbaImage::new(
            (layout.size.width * device_scale).round().max(1.) as u32,
            (layout.size.height * device_scale).round().max(1.) as u32,
        );
        image::imageops::overlay(
            &mut lens,
            &upscaled,
            (sample.destination.x * device_scale).round() as i64,
            (sample.destination.y * device_scale).round() as i64,
        );
        snapshot.magnifier = RasterImage::from_rgba(lens.width(), lens.height(), lens.into_raw())
            .map(|image| CaptureImage {
                rect: Rect {
                    x: frame.x,
                    y: frame.y,
                    width: layout.size.width,
                    height: layout.size.height,
                },
                image,
            });
    }
}

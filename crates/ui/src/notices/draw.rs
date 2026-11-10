use super::*;
use crate::theme::{palette, Palette};
use crate::{DrawCommand, FontFamily, Size, TextAlign, TextMeasurer};
fn r(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect {
        x,
        y,
        width: width.max(0.),
        height: height.max(0.),
    }
}
struct Paint {
    scene: Scene,
    ime_cursor: Option<Rect>,
    controls: Vec<NoticesControl>,
    clip: Rect,
    p: Palette,
}
impl Paint {
    fn fill(&mut self, rect: Rect, color: crate::Color) {
        self.scene.commands.push(DrawCommand::FillRect {
            rect,
            radius: 6.,
            color,
        });
    }
    fn text(&mut self, rect: Rect, text: impl Into<String>, color: crate::Color, size: f32) {
        self.scene.commands.push(DrawCommand::Text {
            rect,
            text: text.into(),
            size,
            weight: 400,
            color,
            font: FontFamily::Sans,
            align: TextAlign::Left,
        });
    }
    fn control(&mut self, target: NoticesTarget, rect: Rect, label: String, value: Option<String>) {
        self.controls.push(NoticesControl {
            target,
            rect,
            hit_rect: rect.intersect(self.clip),
            label,
            value,
        });
    }
    fn button(&mut self, target: NoticesTarget, rect: Rect, label: &str) {
        self.fill(rect, self.p.sunken);
        self.text(
            r(rect.x + 9., rect.y + 7., rect.width - 18., 18.),
            label,
            self.p.ink,
            11.,
        );
        self.control(target, rect, label.into(), None);
    }
}
impl NoticesUi {
    pub fn frame(
        &mut self,
        size: Size,
        scale: f32,
        metrics: &mut impl TextMeasurer,
    ) -> NoticesFrame {
        self.scale = if scale.is_finite() {
            scale.clamp(0.5, 3.)
        } else {
            1.
        };
        let size = Size {
            width: size.width / self.scale,
            height: size.height / self.scale,
        };
        let p = palette(self.appearance.theme, self.appearance.dark);
        let full = r(0., 0., size.width, size.height);
        let body = r(20., 140., size.width - 40., size.height - 160.);
        let mut b = Paint {
            scene: Scene {
                size,
                commands: vec![],
            },
            ime_cursor: None,
            controls: vec![],
            clip: full,
            p,
        };
        b.fill(full, p.surface);
        b.text(
            r(24., 18., size.width - 120., 24.),
            "第三方组件与许可",
            p.ink,
            16.,
        );
        b.text(
            r(24., 46., size.width - 120., 20.),
            "查看组件来源和随附许可声明。",
            p.muted,
            11.,
        );
        b.button(
            NoticesTarget::Close,
            r(size.width - 76., 20., 52., 32.),
            "关闭",
        );
        let search = r(22., 90., size.width - 44., 34.);
        b.fill(search, p.canvas);
        let display = self.search.display_text();
        b.text(
            r(32., 99., size.width - 98., 20.),
            if display.is_empty() {
                "搜索组件或许可证…"
            } else {
                &display
            },
            if display.is_empty() { p.faint } else { p.ink },
            12.,
        );
        b.control(
            NoticesTarget::Search,
            search,
            "搜索组件或许可证".into(),
            Some(self.search.text().into()),
        );
        if !display.is_empty() {
            b.button(
                NoticesTarget::ClearSearch,
                r(size.width - 54., 94., 26., 26.),
                "×",
            );
        }
        if self.focus == Some(NoticesTarget::Search) {
            let measured = metrics.measure(&display, 12., 400, FontFamily::Sans);
            let x = |byte| {
                measured
                    .caret_stops
                    .iter()
                    .find(|(i, _)| *i == byte)
                    .map_or(0., |(_, x)| *x)
            };
            if let Some(byte) = self.search.display_caret() {
                let caret = r(32. + x(byte), 97., 1., 20.);
                b.fill(caret, p.accent);
                b.ime_cursor = Some(caret);
            }
        }
        b.scene.commands.push(DrawCommand::PushClip(body));
        b.clip = body;
        let mut y = body.y - self.scroll;
        if let Some(index) = self.selected {
            let component = self.components[index].clone();
            b.button(NoticesTarget::Back, r(22., y, 136., 30.), "← 返回组件列表");
            y += 44.;
            b.text(r(24., y, body.width - 8., 26.), &component.name, p.ink, 16.);
            y += 32.;
            b.text(
                r(24., y, body.width - 8., 22.),
                format!("{} · {}", component.version, component.license),
                p.muted,
                12.,
            );
            y += 28.;
            b.text(
                r(24., y, body.width - 8., 26.),
                &component.source,
                p.accent,
                11.,
            );
            b.control(
                NoticesTarget::Source,
                r(24., y, body.width - 8., 26.),
                "打开组件来源".into(),
                Some(component.source.clone()),
            );
            y += 36.;
            let width = body.width - 32.;
            let layout =
                metrics.line_layout(&component.text, width, 11., 400, FontFamily::Mono, 17.);
            let height = layout.height.max(60.);
            let rect = r(36., y + 16., width, height);
            b.fill(r(22., y, body.width, height + 32.), p.sunken);
            self.layout = layout;
            self.text_rect = rect;
            if self.focus == Some(NoticesTarget::License) {
                if let Some(edit) = &self.editor {
                    let selection = edit.selection();
                    for pair in self.layout.caret_stops.windows(2) {
                        let (byte, start) = pair[0];
                        let (_, end) = pair[1];
                        if selection.contains(&byte) {
                            b.fill(
                                r(
                                    rect.x + start.x,
                                    rect.y + start.y,
                                    if start.y == end.y {
                                        (end.x - start.x).abs().max(1.)
                                    } else {
                                        (rect.width - start.x).max(1.)
                                    },
                                    17.,
                                ),
                                p.accent_soft,
                            );
                        }
                    }
                }
            }
            b.scene.commands.push(DrawCommand::TextBlock {
                rect,
                text: component.text.clone(),
                size: 11.,
                weight: 400,
                color: p.muted,
                font: FontFamily::Mono,
                line_height: 17.,
            });
            b.control(
                NoticesTarget::License,
                rect,
                "许可全文".into(),
                Some(component.text),
            );
            y += height + 48.;
        } else {
            let columns = [
                (0., 0.46, "组件"),
                (0.46, 0.20, "版本"),
                (0.66, 0.34, "许可"),
            ];
            for (start, width, title) in columns {
                b.text(
                    r(
                        30. + body.width * start,
                        y + 8.,
                        body.width * width - 12.,
                        20.,
                    ),
                    title,
                    p.muted,
                    11.,
                );
            }
            y += 36.;
            if self.filtered.is_empty() {
                b.text(
                    r(32., y + 20., body.width - 24., 28.),
                    "没有匹配的组件",
                    p.muted,
                    13.,
                );
                y += 70.;
            }
            for index in self.filtered.clone() {
                let c = &self.components[index];
                let row = r(22., y, body.width, 40.);
                b.fill(
                    row,
                    if self.focus == Some(NoticesTarget::Component(index)) {
                        p.accent_soft
                    } else {
                        p.surface
                    },
                );
                for (start, width, text) in [
                    (0., 0.46, c.name.as_str()),
                    (0.46, 0.20, c.version.as_str()),
                    (0.66, 0.34, c.license.as_str()),
                ] {
                    b.text(
                        r(
                            30. + body.width * start,
                            y + 12.,
                            body.width * width - 12.,
                            21.,
                        ),
                        text,
                        p.ink,
                        11.,
                    );
                }
                b.control(
                    NoticesTarget::Component(index),
                    row,
                    format!("{} {} {}", c.name, c.version, c.license),
                    None,
                );
                y += 41.;
            }
        }
        b.scene.commands.push(DrawCommand::PopClip);
        self.max_scroll = (y + self.scroll - body.y - body.height).max(0.);
        self.scroll = self.scroll.min(self.max_scroll);
        self.controls = b
            .controls
            .into_iter()
            .map(|mut c| {
                c.rect = c.rect.scaled(self.scale);
                c.hit_rect = c.hit_rect.scaled(self.scale);
                c
            })
            .collect();
        NoticesFrame {
            scene: b.scene.scaled(self.scale),
            ime_cursor: b.ime_cursor.map(|rect| rect.scaled(self.scale)),
            controls: self.controls.clone(),
        }
    }
}

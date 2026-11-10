use super::{
    icons::{self, Icon},
    *,
};
use crate::{
    theme::{palette, Palette},
    Color, DrawCommand, FontFamily, Size, TextAlign, TextMeasurer,
};
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
    controls: Vec<ResultControl>,
    clip: Rect,
    p: Palette,
}
impl Paint {
    fn fill(&mut self, rect: Rect, radius: f32, color: Color) {
        self.scene.commands.push(DrawCommand::FillRect {
            rect,
            radius,
            color,
        });
    }
    fn stroke(&mut self, rect: Rect, radius: f32, color: Color) {
        self.scene.commands.push(DrawCommand::StrokeRect {
            rect,
            radius,
            width: 1.,
            color,
        });
    }
    fn text(&mut self, rect: Rect, text: impl Into<String>, size: f32, weight: u16, color: Color) {
        self.scene.commands.push(DrawCommand::Text {
            rect,
            text: text.into(),
            size,
            weight,
            color,
            font: FontFamily::Sans,
            align: TextAlign::Left,
        });
    }
    fn centered(&mut self, rect: Rect, text: impl Into<String>, size: f32, color: Color) {
        self.scene.commands.push(DrawCommand::Text {
            rect,
            text: text.into(),
            size,
            weight: 500,
            color,
            font: FontFamily::Sans,
            align: TextAlign::Center,
        });
    }
    fn control(
        &mut self,
        target: ResultTarget,
        rect: Rect,
        label: &str,
        role: ResultRole,
        value: Option<String>,
        enabled: bool,
    ) {
        self.controls.push(ResultControl {
            target,
            rect,
            hit_rect: rect.intersect(self.clip),
            label: label.into(),
            role,
            value,
            enabled,
        });
    }
    fn icon(
        &mut self,
        target: ResultTarget,
        rect: Rect,
        label: &str,
        icon: Icon,
        enabled: bool,
        bordered: bool,
    ) {
        if bordered {
            self.fill(rect, 7., self.p.surface);
            self.stroke(rect, 7., self.p.line);
        }
        let size = if rect.width >= 28. { 16. } else { 15. };
        icons::draw(
            &mut self.scene,
            r(
                rect.x + (rect.width - size) / 2.,
                rect.y + (rect.height - size) / 2.,
                size,
                size,
            ),
            icon,
            if enabled { self.p.muted } else { self.p.faint },
        );
        self.control(target, rect, label, ResultRole::Button, None, enabled);
    }
}
fn moved(rect: Rect, origin: Rect) -> Rect {
    r(
        rect.x + origin.x,
        rect.y + origin.y,
        rect.width,
        rect.height,
    )
}
fn text_rect(rect: Rect) -> Rect {
    r(
        rect.x + 10.,
        rect.y + 8.,
        rect.width - 20.,
        rect.height - 16.,
    )
}
fn line_height(text: &str, source: bool) -> f32 {
    text_size(text, source) + 3.
}
fn measured_height(metrics: &mut impl TextMeasurer, text: &str, width: f32, source: bool) -> f32 {
    metrics
        .line_layout(
            text,
            (width - 20.).max(1.),
            text_size(text, source),
            400,
            FontFamily::Sans,
            line_height(text, source),
        )
        .height
        .ceil()
        + 18.
}
impl Paint {
    fn card(&mut self, rect: Rect) {
        self.fill(rect, 8., self.p.surface);
        self.stroke(rect, 8., self.p.line);
    }
    fn button(&mut self, button: &ResultButton, rect: Rect) {
        let icon = match button.target {
            ResultTarget::FavoriteAll
            | ResultTarget::FavoriteOcr
            | ResultTarget::FavoriteResult(_) => Some(Icon::Favorite),
            ResultTarget::CopySource | ResultTarget::CopyOcr | ResultTarget::CopyResult(_) => {
                Some(Icon::Copy)
            }
            ResultTarget::SpeakSource | ResultTarget::SpeakOcr | ResultTarget::SpeakResult(_) => {
                Some(Icon::Volume)
            }
            ResultTarget::Translate | ResultTarget::Retry(_) => Some(Icon::Retry),
            ResultTarget::ClearSource | ResultTarget::ClearOcr => Some(Icon::Clear),
            _ => None,
        };
        if let Some(icon) = icon {
            self.icon(
                button.target.clone(),
                rect,
                button.label,
                icon,
                button.enabled,
                false,
            );
        } else {
            self.fill(rect, 5., self.p.surface);
            self.stroke(rect, 5., self.p.line);
            self.centered(
                r(
                    rect.x + 4.,
                    rect.y + (rect.height - 16.) / 2.,
                    rect.width - 8.,
                    16.,
                ),
                button.label,
                12.,
                if button.enabled {
                    self.p.ink
                } else {
                    self.p.faint
                },
            );
            self.control(
                button.target.clone(),
                rect,
                button.label,
                ResultRole::Button,
                None,
                button.enabled,
            );
        }
    }
}
impl ResultUi {
    pub fn frame(
        &mut self,
        size: Size,
        scale: f32,
        metrics: &mut impl TextMeasurer,
    ) -> ResultFrame {
        self.scale = if scale.is_finite() {
            scale.clamp(0.5, 3.)
        } else {
            1.
        };
        self.text_regions.clear();
        let logical = Size {
            width: size.width / self.scale,
            height: size.height / self.scale,
        };
        let full = r(0., 0., logical.width, logical.height);
        let header = ResultLayout::header(logical);
        let width = (logical.width - 28.).max(1.);
        let measured = ResultMeasurements {
            source: measured_height(metrics, &self.snapshot.source, width, true),
            ocr: measured_height(metrics, &self.snapshot.ocr_text, width, true),
            error: self
                .snapshot
                .error
                .as_ref()
                .map_or(0., |text| measured_height(metrics, text, width + 4., false)),
            rows: self
                .snapshot
                .rows
                .iter()
                .map(|row| {
                    measured_height(
                        metrics,
                        row.presentation(self.snapshot.favorite_pending).text,
                        width,
                        false,
                    )
                })
                .collect(),
            image: self.snapshot.ocr_image.as_ref().map(|image| Size {
                width: image.width() as f32,
                height: image.height() as f32,
            }),
        };
        let layout = self.layout(logical.width, &measured);
        self.max_scroll = (layout.content_height - header.viewport.height).max(0.);
        self.scroll = self.scroll.min(self.max_scroll);
        let document = r(
            header.viewport.x,
            header.viewport.y - self.scroll,
            header.viewport.width,
            layout.content_height,
        );
        let p = palette(
            self.snapshot.appearance.theme,
            self.snapshot.appearance.dark,
        );
        let mut b = Paint {
            scene: Scene {
                size: logical,
                commands: vec![],
            },
            ime_cursor: None,
            controls: vec![],
            clip: full,
            p,
        };
        b.fill(full, 12., p.canvas);
        b.stroke(full, 12., p.line);
        b.fill(r(0., 48., logical.width, 1.), 0., p.line);
        b.control(
            ResultTarget::Header,
            r(0., 0., logical.width, 48.),
            "拖动窗口",
            ResultRole::Button,
            None,
            true,
        );
        if self.pinned {
            b.fill(header.pin, 5., p.accent_soft);
        }
        b.icon(
            ResultTarget::Pin,
            header.pin,
            if self.pinned {
                "取消固定"
            } else {
                "固定窗口"
            },
            Icon::Pin,
            true,
            false,
        );
        b.icon(
            ResultTarget::Close,
            header.close,
            "关闭",
            Icon::Close,
            true,
            false,
        );
        let presentation = self.snapshot.presentation();
        b.text(header.title, presentation.title, 16., 700, p.ink);
        b.text(header.subtitle, &presentation.subtitle, 12., 400, p.muted);
        b.scene
            .commands
            .push(DrawCommand::PushClip(header.viewport));
        b.clip = header.viewport;
        let mut cards = vec![];
        if let Some(source) = &layout.source {
            let outer = moved(source.frame, document);
            b.card(outer);
            let control = moved(source.text, outer);
            self.text_field(
                &mut b,
                ResultTarget::Source,
                control,
                text_rect(control),
                "原文",
                self.snapshot.source.clone(),
                text_size(&self.snapshot.source, true),
                line_height(&self.snapshot.source, true),
                metrics,
            );
            if self.snapshot.source.is_empty() {
                b.text(
                    moved(source.placeholder, outer),
                    "输入需要翻译的文本…",
                    14.,
                    400,
                    p.muted,
                );
            }
            b.text(
                moved(source.metadata, outer),
                &presentation.source_metadata,
                11.,
                400,
                p.muted,
            );
            for (button, rect) in presentation.source_buttons.iter().zip(source.buttons) {
                b.button(button, moved(rect, outer));
            }
        }
        if let Some(rect) = layout.collapsed_source {
            let rect = moved(rect, document);
            b.card(rect);
            b.text(
                r(rect.x + 8., rect.y + 12., rect.width - 16., 20.),
                &presentation.collapsed_source,
                12.,
                400,
                p.ink,
            );
            b.control(
                ResultTarget::ToggleSource,
                rect,
                "展开原文",
                ResultRole::Button,
                None,
                true,
            );
        }
        if let Some([source, swap, target]) = layout.languages {
            for (source_side, rect, code) in [
                (true, source, &self.snapshot.source_language),
                (false, target, &self.snapshot.target_language),
            ] {
                let rect = moved(rect, document);
                b.fill(rect, 5., p.surface);
                b.stroke(rect, 5., p.line);
                let label = LANGUAGES
                    .iter()
                    .find(|(id, _)| *id == code)
                    .map_or(code.as_str(), |(_, label)| *label);
                b.centered(
                    r(rect.x + 4., rect.y + 10., rect.width - 24., 20.),
                    label,
                    13.,
                    p.ink,
                );
                icons::draw(
                    &mut b.scene,
                    r(rect.x + rect.width - 22., rect.y + 10., 16., 16.),
                    Icon::Down,
                    p.muted,
                );
                b.control(
                    if source_side {
                        ResultTarget::SourceLanguage
                    } else {
                        ResultTarget::TargetLanguage
                    },
                    rect,
                    if source_side {
                        "源语言"
                    } else {
                        "目标语言"
                    },
                    ResultRole::Button,
                    Some(code.clone()),
                    true,
                );
            }
            b.icon(
                ResultTarget::Swap,
                moved(swap, document),
                "切换语言",
                Icon::Swap,
                true,
                false,
            );
        }
        if let Some(rect) = layout.error {
            let rect = moved(rect, document);
            b.scene.commands.push(DrawCommand::TextBlock {
                rect: text_rect(rect),
                text: self.snapshot.error.clone().unwrap(),
                size: text_size(self.snapshot.error.as_deref().unwrap(), false),
                weight: 400,
                color: p.danger,
                font: FontFamily::Sans,
                line_height: line_height(self.snapshot.error.as_deref().unwrap(), false),
            });
        }
        for (row, layout) in self.snapshot.rows.clone().iter().zip(&layout.rows) {
            let row_presentation = row.presentation(self.snapshot.favorite_pending);
            let expanded = self.row_expanded(&row.id);
            let outer = moved(layout.frame, document);
            cards.push(outer);
            b.card(outer);
            let title = moved(layout.title, outer);
            b.text(
                title,
                format!("{} {}", if expanded { "▾" } else { "▸" }, row.name),
                13.,
                700,
                p.ink,
            );
            b.control(
                ResultTarget::ToggleRow(row.id.clone()),
                title,
                &format!("{} {}", if expanded { "收起" } else { "展开" }, row.name),
                ResultRole::Button,
                None,
                true,
            );
            b.text(
                moved(layout.status, outer),
                row_presentation.status,
                11.,
                400,
                if row_presentation.error {
                    p.danger
                } else {
                    p.muted
                },
            );
            for (button, rect) in row_presentation.buttons.iter().zip(layout.buttons) {
                b.button(button, moved(rect, outer));
            }
            if expanded {
                let control = moved(layout.text, outer);
                self.text_field(
                    &mut b,
                    ResultTarget::ResultText(row.id.clone()),
                    control,
                    text_rect(control),
                    &row.name,
                    row_presentation.text.into(),
                    text_size(row_presentation.text, false),
                    line_height(row_presentation.text, false),
                    metrics,
                );
            }
        }
        if let Some(ocr) = &layout.ocr {
            if let Some(rect) = ocr.language {
                b.text(
                    moved(rect, document),
                    &presentation.ocr_language,
                    11.,
                    400,
                    p.muted,
                );
            }
            b.text(
                moved(ocr.image_title, document),
                presentation.image_title,
                13.,
                400,
                p.muted,
            );
            if let (Some(rect), Some(image)) = (ocr.image, &self.snapshot.ocr_image) {
                let rect = moved(rect, document);
                let scale =
                    (rect.width / image.width() as f32).min(rect.height / image.height() as f32);
                let width = image.width() as f32 * scale;
                let height = image.height() as f32 * scale;
                b.scene.commands.push(DrawCommand::Image {
                    rect: r(
                        rect.x + (rect.width - width) / 2.,
                        rect.y + (rect.height - height) / 2.,
                        width,
                        height,
                    ),
                    image: image.clone(),
                });
            }
            b.button(&presentation.upload, moved(ocr.upload, document));
            b.text(
                moved(ocr.title, document),
                presentation.ocr_title,
                13.,
                400,
                p.muted,
            );
            let outer = moved(ocr.frame, document);
            b.card(outer);
            let control = moved(ocr.text, outer);
            self.text_field(
                &mut b,
                ResultTarget::OcrText,
                control,
                text_rect(control),
                "识别文本",
                self.snapshot.ocr_text.clone(),
                text_size(&self.snapshot.ocr_text, true),
                line_height(&self.snapshot.ocr_text, true),
                metrics,
            );
            if self.snapshot.ocr_text.is_empty() {
                b.text(
                    moved(ocr.placeholder, outer),
                    "OCR 结果…",
                    14.,
                    400,
                    p.muted,
                );
            }
            if !self.snapshot.ocr_tokens.is_empty() {
                let rect = moved(ocr.fragments, outer);
                b.fill(rect, 5., p.surface);
                b.stroke(rect, 5., p.line);
                b.text(
                    r(rect.x + 8., rect.y + 9., rect.width - 32., 20.),
                    format!("推荐复制片段 · {} 个", self.snapshot.ocr_tokens.len()),
                    12.,
                    400,
                    p.ink,
                );
                icons::draw(
                    &mut b.scene,
                    r(rect.x + rect.width - 24., rect.y + 9., 16., 16.),
                    Icon::Down,
                    p.muted,
                );
                b.control(
                    ResultTarget::CopyFragments,
                    rect,
                    "推荐复制片段",
                    ResultRole::Button,
                    None,
                    true,
                );
            }
            b.text(
                moved(ocr.metadata, outer),
                &presentation.ocr_metadata,
                11.,
                400,
                p.muted,
            );
            for (button, rect) in presentation.ocr_buttons.iter().zip(ocr.buttons) {
                b.button(button, moved(rect, outer));
            }
        }
        b.scene.commands.push(DrawCommand::PopClip);
        b.clip = full;
        let menu = if let Some(source) = self.menu {
            Some((
                if source {
                    ResultTarget::SourceLanguage
                } else {
                    ResultTarget::TargetLanguage
                },
                LANGUAGES
                    .iter()
                    .map(|(code, label)| {
                        (
                            ResultTarget::Language(source, (*code).into()),
                            (*label).to_owned(),
                        )
                    })
                    .collect::<Vec<_>>(),
            ))
        } else if self.fragment_menu {
            Some((
                ResultTarget::CopyFragments,
                self.snapshot
                    .ocr_tokens
                    .iter()
                    .map(|token| {
                        (
                            ResultTarget::CopyToken(token.value.clone()),
                            format!("{}  {}", token.label, token.value),
                        )
                    })
                    .collect::<Vec<_>>(),
            ))
        } else {
            None
        };
        if let Some((target, options)) = menu {
            if let Some(anchor) = b
                .controls
                .iter()
                .find(|control| control.target == target)
                .map(|control| control.rect)
            {
                let natural = options.len() as f32 * 28. + 8.;
                let height = natural.min((logical.height - 16.).max(1.));
                self.menu_max_scroll = (natural - height).max(0.);
                self.menu_scroll = self.menu_scroll.clamp(0., self.menu_max_scroll);
                let top =
                    (anchor.y + anchor.height + 4.).min((logical.height - height - 8.).max(4.));
                b.fill(r(anchor.x, top, anchor.width, height), 7., p.raised);
                b.stroke(r(anchor.x, top, anchor.width, height), 7., p.line);
                let popup = r(anchor.x, top, anchor.width, height);
                b.scene.commands.push(DrawCommand::PushClip(popup));
                b.clip = popup;
                for (index, (target, label)) in options.into_iter().enumerate() {
                    let rect = r(
                        anchor.x + 4.,
                        top + 4. + index as f32 * 28. - self.menu_scroll,
                        anchor.width - 8.,
                        28.,
                    );
                    if self.focus.as_ref() == Some(&target) {
                        b.fill(rect, 4., p.accent_soft);
                    }
                    b.text(
                        r(rect.x + 8., rect.y + 6., rect.width - 16., 20.),
                        &label,
                        12.,
                        400,
                        p.ink,
                    );
                    b.control(target, rect, &label, ResultRole::MenuItem, None, true);
                }
                b.scene.commands.push(DrawCommand::PopClip);
            }
        }
        self.controls = b
            .controls
            .iter()
            .cloned()
            .map(|mut control| {
                control.rect = control.rect.scaled(self.scale);
                control.hit_rect = control.hit_rect.scaled(self.scale);
                control
            })
            .collect();
        ResultFrame {
            scene: b.scene.scaled(self.scale),
            ime_cursor: b.ime_cursor.map(|rect| rect.scaled(self.scale)),
            content_height: (layout.content_height + 72.) * self.scale,
            viewport: header.viewport.scaled(self.scale),
            controls: self.controls.clone(),
            panel: full.scaled(self.scale),
            cards: cards
                .into_iter()
                .map(|rect| rect.scaled(self.scale))
                .collect(),
        }
    }
    fn text_field(
        &mut self,
        b: &mut Paint,
        target: ResultTarget,
        control: Rect,
        rect: Rect,
        label: &str,
        value: String,
        size: f32,
        line: f32,
        metrics: &mut impl TextMeasurer,
    ) {
        let focused = self.focus.as_ref() == Some(&target);
        let display = if focused {
            self.editor
                .as_ref()
                .map_or_else(|| value.clone(), |e| e.display_text())
        } else {
            value.clone()
        };
        let layout = metrics.line_layout(&display, rect.width, size, 400, FontFamily::Sans, line);
        self.text_regions
            .push((target.clone(), rect, layout.clone()));
        if focused {
            if let Some(edit) = &self.editor {
                if let Some(selection) = edit.display_selection() {
                    for pair in layout.caret_stops.windows(2) {
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
                                    line,
                                ),
                                0.,
                                b.p.accent_soft,
                            );
                        }
                    }
                }
                if let Some((_, caret)) = edit
                    .display_caret()
                    .and_then(|byte| layout.caret_stops.iter().find(|(i, _)| *i == byte))
                {
                    let caret = r(rect.x + caret.x, rect.y + caret.y, 1., line);
                    b.fill(caret, 0., b.p.accent);
                    if target == ResultTarget::Source {
                        b.ime_cursor = Some(caret);
                    }
                }
            }
        }
        b.scene.commands.push(DrawCommand::TextBlock {
            rect,
            text: display,
            size,
            weight: 400,
            color: if matches!(target, ResultTarget::ResultText(_))
                && self.snapshot.rows.iter().any(|r| {
                    target == ResultTarget::ResultText(r.id.clone()) && r.status == RowStatus::Error
                }) {
                b.p.danger
            } else {
                b.p.ink
            },
            font: FontFamily::Sans,
            line_height: line,
        });
        b.control(
            target.clone(),
            control,
            label,
            if target == ResultTarget::Source {
                ResultRole::MultilineInput
            } else {
                ResultRole::Text
            },
            Some(value),
            true,
        );
    }
}

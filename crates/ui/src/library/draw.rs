use super::*;
use crate::theme::{palette, Palette};
use crate::{
    Color, DrawCommand, FontFamily, Page, Point, Rect, Scene, Size, TextAlign, TextMeasurer, Theme,
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
    controls: Vec<LibraryControl>,
    p: Palette,
    clip: Rect,
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
    fn block(&mut self, rect: Rect, text: &str, size: f32, line_height: f32, color: Color) {
        self.scene.commands.push(DrawCommand::TextBlock {
            rect,
            text: text.into(),
            size,
            weight: 400,
            color,
            font: FontFamily::Sans,
            line_height,
        });
    }
    fn line(&mut self, x: f32, y: f32, width: f32, color: Color) {
        self.fill(r(x, y, width, 1.), 0., color);
    }
    fn control(
        &mut self,
        target: LibraryTarget,
        rect: Rect,
        role: LibraryRole,
        label: &str,
        enabled: bool,
        selected: bool,
    ) {
        self.controls.push(LibraryControl {
            target,
            rect,
            hit_rect: rect.intersect(self.clip),
            role,
            label: label.into(),
            enabled,
            selected,
        });
    }
    fn button(
        &mut self,
        target: LibraryTarget,
        rect: Rect,
        label: &str,
        enabled: bool,
        selected: bool,
    ) {
        let p = self.p;
        self.fill(rect, 7., if selected { p.accent_soft } else { p.surface });
        self.stroke(rect, 7., if selected { p.accent } else { p.line });
        self.text(
            r(rect.x + 10., rect.y + 6., rect.width - 20., 16.),
            label,
            10.5,
            500,
            if enabled {
                if selected {
                    p.accent
                } else {
                    p.muted
                }
            } else {
                p.faint
            },
        );
        self.control(target, rect, LibraryRole::Button, label, enabled, selected);
    }
    fn clip(&mut self, rect: Rect) {
        self.clip = rect;
        self.scene.commands.push(DrawCommand::PushClip(rect));
    }
    fn unclip(&mut self) {
        self.scene.commands.push(DrawCommand::PopClip);
        self.clip = self.scene.content;
    }
}

impl LibraryUi {
    pub fn frame(
        &mut self,
        size: Size,
        scale: f32,
        metrics: &mut impl TextMeasurer,
    ) -> LibraryFrame {
        let logical = self.logical_size(size, scale);
        let page = if self.snapshot.kind == LibraryKind::History {
            Page::History
        } else {
            Page::Favorites
        };
        let scene = crate::canvas::shell_frame(&self.snapshot.shell, page, logical);
        let p = palette(self.snapshot.shell.theme, self.snapshot.shell.dark);
        let content = scene.content;
        let narrow = logical.width <= 700.;
        let list_width = ((content.width * if narrow { 0.4 } else { 0.36 })
            .max(if narrow { 185. } else { 250. })
            .min(content.width)
            * 64.)
            .floor()
            / 64.;
        let list = r(content.x, content.y, list_width - 1., content.height);
        let detail = r(
            content.x + list_width,
            content.y,
            content.width - list_width,
            content.height,
        );
        let mut b = Paint {
            scene,
            controls: vec![],
            p,
            clip: content,
        };
        b.fill(content, 0., p.surface);
        b.fill(list, 0., [p.sunken[0], p.sunken[1], p.sunken[2], 153]);
        b.fill(detail, 0., p.canvas);
        b.clip(list);
        self.draw_list(&mut b, list, metrics);
        b.unclip();
        b.clip(detail);
        let commands = b.scene.commands.len();
        let controls = b.controls.len();
        self.draw_detail(&mut b, detail, narrow, metrics);
        if self.detail_limit > 0. {
            b.scene.commands.truncate(commands);
            b.controls.truncate(controls);
            self.draw_detail(
                &mut b,
                r(detail.x, detail.y, detail.width - 8., detail.height),
                narrow,
                metrics,
            );
            self.detail_view = detail;
        }
        b.unclip();
        if let Some(target) = self.focus.filter(|_| self.keyboard_focus) {
            if let Some(control) = b.controls.iter().find(|c| c.target == target).cloned() {
                if control.hit_rect.height > 0. {
                    b.scene.commands.push(DrawCommand::StrokeRect {
                        rect: r(
                            control.rect.x + 1.,
                            control.rect.y + 1.,
                            control.rect.width - 2.,
                            control.rect.height - 2.,
                        ),
                        radius: 6.,
                        width: 1.,
                        color: p.accent,
                    });
                }
            }
        }
        let frame = LibraryFrame {
            scene: b.scene.scaled(self.scale),
            controls: b
                .controls
                .into_iter()
                .map(|mut c| {
                    c.rect = c.rect.scaled(self.scale);
                    c.hit_rect = c.hit_rect.scaled(self.scale);
                    c
                })
                .collect(),
        };
        self.frame = Some(frame.clone());
        frame
    }
    fn draw_list(&mut self, b: &mut Paint, area: Rect, metrics: &mut impl TextMeasurer) {
        let p = b.p;
        let title = if self.snapshot.kind == LibraryKind::History {
            "历史记录"
        } else {
            "收藏夹"
        };
        let heading_h: f32 = if self.snapshot.shell.theme == Theme::Paper {
            160.
        } else {
            156.
        };
        let heading_offset = if self.snapshot.shell.theme == Theme::Paper {
            4.
        } else {
            0.
        };
        let header = r(area.x, area.y, area.width, heading_h.min(area.height));
        let footer = r(
            area.x,
            (area.y + area.height - 44.).max(area.y),
            area.width,
            44.,
        );
        self.list_view = r(
            area.x,
            area.y + header.height,
            area.width,
            (footer.y - area.y - header.height).max(0.),
        );
        b.fill(header, 0., p.surface);
        b.line(area.x, header.y + header.height - 1., area.width, p.line);
        let font = if self.snapshot.shell.theme == Theme::Paper {
            FontFamily::Serif
        } else {
            FontFamily::Sans
        };
        b.scene.commands.push(DrawCommand::Text {
            rect: r(area.x + 16., area.y + 20., area.width - 32., 36.),
            text: title.into(),
            size: 26.,
            weight: if font == FontFamily::Serif { 400 } else { 700 },
            color: p.ink,
            font,
            align: TextAlign::Left,
        });
        let total = format!("{} 项", self.snapshot.total);
        let width = measure_width(metrics, &total, 10.5, 400);
        b.text(
            r(area.x + area.width - 16. - width, area.y + 39., width, 16.),
            total,
            10.5,
            400,
            p.faint,
        );
        let search = r(
            area.x + 16.,
            area.y + 67. + heading_offset,
            area.width - 32.,
            36.,
        );
        self.draw_input(
            b,
            LibraryField::Search,
            search,
            if self.snapshot.kind == LibraryKind::History {
                "搜索历史记录"
            } else {
                "搜索收藏的内容、备注或标签"
            },
            metrics,
        );
        let mut x = area.x + 16.;
        for &filter in LibraryFilter::for_kind(self.snapshot.kind) {
            let width = button_width(metrics, filter.label());
            b.button(
                LibraryTarget::Filter(filter),
                r(x, area.y + 115. + heading_offset, width, 28.),
                filter.label(),
                true,
                filter == self.query.filter,
            );
            x += width + 6.;
        }
        b.unclip();
        b.clip(self.list_view);
        let row_h = match self.snapshot.shell.theme {
            Theme::Paper => 88.,
            Theme::Studio => 72.,
            _ => 74.,
        };
        self.list_limit =
            (self.snapshot.items.len() as f32 * row_h - self.list_view.height).max(0.);
        self.list_scroll = self.list_scroll.clamp(0., self.list_limit);
        if self.snapshot.items.is_empty() {
            let message = if self.snapshot.loading {
                if self.snapshot.kind == LibraryKind::History {
                    "正在加载历史记录…"
                } else {
                    "正在加载收藏夹…"
                }
            } else if self.query.search.is_empty() {
                if self.snapshot.kind == LibraryKind::History {
                    "暂无历史记录"
                } else {
                    "暂无收藏内容"
                }
            } else if self.snapshot.kind == LibraryKind::History {
                "没有匹配的历史记录"
            } else {
                "没有匹配的收藏"
            };
            b.text(
                r(
                    area.x + 32.,
                    self.list_view.y + (self.list_view.height.min(360.) - 18.) / 2.,
                    area.width - 64.,
                    36.,
                ),
                message,
                10.5,
                400,
                p.faint,
            );
        }
        for (index, item) in self.snapshot.items.iter().enumerate() {
            let row = r(
                area.x,
                self.list_view.y + index as f32 * row_h - self.list_scroll,
                area.width - 8.,
                row_h,
            );
            let selected = Some(item.key) == self.selected;
            if selected {
                b.fill(row, 0., p.accent_soft);
                b.fill(r(row.x, row.y + 8., 3., row.height - 16.), 2., p.accent);
            }
            b.line(row.x, row.y + row.height - 1., row.width, p.line);
            let padding = if self.snapshot.shell.theme == Theme::Paper {
                15.
            } else {
                10.
            };
            let color = match item.key.kind {
                ContentKind::Translation => p.accent,
                ContentKind::Ocr => [139, 92, 246, 255],
                ContentKind::Screenshot => p.good,
            };
            b.fill(r(row.x + 16., row.y + padding + 4., 6., 6.), 3., color);
            b.text(
                r(row.x + 28., row.y + padding, row.width - 44., 15.),
                item.key.kind.label(),
                10.5,
                700,
                p.muted,
            );
            let tw = measure_width(metrics, &item.relative_time, 9.5, 400);
            b.text(
                r(row.x + row.width - 16. - tw, row.y + padding, tw, 15.),
                &item.relative_time,
                9.5,
                400,
                p.faint,
            );
            b.text(
                r(row.x + 16., row.y + padding + 17.3, row.width - 32., 15.3),
                item.title(self.snapshot.kind),
                10.5,
                600,
                p.ink,
            );
            b.text(
                r(row.x + 16., row.y + padding + 36.5, row.width - 32., 15.3),
                item.preview(self.snapshot.kind),
                10.5,
                400,
                p.muted,
            );
            b.control(
                LibraryTarget::Item(item.key),
                row,
                LibraryRole::ListItem,
                &item.title(self.snapshot.kind),
                true,
                selected,
            );
        }
        b.unclip();
        b.clip(area);
        b.fill(footer, 0., p.surface);
        b.line(footer.x, footer.y, footer.width, p.line);
        let start = if self.snapshot.total == 0 {
            0
        } else {
            self.query.page * 20 + 1
        };
        let end = ((self.query.page + 1) * 20).min(self.snapshot.total);
        b.text(
            r(footer.x + 16., footer.y + 15., footer.width - 95., 16.),
            format!("{start}–{end} / {}", self.snapshot.total),
            10.5,
            400,
            p.muted,
        );
        let x = footer.x + footer.width - 81.;
        b.button(
            LibraryTarget::PreviousPage,
            r(x, footer.y + 8.5, 32., 28.),
            "‹",
            self.query.page > 0,
            false,
        );
        b.button(
            LibraryTarget::NextPage,
            r(x + 32., footer.y + 8.5, 32., 28.),
            "›",
            end < self.snapshot.total,
            false,
        );
        if self.snapshot.kind == LibraryKind::History && self.snapshot.total > 0 {
            let clear = r(
                (x - 55.9375).max(footer.x + 16.),
                footer.y + 15.,
                42.9375,
                15.,
            );
            b.text(clear, "清空记录", 10.5, 400, p.faint);
            b.control(
                LibraryTarget::ClearHistory,
                clear,
                LibraryRole::Button,
                "清空记录",
                true,
                false,
            );
        }
        if let Some(error) = &self.snapshot.error {
            b.text(
                r(area.x + 16., self.list_view.y, area.width - 32., 40.),
                error,
                10.5,
                400,
                p.danger,
            );
        }
        b.unclip();
        b.clip(b.scene.content);
    }
    fn draw_detail(
        &mut self,
        b: &mut Paint,
        area: Rect,
        narrow: bool,
        metrics: &mut impl TextMeasurer,
    ) {
        self.detail_view = area;
        let p = b.p;
        let padding = if narrow {
            18.
        } else if self.snapshot.shell.theme == Theme::Paper {
            36.
        } else {
            28.
        };
        let width = (area.width - padding * 2.).min(820.).max(1.);
        let x = area.x + (area.width - width) / 2.;
        let Some(item) = self
            .snapshot
            .items
            .iter()
            .find(|i| Some(i.key) == self.selected)
            .cloned()
        else {
            b.text(
                r(x, area.y + 170., width, 36.),
                if self.snapshot.kind == LibraryKind::History {
                    "选择一条记录查看详情"
                } else {
                    "选择一条收藏查看详情"
                },
                10.5,
                400,
                p.faint,
            );
            self.detail_limit = 0.;
            return;
        };
        let vertical_padding = if narrow {
            18.
        } else if self.snapshot.shell.theme == Theme::Paper {
            31.
        } else {
            28.
        };
        let top = area.y + vertical_padding - self.detail_scroll;
        let mut y = top;
        let subtitle = match &item.content {
            LibraryContent::Translation {
                source_language,
                target_language,
                ..
            } => format!("翻译 · {source_language} → {target_language}"),
            LibraryContent::Ocr {
                language, provider, ..
            } => format!(
                "OCR · {} · {provider}",
                language.as_deref().unwrap_or("自动识别")
            ),
            LibraryContent::Screenshot { width, height } => format!("截图 · {width} × {height}"),
        };
        let favorite = self.snapshot.kind == LibraryKind::Favorites;
        let date_height = if self.snapshot.shell.theme == Theme::Paper {
            42.05
        } else {
            21.75
        };
        b.scene.commands.push(DrawCommand::Text {
            rect: r(x, y, width - 76., date_height),
            text: item.date.clone(),
            size: if self.snapshot.shell.theme == Theme::Paper {
                29.
            } else {
                15.
            },
            weight: if self.snapshot.shell.theme == Theme::Paper {
                400
            } else {
                700
            },
            color: p.ink,
            font: if self.snapshot.shell.theme == Theme::Paper {
                FontFamily::Serif
            } else {
                FontFamily::Sans
            },
            align: TextAlign::Left,
        });
        b.text(
            r(x, y + date_height + 4., width - 76., 16.),
            subtitle,
            10.5,
            400,
            p.muted,
        );
        let mut action_x = x + width - 32.;
        self.detail_action(
            b,
            LibraryTarget::Delete(item.key),
            r(action_x, y, 32., 32.),
            "⌫",
            "删除",
            true,
            p.faint,
        );
        action_x -= 36.;
        if favorite {
            self.detail_action(
                b,
                LibraryTarget::Unfavorite(item.key),
                r(action_x, y, 32., 32.),
                "★",
                "取消收藏",
                true,
                favorite_color(self.snapshot.shell.theme, self.snapshot.shell.dark),
            );
        } else if let LibraryContent::Ocr { favorite, .. } = &item.content {
            self.detail_action(
                b,
                LibraryTarget::Favorite(item.key, 0),
                r(action_x, y, 32., 32.),
                if *favorite { "★" } else { "☆" },
                if *favorite { "已收藏" } else { "收藏" },
                !*favorite,
                p.accent,
            );
        }
        y += if self.snapshot.shell.theme == Theme::Paper {
            81.
        } else {
            60.
        };
        match &item.content {
            LibraryContent::Translation {
                source,
                source_language,
                target_language,
                results,
            } => {
                y = self.text_card(
                    b,
                    x,
                    y,
                    width,
                    "原文",
                    if self.snapshot.kind == LibraryKind::History {
                        format!("{} chars", source.encode_utf16().count())
                    } else {
                        String::new()
                    },
                    source,
                    &[(
                        LibraryTarget::Copy(item.key, CopyPart::Source),
                        "复制",
                        true,
                    )],
                    metrics,
                );
                if self.snapshot.kind == LibraryKind::History {
                    let source_width = measure_width(metrics, source_language, 10.5, 400) + 24.;
                    let target_width = measure_width(metrics, target_language, 10.5, 400) + 24.;
                    b.fill(r(x, y, source_width, 31.), 10., p.surface);
                    b.text(
                        r(x + 12., y + 8., source_width - 24., 16.),
                        source_language,
                        10.5,
                        400,
                        p.muted,
                    );
                    b.text(
                        r(x + source_width + 8., y + 8., 14., 16.),
                        "→",
                        10.5,
                        400,
                        p.muted,
                    );
                    let target_x = x + source_width + 30.;
                    b.fill(r(target_x, y, target_width, 31.), 10., p.surface);
                    b.text(
                        r(target_x + 12., y + 8., target_width - 24., 16.),
                        target_language,
                        10.5,
                        400,
                        p.muted,
                    );
                    y += 43.;
                }
                for (index, result) in results.iter().enumerate() {
                    let mut actions = vec![(
                        LibraryTarget::Copy(item.key, CopyPart::Result(index)),
                        "复制",
                        true,
                    )];
                    if !favorite {
                        actions.push((
                            LibraryTarget::Favorite(item.key, index),
                            if result.favorite {
                                "已收藏"
                            } else {
                                "收藏"
                            },
                            true,
                        ));
                    }
                    y = self.text_card(
                        b,
                        x,
                        y,
                        width,
                        "译文",
                        result.provider.clone(),
                        &result.text,
                        &actions,
                        metrics,
                    );
                }
            }
            LibraryContent::Ocr {
                text,
                confidence,
                can_rerun,
                ..
            } => {
                if let Some(image) = &item.thumbnail {
                    y = self.image_card(b, x, y, width, "原图", "", image, 256., &[], metrics);
                }
                let mut actions =
                    vec![(LibraryTarget::Copy(item.key, CopyPart::Ocr), "复制", true)];
                if *can_rerun {
                    actions.push((LibraryTarget::RerunOcr(item.key), "重新识别并复制", true));
                }
                let meta = if !favorite {
                    confidence
                        .map(|c| format!("置信度 {}%", (c * 100.).round()))
                        .unwrap_or_else(|| format!("{} chars", text.encode_utf16().count()))
                } else {
                    String::new()
                };
                y = self.text_card(b, x, y, width, "识别文本", meta, text, &actions, metrics);
            }
            LibraryContent::Screenshot {
                width: iw,
                height: ih,
            } => {
                let actions = [
                    (LibraryTarget::CopyScreenshot(item.key), "复制图片", true),
                    (LibraryTarget::RevealScreenshot(item.key), "显示文件", true),
                ];
                if let Some(image) = &item.thumbnail {
                    y = self.image_card(
                        b,
                        x,
                        y,
                        width,
                        "截图预览",
                        &format!("{iw} × {ih}"),
                        image,
                        360.,
                        &actions,
                        metrics,
                    );
                } else {
                    y = self.text_card(
                        b,
                        x,
                        y,
                        width,
                        "截图预览",
                        format!("{iw} × {ih}"),
                        "预览加载中…",
                        &actions,
                        metrics,
                    );
                }
            }
        }
        if favorite {
            y = self.metadata_card(b, x, y, width, &item, metrics);
        }
        self.detail_limit = (y + self.detail_scroll - area.y + padding - area.height).max(0.);
        self.detail_scroll = self.detail_scroll.clamp(0., self.detail_limit);
    }
    fn detail_action(
        &self,
        b: &mut Paint,
        target: LibraryTarget,
        rect: Rect,
        glyph: &str,
        label: &str,
        enabled: bool,
        color: Color,
    ) {
        b.text(
            r(rect.x + 8., rect.y + 8., rect.width - 16., 16.),
            glyph,
            12.,
            400,
            if enabled { color } else { b.p.faint },
        );
        b.control(target, rect, LibraryRole::Button, label, enabled, false);
    }
    fn card(&self, b: &mut Paint, rect: Rect, label: &str, meta: &str) {
        let p = b.p;
        let paper = self.snapshot.shell.theme == Theme::Paper;
        if !paper {
            b.fill(rect, 10., p.surface);
            b.stroke(rect, 10., p.line);
        } else {
            b.line(rect.x, rect.y + rect.height - 1., rect.width, p.line);
        }
        let pad = if paper { 0. } else { 17. };
        let top = if paper { 16. } else { 17. };
        b.text(
            r(rect.x + pad, rect.y + top, rect.width - 2. * pad, 16.),
            label,
            10.5,
            700,
            p.faint,
        );
        let width = (rect.width - 2. * pad) * 0.6;
        b.scene.commands.push(DrawCommand::Text {
            rect: r(rect.x + rect.width - pad - width, rect.y + top, width, 16.),
            text: meta.into(),
            size: 10.5,
            weight: 500,
            color: p.faint,
            font: FontFamily::Sans,
            align: TextAlign::Right,
        });
    }
    fn card_actions(
        &self,
        b: &mut Paint,
        x: f32,
        y: f32,
        width: f32,
        actions: &[(LibraryTarget, &str, bool)],
        metrics: &mut impl TextMeasurer,
    ) {
        let p = b.p;
        b.line(x, y, width, p.line);
        let mut xx = x;
        for &(target, label, enabled) in actions {
            let w = button_width(metrics, label);
            b.button(target, r(xx, y + 13., w, 28.), label, enabled, false);
            xx += w + 6.;
        }
    }
    fn text_card(
        &self,
        b: &mut Paint,
        x: f32,
        y: f32,
        width: f32,
        label: &str,
        meta: String,
        text: &str,
        actions: &[(LibraryTarget, &str, bool)],
        metrics: &mut impl TextMeasurer,
    ) -> f32 {
        let paper = self.snapshot.shell.theme == Theme::Paper;
        let pad = if paper { 0. } else { 17. };
        let inside = (width - 2. * pad).max(1.);
        let body = metrics
            .line_layout(text, inside, 12.5, 400, FontFamily::Sans, 28.)
            .height
            .max(28.);
        let height = if paper { 0. } else { 1. }
            + 16.
            + 16.
            + 12.
            + body
            + if actions.is_empty() {
                16.
            } else {
                16. + 1. + 12. + 28. + 16.
            };
        self.card(b, r(x, y, width, height), label, &meta);
        let body_y = if paper { 43. } else { 44. };
        b.block(
            r(x + pad, y + body_y, inside, body),
            text,
            12.5,
            28.,
            b.p.ink,
        );
        if !actions.is_empty() {
            self.card_actions(
                b,
                x + pad,
                y + body_y + body + 16.,
                inside,
                actions,
                metrics,
            );
        }
        y + height + 12.
    }
    fn image_card(
        &self,
        b: &mut Paint,
        x: f32,
        y: f32,
        width: f32,
        label: &str,
        meta: &str,
        image: &crate::RasterImage,
        max_height: f32,
        actions: &[(LibraryTarget, &str, bool)],
        metrics: &mut impl TextMeasurer,
    ) -> f32 {
        let paper = self.snapshot.shell.theme == Theme::Paper;
        let pad = if paper { 0. } else { 17. };
        let inside = (width - pad * 2.).max(1.);
        let h = (inside * image.height() as f32 / image.width() as f32).min(max_height);
        let draw_width = (h * image.width() as f32 / image.height() as f32).min(inside);
        let body_y = if paper { 43. } else { 44. };
        let height = if paper { 0. } else { 1. }
            + 44.
            + h
            + if actions.is_empty() {
                16.
            } else {
                16. + 1. + 12. + 28. + 16.
            };
        self.card(b, r(x, y, width, height), label, meta);
        b.fill(r(x + pad, y + body_y, inside, h), 10., b.p.sunken);
        b.scene.commands.push(DrawCommand::Image {
            rect: r(
                x + pad + (inside - draw_width) / 2.,
                y + body_y,
                draw_width,
                h,
            ),
            image: image.clone(),
        });
        if !actions.is_empty() {
            self.card_actions(b, x + pad, y + body_y + h + 16., inside, actions, metrics);
        }
        y + height + 12.
    }
    fn metadata_card(
        &mut self,
        b: &mut Paint,
        x: f32,
        y: f32,
        width: f32,
        item: &LibraryItem,
        metrics: &mut impl TextMeasurer,
    ) -> f32 {
        let paper = self.snapshot.shell.theme == Theme::Paper;
        let pad = if paper { 0. } else { 17. };
        let inner = (width - 2. * pad).max(1.);
        let height = if paper { 220. } else { 221. };
        let offset = if paper { 0. } else { 1. };
        self.card(b, r(x, y, width, height), "收藏信息", "自动保存");
        let xx = x + pad;
        b.fill(
            r(xx, y + 44. + offset, inner, height - 60.),
            10.,
            b.p.sunken,
        );
        b.text(
            r(xx + 12., y + 56. + offset, inner - 24., 16.),
            "笔记",
            10.5,
            400,
            b.p.muted,
        );
        self.draw_input(
            b,
            LibraryField::Note(item.key),
            r(xx + 12., y + 74. + offset, inner - 24., 54.),
            "添加上下文笔记",
            metrics,
        );
        b.text(
            r(xx + 12., y + 136. + offset, inner - 24., 16.),
            "标签（使用逗号分隔）",
            10.5,
            400,
            b.p.muted,
        );
        self.draw_input(
            b,
            LibraryField::Tags(item.key),
            r(xx + 12., y + 155. + offset, inner - 24., 36.),
            "工作, 技术",
            metrics,
        );
        y + height + 12.
    }
    fn draw_input(
        &mut self,
        b: &mut Paint,
        field: LibraryField,
        rect: Rect,
        placeholder: &str,
        metrics: &mut impl TextMeasurer,
    ) {
        let p = b.p;
        let target = match field {
            LibraryField::Search => LibraryTarget::Search,
            LibraryField::Note(key) => LibraryTarget::Note(key),
            LibraryField::Tags(key) => LibraryTarget::Tags(key),
        };
        let multiline = matches!(field, LibraryField::Note(_));
        let size = if matches!(field, LibraryField::Search) {
            10.5
        } else {
            12.5
        };
        let line_height = size * 1.5;
        let value = if let Some(edit) = self.editing.as_ref().filter(|e| e.field == field) {
            edit.buffer.display_text()
        } else {
            match field {
                LibraryField::Search => self.query.search.clone(),
                LibraryField::Note(key) => self
                    .snapshot
                    .items
                    .iter()
                    .find(|i| i.key == key)
                    .and_then(|i| i.note.clone())
                    .unwrap_or_default(),
                LibraryField::Tags(key) => self
                    .snapshot
                    .items
                    .iter()
                    .find(|i| i.key == key)
                    .map(|i| i.tags.join(", "))
                    .unwrap_or_default(),
            }
        };
        let focused = self.editing.as_ref().is_some_and(|e| e.field == field);
        b.fill(
            rect,
            7.,
            if focused || field != LibraryField::Search {
                p.surface
            } else {
                p.sunken
            },
        );
        b.stroke(rect, 7., if focused { p.accent } else { p.line });
        let inset = if field == LibraryField::Search {
            32.
        } else {
            12.
        };
        let text_rect = r(
            rect.x + inset,
            rect.y
                + if multiline {
                    8.
                } else {
                    (rect.height - line_height) / 2.
                },
            rect.width - inset - 12.,
            if multiline {
                rect.height - 16.
            } else {
                line_height
            },
        );
        if field == LibraryField::Search {
            b.text(
                r(rect.x + 12., rect.y + 9., 16., 18.),
                "⌕",
                12.5,
                400,
                p.faint,
            );
        }
        let layout = metrics.line_layout(
            &value,
            if multiline {
                text_rect.width.max(1.)
            } else {
                100_000.
            },
            size,
            400,
            FontFamily::Sans,
            line_height,
        );
        let mut scroll = Point::default();
        if let Some(edit) = self.editing.as_mut().filter(|e| e.field == field) {
            if let Some(caret) = edit.buffer.display_caret().and_then(|byte| {
                layout
                    .caret_stops
                    .iter()
                    .find(|(b, _)| *b == byte)
                    .map(|(_, p)| *p)
            }) {
                if multiline {
                    if caret.y < edit.scroll.y {
                        edit.scroll.y = caret.y;
                    } else if caret.y + line_height > edit.scroll.y + text_rect.height {
                        edit.scroll.y = caret.y + line_height - text_rect.height;
                    }
                } else {
                    if caret.x < edit.scroll.x {
                        edit.scroll.x = caret.x;
                    } else if caret.x > edit.scroll.x + text_rect.width {
                        edit.scroll.x = caret.x - text_rect.width;
                    }
                }
            }
            edit.rect = text_rect;
            edit.layout = layout.clone();
            scroll = edit.scroll;
        }
        let clip = b.clip;
        let visible = text_rect.intersect(clip);
        b.scene.commands.push(DrawCommand::PushClip(visible));
        if let Some(edit) = self.editing.as_ref().filter(|e| e.field == field) {
            if let Some(selection) = edit.buffer.display_selection().filter(|s| !s.is_empty()) {
                for pair in layout.caret_stops.windows(2) {
                    let (byte, a) = pair[0];
                    let (next, z) = pair[1];
                    if byte >= selection.start && next <= selection.end {
                        let w = if z.y > a.y {
                            (text_rect.width - a.x).max(3.)
                        } else {
                            (z.x - a.x).abs().max(1.)
                        };
                        b.fill(
                            r(
                                text_rect.x + if z.y > a.y { a.x } else { a.x.min(z.x) } - scroll.x,
                                text_rect.y + a.y - scroll.y,
                                w,
                                line_height,
                            ),
                            0.,
                            p.accent_soft,
                        );
                    }
                }
            }
        }
        b.block(
            r(
                text_rect.x - scroll.x,
                text_rect.y - scroll.y,
                if multiline { text_rect.width } else { 100_000. },
                layout.height.max(line_height),
            ),
            if value.is_empty() {
                placeholder
            } else {
                &value
            },
            size,
            line_height,
            if value.is_empty() { p.faint } else { p.ink },
        );
        if let Some(edit) = self.editing.as_ref().filter(|e| e.field == field) {
            if let Some(composition) = edit.buffer.composition_range() {
                for pair in layout.caret_stops.windows(2) {
                    let (byte, a) = pair[0];
                    let (next, z) = pair[1];
                    if byte >= composition.start && next <= composition.end && a.y == z.y {
                        b.line(
                            text_rect.x + a.x.min(z.x) - scroll.x,
                            text_rect.y + a.y + line_height - 1. - scroll.y,
                            (z.x - a.x).abs(),
                            p.accent,
                        );
                    }
                }
            }
            if let Some(byte) = edit.buffer.display_caret() {
                if let Some((_, caret)) = layout.caret_stops.iter().find(|(b, _)| *b == byte) {
                    let caret = r(
                        text_rect.x + caret.x - scroll.x,
                        text_rect.y + caret.y - scroll.y,
                        1.,
                        line_height,
                    );
                    b.fill(caret, 0., p.ink);
                    if caret.intersect(visible).height > 0. {
                        b.scene.ime_cursor = Some(caret);
                    }
                }
            }
        }
        b.scene.commands.push(DrawCommand::PopClip);
        b.control(
            target,
            rect,
            if field == LibraryField::Search {
                LibraryRole::SearchInput
            } else if multiline {
                LibraryRole::MultilineInput
            } else {
                LibraryRole::TextInput
            },
            match field {
                LibraryField::Search => "搜索",
                LibraryField::Note(_) => "笔记",
                LibraryField::Tags(_) => "标签",
            },
            true,
            false,
        );
    }
}
fn measure_width(metrics: &mut impl TextMeasurer, text: &str, size: f32, weight: u16) -> f32 {
    metrics
        .measure(text, size, weight, FontFamily::Sans)
        .caret_stops
        .iter()
        .map(|(_, x)| *x)
        .fold(0., f32::max)
}

fn favorite_color(theme: Theme, dark: bool) -> Color {
    match (theme, dark) {
        (Theme::Native, false) => [160, 117, 50, 255],
        (Theme::Native, true) => [217, 178, 129, 255],
        (Theme::Studio, false) => [155, 120, 63, 255],
        (Theme::Studio, true) => [222, 184, 136, 255],
        (Theme::Paper, false) => [163, 133, 79, 255],
        (Theme::Paper, true) => [198, 177, 130, 255],
    }
}

fn button_width(metrics: &mut impl TextMeasurer, label: &str) -> f32 {
    // Fixed retained controls use the measured system-font widths from the
    // actual React fixture, while free text uses the supplied font measurer.
    match label {
        "全部" | "翻译" | "截图" | "复制" | "收藏" => 43.46875,
        "OCR" => 45.046875,
        "已收藏" => 54.203125,
        "复制图片" | "显示文件" => 64.9375,
        "重新识别并复制" => 97.125,
        _ => measure_width(metrics, label, 10.5, 500) + 22.,
    }
}

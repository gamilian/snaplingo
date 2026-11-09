use super::{
    icons::{self, Icon},
    *,
};
use crate::{
    theme::{palette, Palette},
    Color, DrawCommand, FontFamily, Size, TextAlign, TextMeasurer, Theme,
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
    controls: Vec<ResultControl>,
    clip: Rect,
    p: Palette,
    radius: f32,
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
    fn box_frame(&mut self, rect: Rect, strong: bool) {
        self.fill(rect, 13., self.p.surface);
        self.stroke(
            rect,
            13.,
            if strong {
                self.p.line_strong
            } else {
                self.p.line
            },
        );
    }
}
fn language(code: &str) -> &str {
    match code {
        "zh-CN" => "中文简体",
        "zh-TW" => "中文繁体",
        _ => LANGUAGES
            .iter()
            .find(|(id, _)| *id == code)
            .map_or(code, |(_, name)| *name),
    }
}
fn text_style(text: &str, ocr: bool, result: bool) -> (f32, f32) {
    let lines: usize = text
        .split('\n')
        .map(|line| {
            line.chars()
                .count()
                .div_ceil(if result { 72 } else { 54 })
                .max(1)
        })
        .sum();
    let density = (text.chars().count() as f32 / if result { 180. } else { 140. })
        .max(lines as f32 / if result { 3. } else { 2. });
    let size = if density >= 2.2 {
        12.
    } else if density >= 1.15 {
        13.
    } else {
        14.
    };
    let line = if ocr {
        if density >= 2.2 {
            1.28
        } else if density >= 1.15 {
            1.36
        } else {
            1.44
        }
    } else {
        1.38
    };
    (size, size * line)
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
        let p = palette(self.snapshot.shell.theme, self.snapshot.shell.dark);
        let full = r(0., 0., logical.width, logical.height);
        let panel_width = (logical.width - 16.).max(1.);
        let width = (logical.width - 40.).max(1.);
        let body = r(20., 56., width, (logical.height - 64.).max(0.));
        let mut b = Paint {
            scene: Scene {
                size: logical,
                ime_cursor: None,
                content: full,
                commands: vec![],
                hits: vec![],
                geometry: vec![],
                content_height: 0.,
                viewport: body,
            },
            controls: vec![],
            clip: full,
            p,
            radius: if self.snapshot.shell.theme == Theme::Native {
                13.
            } else {
                8.
            },
        };
        b.icon(
            ResultTarget::Pin,
            r(20., 17.5, 28., 28.),
            if self.pinned {
                "取消固定"
            } else {
                "固定窗口"
            },
            Icon::Pin,
            true,
            false,
        );
        if self.pinned {
            b.fill(r(20., 17.5, 28., 28.), 7., p.accent_soft);
            icons::draw(&mut b.scene, r(26., 23.5, 16., 16.), Icon::Pin, p.accent);
        }
        b.fill(r(60., 17.5, 28., 28.), 7., p.accent_soft);
        icons::draw(
            &mut b.scene,
            r(66., 23.5, 16., 16.),
            if self.snapshot.mode == ResultMode::Ocr {
                Icon::Scan
            } else {
                Icon::Language
            },
            p.accent,
        );
        b.text(
            r(100., 22., 48., 24.),
            if self.snapshot.mode == ResultMode::Ocr {
                "OCR"
            } else {
                "翻译"
            },
            16.,
            700,
            p.ink,
        );
        let subtitle = if self.snapshot.mode == ResultMode::Ocr {
            if self.snapshot.ocr_running {
                "识别中".into()
            } else if self.snapshot.ocr_text.is_empty() {
                "等待上传".into()
            } else {
                format!(
                    "已识别 {} 个字符{}",
                    self.snapshot.ocr_text.chars().count(),
                    self.snapshot
                        .ocr_confidence
                        .map_or(String::new(), |c| format!(" · 置信度 {:.0}%", c * 100.))
                )
            }
        } else if self.snapshot.rows.is_empty() {
            "准备翻译".into()
        } else {
            format!(
                "{}/{} 个服务已返回",
                self.snapshot
                    .rows
                    .iter()
                    .filter(|r| r.status == RowStatus::Success)
                    .count(),
                self.snapshot.rows.len()
            )
        };
        b.text(
            r(139., 24., logical.width - 210., 20.),
            subtitle,
            10.5,
            600,
            p.muted,
        );
        b.control(
            ResultTarget::Header,
            r(60., 8., logical.width - 112., 47.),
            "拖动窗口",
            ResultRole::Button,
            None,
            true,
        );
        b.icon(
            ResultTarget::Close,
            r(logical.width - 48., 17.5, 28., 28.),
            "关闭",
            Icon::Close,
            true,
            false,
        );
        b.fill(r(8., 55., panel_width, 1.), 0., p.line);
        b.scene.commands.push(DrawCommand::PushClip(body));
        b.clip = body;
        let mut y = 68. - self.scroll;
        let mut cards = vec![];
        if let Some(error) = &self.snapshot.error {
            b.text(r(20., y, width, 30.), error, 12., 400, p.danger);
            y += 38.;
        }

        if self.snapshot.mode == ResultMode::Translation {
            if self.source_collapsed {
                let rect = r(20., y, width, 40.);
                b.fill(rect, 8., p.sunken);
                b.stroke(rect, 8., p.line);
                b.text(
                    r(32., y + 12., width - 100., 18.),
                    &self.snapshot.source,
                    10.5,
                    400,
                    p.muted,
                );
                b.text(
                    r(logical.width - 90., y + 12., 68., 18.),
                    "展开原文",
                    10.5,
                    600,
                    p.accent,
                );
                b.control(
                    ResultTarget::ToggleSource,
                    rect,
                    "展开原文",
                    ResultRole::Button,
                    None,
                    true,
                );
                y += 52.;
            } else {
                let (size, line) = text_style(&self.snapshot.source, false, false);
                let text_height = (metrics
                    .line_layout(
                        &self.snapshot.source,
                        width - 26.,
                        size,
                        400,
                        FontFamily::Sans,
                        line,
                    )
                    .height
                    + 20.)
                    .floor()
                    .max(52.);
                let outer = r(20., y, width, text_height + 50.);
                b.box_frame(outer, true);
                self.text_field(
                    &mut b,
                    ResultTarget::Source,
                    r(21., y + 1., width - 2., text_height),
                    r(33., y + 11., width - 26., text_height - 20.),
                    "原文",
                    self.snapshot.source.clone(),
                    size,
                    line,
                    metrics,
                );
                let footer = y + text_height + 8.;
                b.fill(r(21., footer, width - 2., 41.), 0., p.sunken);
                b.fill(r(21., footer, width - 2., 1.), 0., p.line);
                b.text(
                    r(33., footer + 14., width - 250., 16.),
                    format!(
                        "{} chars · 识别为 {}",
                        self.snapshot.source.chars().count(),
                        language(
                            self.snapshot
                                .detected_language
                                .as_deref()
                                .unwrap_or(&self.snapshot.source_language)
                        )
                    ),
                    10.5,
                    400,
                    p.faint,
                );
                let right = logical.width - 33.;
                let start = right - 217.;
                b.text(
                    r(start + 8., footer + 14., 29., 18.),
                    "折叠",
                    10.5,
                    600,
                    p.muted,
                );
                b.control(
                    ResultTarget::ToggleSource,
                    r(start, footer + 7., 37., 28.),
                    "折叠",
                    ResultRole::Button,
                    None,
                    true,
                );
                let has = !self.snapshot.source.trim().is_empty();
                for (index, (target, label, icon, enabled)) in [
                    (
                        ResultTarget::FavoriteAll,
                        "收藏",
                        Icon::Favorite,
                        !self.snapshot.favorite_pending
                            && self.snapshot.rows.iter().any(|r| {
                                r.status == RowStatus::Success && !r.favorite && !r.text.is_empty()
                            }),
                    ),
                    (ResultTarget::CopySource, "复制", Icon::Copy, has),
                    (ResultTarget::SpeakSource, "朗读", Icon::Volume, has),
                    (
                        ResultTarget::Translate,
                        "重试",
                        Icon::Retry,
                        has && !self.snapshot.translating,
                    ),
                    (ResultTarget::ClearSource, "清空", Icon::Clear, has),
                ]
                .into_iter()
                .enumerate()
                {
                    b.icon(
                        target,
                        r(start + 45. + index as f32 * 36., footer + 7., 28., 28.),
                        label,
                        icon,
                        enabled,
                        true,
                    );
                }
                y += outer.height + 12.;
            }
            let lang = r(20., y, width, 40.);
            b.fill(lang, b.radius, p.sunken);
            b.stroke(lang, b.radius, p.line);
            let half = (width - 42.) / 2.;
            b.centered(
                r(21., y + 10., half, 20.),
                language(&self.snapshot.source_language),
                12.5,
                p.ink,
            );
            icons::draw(
                &mut b.scene,
                r(21. + half - 28., y + 12., 16., 16.),
                Icon::Down,
                p.faint,
            );
            b.control(
                ResultTarget::SourceLanguage,
                r(21., y + 2., half, 36.),
                "源语言",
                ResultRole::Button,
                Some(self.snapshot.source_language.clone()),
                true,
            );
            b.icon(
                ResultTarget::Swap,
                r(21. + half, y + 1., 40., 38.),
                "切换语言",
                Icon::Swap,
                true,
                false,
            );
            b.centered(
                r(61. + half, y + 10., half, 20.),
                language(&self.snapshot.target_language),
                12.5,
                p.ink,
            );
            icons::draw(
                &mut b.scene,
                r(logical.width - 60., y + 12., 16., 16.),
                Icon::Down,
                p.faint,
            );
            b.control(
                ResultTarget::TargetLanguage,
                r(61. + half, y + 2., half, 36.),
                "目标语言",
                ResultRole::Button,
                Some(self.snapshot.target_language.clone()),
                true,
            );
            y += 52.;
            for row in self.snapshot.rows.clone() {
                let collapsed = self.collapsed_rows.contains(&row.id);
                let text = if row.status == RowStatus::Error {
                    row.error.clone().unwrap_or_else(|| "翻译失败".into())
                } else {
                    row.text.clone()
                };
                let (size, line) = text_style(&text, false, true);
                let body_height = if collapsed {
                    0.
                } else if row.status == RowStatus::Pending {
                    62.
                } else {
                    (metrics
                        .line_layout(&text, width - 26., size, 400, FontFamily::Sans, line)
                        .height
                        + 16.)
                        .floor()
                };
                let height = 43. + body_height;
                let card = r(20., y, width, height);
                cards.push(card);
                b.box_frame(card, false);
                if self.snapshot.shell.theme == Theme::Paper {
                    if let Some(DrawCommand::FillRect { radius, .. }) =
                        b.scene.commands.iter_mut().rev().find(
                            |c| matches!(c, DrawCommand::FillRect { rect, .. } if *rect == card),
                        )
                    {
                        *radius = 0.;
                    }
                    if let Some(DrawCommand::StrokeRect { radius, .. }) =
                        b.scene.commands.last_mut()
                    {
                        *radius = 0.;
                    }
                }
                b.fill(r(21., y + 1., width - 2., 41.), 0., p.sunken);
                if !collapsed {
                    b.fill(r(21., y + 41., width - 2., 1.), 0., p.line);
                }
                let provider = match row.id.as_str() {
                    "google" => [66, 133, 244, 255],
                    "deeplx" => [15, 43, 70, 255],
                    "baidu" => [41, 50, 225, 255],
                    "openai" => [16, 163, 127, 255],
                    _ => p.muted,
                };
                b.fill(r(33., y + 11., 4., 20.), 2., provider);
                b.text(
                    r(49., y + 12., width - 220., 20.),
                    &row.name,
                    13.,
                    600,
                    p.ink,
                );
                if row.status != RowStatus::Success {
                    let label = if row.status == RowStatus::Pending {
                        "翻译中"
                    } else {
                        "失败"
                    };
                    let name_width = metrics
                        .measure(&row.name, 13., 500, FontFamily::Sans)
                        .caret_stops
                        .iter()
                        .map(|(_, x)| *x)
                        .fold(0., f32::max);
                    let rect = r(
                        61. + name_width,
                        y + 10.,
                        if row.status == RowStatus::Pending {
                            48.
                        } else {
                            37.
                        },
                        20.,
                    );
                    b.fill(
                        rect,
                        7.,
                        if row.status == RowStatus::Pending {
                            p.accent_soft
                        } else {
                            [p.danger[0], p.danger[1], p.danger[2], 25]
                        },
                    );
                    b.text(
                        r(rect.x + 8., rect.y + 3., rect.width - 16., 16.),
                        label,
                        10.5,
                        600,
                        if row.status == RowStatus::Pending {
                            p.accent
                        } else {
                            p.danger
                        },
                    );
                }
                let mut actions = vec![];
                if row.status != RowStatus::Pending && row.retryable {
                    actions.push((
                        ResultTarget::Retry(row.id.clone()),
                        "重试",
                        Icon::Retry,
                        true,
                    ));
                }
                if row.status != RowStatus::Error {
                    actions.push((
                        ResultTarget::SpeakResult(row.id.clone()),
                        "朗读",
                        Icon::Volume,
                        !row.text.is_empty(),
                    ));
                }
                if row.status == RowStatus::Success {
                    actions.push((
                        ResultTarget::FavoriteResult(row.id.clone()),
                        if row.favorite { "已收藏" } else { "收藏" },
                        Icon::Favorite,
                        !row.favorite && !self.snapshot.favorite_pending,
                    ));
                }
                if row.status != RowStatus::Error {
                    actions.push((
                        ResultTarget::CopyResult(row.id.clone()),
                        "复制",
                        Icon::Copy,
                        !row.text.is_empty(),
                    ));
                }
                actions.push((
                    ResultTarget::ToggleRow(row.id.clone()),
                    if collapsed { "展开" } else { "收起" },
                    if collapsed { Icon::Down } else { Icon::Up },
                    true,
                ));
                let start = logical.width - 33. - (actions.len() as f32 * 32. - 8.);
                for (index, (target, label, icon, enabled)) in actions.into_iter().enumerate() {
                    b.icon(
                        target,
                        r(start + index as f32 * 32., y + 9., 24., 24.),
                        label,
                        icon,
                        enabled,
                        true,
                    );
                }
                if !collapsed {
                    if row.status == RowStatus::Pending {
                        for (index, fraction) in [11. / 12., 0.75, 0.5].into_iter().enumerate() {
                            b.fill(
                                r(
                                    33.,
                                    y + 51. + index as f32 * 18.,
                                    (width - 26.) * fraction,
                                    10.,
                                ),
                                5.,
                                p.raised,
                            );
                        }
                    } else {
                        self.text_field(
                            &mut b,
                            ResultTarget::ResultText(row.id.clone()),
                            r(21., y + 42., width - 2., body_height),
                            r(33., y + 50., width - 26., body_height - 16.),
                            &row.name,
                            text,
                            size,
                            line,
                            metrics,
                        );
                    }
                }
                y += height + 12.;
            }
        } else {
            self.ocr(&mut b, &mut y, width, logical.width, metrics);
        }
        b.scene.commands.push(DrawCommand::PopClip);
        let natural = y
            + self.scroll
            + if self.snapshot.mode == ResultMode::Ocr {
                8.
            } else {
                0.
            };
        let panel = r(
            8.,
            8.,
            panel_width,
            (natural - 8.).min(logical.height - 16.),
        );
        b.scene.commands.insert(
            0,
            DrawCommand::FillRect {
                rect: panel,
                radius: b.radius,
                color: p.surface,
            },
        );
        self.max_scroll = (natural - logical.height + 8.).max(0.);
        self.scroll = self.scroll.min(self.max_scroll);
        b.scene.content_height = natural;
        b.scene.content = panel;
        b.clip = full;
        if let Some(source) = self.menu {
            let target = if source {
                ResultTarget::SourceLanguage
            } else {
                ResultTarget::TargetLanguage
            };
            if let Some(anchor) = b
                .controls
                .iter()
                .find(|c| c.target == target)
                .map(|c| c.rect)
            {
                let height = LANGUAGES.len() as f32 * 28. + 8.;
                let top =
                    (anchor.y + anchor.height + 4.).min((logical.height - height - 8.).max(54.));
                b.fill(r(anchor.x, top, anchor.width, height), 7., p.raised);
                for (index, (code, label)) in LANGUAGES.iter().enumerate() {
                    let rect = r(
                        anchor.x + 4.,
                        top + 4. + index as f32 * 28.,
                        anchor.width - 8.,
                        28.,
                    );
                    b.text(
                        r(rect.x + 8., rect.y + 6., rect.width - 16., 20.),
                        *label,
                        12.,
                        400,
                        p.ink,
                    );
                    b.control(
                        ResultTarget::Language(source, (*code).into()),
                        rect,
                        label,
                        ResultRole::MenuItem,
                        None,
                        true,
                    );
                }
            }
        }
        self.controls = b
            .controls
            .iter()
            .cloned()
            .map(|mut c| {
                c.rect = c.rect.scaled(self.scale);
                c.hit_rect = c.hit_rect.scaled(self.scale);
                c
            })
            .collect();
        ResultFrame {
            scene: b.scene.scaled(self.scale),
            controls: self.controls.clone(),
            panel: panel.scaled(self.scale),
            cards: cards.into_iter().map(|r| r.scaled(self.scale)).collect(),
        }
    }
    fn ocr(
        &mut self,
        b: &mut Paint,
        y: &mut f32,
        width: f32,
        window_width: f32,
        metrics: &mut impl TextMeasurer,
    ) {
        let p = b.p;
        if let Some(language) = &self.snapshot.ocr_language {
            b.text(
                r(20., *y, width, 16.),
                format!("识别语言：{language}"),
                10.5,
                400,
                p.muted,
            );
            *y += 35.;
        }
        if let Some(image) = &self.snapshot.ocr_image {
            b.text(r(20., *y, width, 18.), "原图", 13., 700, p.muted);
            *y += 26.;
            b.text(
                r(window_width - 66., *y - 24., 46., 16.),
                "source",
                10.5,
                400,
                p.faint,
            );
            let image_height =
                ((width - 2.) * image.height() as f32 / image.width() as f32).min(220.);
            let image_box = r(20., *y, width, image_height + 2.);
            b.fill(image_box, 13., p.sunken);
            b.stroke(image_box, 13., p.line);
            let image_scale =
                ((width - 2.) / image.width() as f32).min(image_height / image.height() as f32);
            let image_width = image.width() as f32 * image_scale;
            b.scene.commands.push(DrawCommand::Image {
                rect: r(
                    21. + (width - 2. - image_width) / 2.,
                    *y + 1.,
                    image_width,
                    image_height,
                ),
                image: image.clone(),
            });
            *y += image_box.height + 8.;
            let rect = r(20., *y, width, 32.);
            b.fill(rect, 8., p.good_soft);
            b.stroke(rect, 8., p.good);
            icons::draw(
                &mut b.scene,
                r(20. + width / 2. - 53., *y + 8., 16., 16.),
                Icon::Upload,
                p.good,
            );
            b.text(
                r(20. + width / 2. - 29., *y + 7., 90., 20.),
                "上传图片 OCR",
                13.,
                700,
                p.good,
            );
            b.control(
                ResultTarget::FileOcr,
                rect,
                "上传图片 OCR",
                ResultRole::Button,
                None,
                !self.snapshot.ocr_running && !self.snapshot.dialog_open,
            );
            *y += 44.;
        } else {
            b.text(r(20., *y, width / 2., 18.), "图片", 13., 700, p.muted);
            b.text(
                r(window_width - 112., *y + 2., 92., 16.),
                "PNG/JPG/WebP",
                10.5,
                400,
                p.faint,
            );
            *y += 30.;
            let rect = r(20., *y, width, 119.);
            b.fill(rect, b.radius, p.sunken);
            b.stroke(rect, b.radius, p.line_strong);
            b.fill(
                r(20. + width / 2. - 18., *y + 13., 36., 36.),
                7.,
                p.good_soft,
            );
            icons::draw(
                &mut b.scene,
                r(20. + width / 2. - 10., *y + 21., 20., 20.),
                Icon::Upload,
                p.good,
            );
            b.centered(
                r(20., *y + 63., width, 20.),
                if self.snapshot.ocr_running {
                    "识别中..."
                } else {
                    "上传图片 OCR"
                },
                13.,
                p.ink,
            );
            b.centered(
                r(20., *y + 91., width, 17.),
                "选择一张图片后自动开始识别",
                10.5,
                p.muted,
            );
            b.control(
                ResultTarget::FileOcr,
                rect,
                "上传图片 OCR",
                ResultRole::Button,
                None,
                !self.snapshot.ocr_running && !self.snapshot.dialog_open,
            );
            *y += 131.;
        }
        b.text(
            r(20., *y, width / 2., 18.),
            if self.snapshot.ocr_image.is_some() {
                "OCR 结果"
            } else {
                "识别文本"
            },
            13.,
            700,
            p.muted,
        );
        b.text(
            r(window_width - 66., *y + 2., 46., 16.),
            format!("{} chars", self.snapshot.ocr_text.chars().count()),
            10.5,
            400,
            p.faint,
        );
        *y += 30.;
        let (size, line) = text_style(&self.snapshot.ocr_text, true, false);
        let layout = metrics.line_layout(
            &self.snapshot.ocr_text,
            width - 26.,
            size,
            400,
            FontFamily::Sans,
            line,
        );
        let height = (layout.height + 24. + 28.).floor().max(106.);
        let baseline_gap = if self.snapshot.ocr_image.is_some() {
            1.
        } else {
            8.
        };
        let outer = r(20., *y, width, height + baseline_gap + 42.);
        b.box_frame(outer, true);
        self.text_field(
            b,
            ResultTarget::OcrText,
            r(21., *y + 1., width - 2., height),
            r(33., *y + 13., width - 26., height - 24.),
            "识别文本",
            self.snapshot.ocr_text.clone(),
            size,
            line,
            metrics,
        );
        let mut token_height = 0.;
        if self.snapshot.ocr_image.is_some() && !self.snapshot.ocr_tokens.is_empty() {
            let top = *y + height + baseline_gap;
            let first_command = b.scene.commands.len();
            let mut x = 33.;
            let mut row = top + 33.;
            b.text(
                r(33., top + 9., width - 26., 16.),
                "推荐复制片段",
                10.5,
                700,
                p.muted,
            );
            b.text(
                r(window_width - 52., top + 9., 20., 16.),
                format!("{} 个", self.snapshot.ocr_tokens.len()),
                10.5,
                400,
                p.faint,
            );
            for token in &self.snapshot.ocr_tokens {
                let span_width = |metrics: &mut dyn TextMeasurer, text: &str| {
                    metrics
                        .measure(text, 10.5, 400, FontFamily::WindowsUi)
                        .caret_stops
                        .iter()
                        .map(|(_, x)| *x)
                        .fold(0., f32::max)
                };
                let label_width = span_width(metrics, &token.label);
                let value_width = span_width(metrics, &token.value);
                let token_width = (label_width + value_width + 26.).min(width - 26.);
                if x + token_width > window_width - 33. {
                    x = 33.;
                    row += 29.;
                }
                let rect = r(x, row, token_width, 23.);
                b.fill(rect, 12., p.surface);
                b.stroke(rect, 12., p.line);
                b.scene.commands.push(DrawCommand::Text {
                    rect: r(x + 11., row + 5., label_width, 14.),
                    text: token.label.clone(),
                    size: 10.5,
                    weight: 400,
                    color: p.faint,
                    font: FontFamily::WindowsUi,
                    align: TextAlign::Left,
                });
                b.scene.commands.push(DrawCommand::Text {
                    rect: r(
                        x + 15. + label_width,
                        row + 5.,
                        (token_width - label_width - 26.).max(1.),
                        14.,
                    ),
                    text: token.value.clone(),
                    size: 10.5,
                    weight: 400,
                    color: p.ink,
                    font: FontFamily::WindowsUi,
                    align: TextAlign::Left,
                });
                b.control(
                    ResultTarget::CopyToken(token.value.clone()),
                    rect,
                    &format!("复制{}: {}", token.label, token.value),
                    ResultRole::Button,
                    None,
                    true,
                );
                x += token_width + 6.;
            }
            token_height = row + 31. - top;
            b.scene.commands.insert(
                first_command,
                DrawCommand::FillRect {
                    rect: r(21., top, width - 2., token_height),
                    radius: 0.,
                    color: [p.sunken[0], p.sunken[1], p.sunken[2], 153],
                },
            );
            b.scene.commands.insert(
                first_command + 1,
                DrawCommand::FillRect {
                    rect: r(21., top, width - 2., 1.),
                    radius: 0.,
                    color: p.line,
                },
            );
        }
        if token_height > 0. {
            for command in &mut b.scene.commands {
                match command {
                    DrawCommand::FillRect { rect, .. } | DrawCommand::StrokeRect { rect, .. }
                        if *rect == outer =>
                    {
                        rect.height += token_height
                    }
                    _ => {}
                }
            }
        }
        let footer = *y + height + baseline_gap + token_height;
        b.fill(r(21., footer, width - 2., 41.), 0., p.sunken);
        b.fill(r(21., footer, width - 2., 1.), 0., p.line);
        b.text(
            r(33., footer + 14., width - 180., 16.),
            format!("{} chars", self.snapshot.ocr_text.chars().count()),
            10.5,
            400,
            p.faint,
        );
        let enabled = !self.snapshot.ocr_text.trim().is_empty();
        for (index, (target, label, icon, active)) in [
            (ResultTarget::SpeakOcr, "朗读", Icon::Volume, enabled),
            (
                ResultTarget::FavoriteOcr,
                if self.snapshot.ocr_favorite {
                    "已收藏"
                } else {
                    "收藏"
                },
                Icon::Favorite,
                enabled && !self.snapshot.ocr_favorite && !self.snapshot.favorite_pending,
            ),
            (ResultTarget::CopyOcr, "复制", Icon::Copy, enabled),
            (ResultTarget::ClearOcr, "清空", Icon::Clear, enabled),
        ]
        .into_iter()
        .enumerate()
        {
            b.icon(
                target,
                r(
                    window_width - 169. + index as f32 * 36.,
                    footer + 7.,
                    28.,
                    28.,
                ),
                label,
                icon,
                active,
                true,
            );
        }
        *y += outer.height + token_height + 4.;
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
                        b.scene.ime_cursor = Some(caret);
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
            font: if self.snapshot.shell.theme == Theme::Paper {
                FontFamily::Serif
            } else {
                FontFamily::Sans
            },
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

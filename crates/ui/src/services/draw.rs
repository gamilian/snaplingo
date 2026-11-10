use super::*;
use crate::theme::{palette, Palette};
use crate::{
    Color, DrawCommand, FontFamily, Page, Point, Rect, Scene, Size, TextAlign, TextMeasurer, Theme,
};
use unicode_segmentation::UnicodeSegmentation;

struct Draw {
    scene: Scene,
    controls: Vec<ServicesControlInfo>,
    epoch: u64,
    p: Palette,
    clip: Rect,
    focus: Option<ServiceControl>,
}
fn r(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect {
        x,
        y,
        width: width.max(0.),
        height: height.max(0.),
    }
}
impl Draw {
    fn fill(&mut self, rect: Rect, radius: f32, color: Color) {
        self.scene.commands.push(DrawCommand::FillRect {
            rect,
            radius,
            color,
        });
    }
    fn border(&mut self, rect: Rect, radius: f32, color: Color) {
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
    fn block(&mut self, rect: Rect, text: impl Into<String>, size: f32, color: Color) {
        self.scene.commands.push(DrawCommand::TextBlock {
            rect,
            text: text.into(),
            size,
            weight: 400,
            color,
            font: FontFamily::Sans,
            line_height: 20.,
        });
    }
    fn control(
        &mut self,
        rect: Rect,
        control: ServiceControl,
        role: ServicesRole,
        label: impl Into<String>,
        enabled: bool,
        selected: bool,
        value: Option<String>,
    ) {
        let hit_rect = rect.intersect(self.clip);
        if self.focus.as_ref() == Some(&control) {
            self.scene.commands.push(DrawCommand::StrokeRect {
                rect: r(rect.x - 2., rect.y - 2., rect.width + 4., rect.height + 4.),
                radius: 7.,
                width: 2.,
                color: self.p.accent,
            });
        }
        self.controls.push(ServicesControlInfo {
            target: ServicesTarget {
                revision: self.epoch,
                control,
            },
            rect,
            hit_rect,
            role,
            label: label.into(),
            enabled,
            selected,
            value,
            range: None,
        });
    }
    fn button(
        &mut self,
        rect: Rect,
        control: ServiceControl,
        label: &str,
        enabled: bool,
        selected: bool,
    ) {
        let segmented = matches!(
            control,
            ServiceControl::Protocol(_) | ServiceControl::ProtocolFamily(_)
        );
        let icon = matches!(
            control,
            ServiceControl::Configure(_) | ServiceControl::TestSaved(_) | ServiceControl::Remove(_)
        ) || (control == ServiceControl::CancelForm && label == "‹");
        let primary = matches!(control, ServiceControl::Speak | ServiceControl::SaveForm)
            || matches!(control, ServiceControl::SetActive { active: true, .. });
        // The retained prompt-save button uses raised/faint styling even while
        // enabled; its actual click and enabled state are covered by references.
        let muted_prompt = control == ServiceControl::SavePrompt;
        let foreground = if !enabled || muted_prompt {
            self.p.faint
        } else if primary {
            self.p.accent_on
        } else if selected && !segmented {
            self.p.accent
        } else {
            self.p.ink
        };
        if !icon || control == ServiceControl::CancelForm {
            self.fill(
                rect,
                if icon { 18. } else { 8. },
                if !enabled || muted_prompt {
                    self.p.raised
                } else if primary {
                    self.p.accent
                } else if control == ServiceControl::CancelForm
                    && label == "取消"
                    && rect.height == 40.
                {
                    self.p.raised
                } else if segmented {
                    if selected {
                        self.p.surface
                    } else {
                        self.p.raised
                    }
                } else if selected {
                    self.p.accent_soft
                } else {
                    self.p.surface
                },
            );
            if !segmented
                && !primary
                && !(control == ServiceControl::CancelForm && label == "取消" && rect.height == 40.)
            {
                self.border(
                    rect,
                    if icon { 18. } else { 8. },
                    if selected { self.p.accent } else { self.p.line },
                );
            }
        }
        if icon {
            let points: &[&[(f32, f32)]] = match control {
                ServiceControl::Configure(_) => &[
                    &[
                        (13.5, 6.5),
                        (17.5, 10.5),
                        (9., 19.),
                        (5., 19.),
                        (5., 15.),
                        (13.5, 6.5),
                    ],
                    &[(12., 8.), (16., 12.)],
                ],
                ServiceControl::TestSaved(_) => &[&[
                    (3., 12.),
                    (7., 12.),
                    (10., 4.),
                    (14., 20.),
                    (17., 12.),
                    (21., 12.),
                ]],
                ServiceControl::Remove(_) => &[
                    &[(4., 7.), (20., 7.)],
                    &[(9., 7.), (9., 4.), (15., 4.), (15., 7.)],
                    &[(6., 7.), (7., 20.), (17., 20.), (18., 7.)],
                    &[(10., 10.), (10., 17.)],
                    &[(14., 10.), (14., 17.)],
                ],
                _ => &[&[(15., 19.), (8., 12.), (15., 5.)]],
            };
            for path in points {
                for pair in path.windows(2) {
                    self.scene.commands.push(DrawCommand::Line {
                        from: Point {
                            x: rect.x + 10. + pair[0].0 * 16. / 24.,
                            y: rect.y + 10. + pair[0].1 * 16. / 24.,
                        },
                        to: Point {
                            x: rect.x + 10. + pair[1].0 * 16. / 24.,
                            y: rect.y + 10. + pair[1].1 * 16. / 24.,
                        },
                        width: 1.35,
                        color: self.p.muted,
                    });
                }
            }
        } else {
            let leading = matches!(
                control,
                ServiceControl::SetActive { .. } | ServiceControl::AddCustom
            );
            self.text(
                r(
                    rect.x + if leading { 32. } else { 10. },
                    rect.y + (rect.height - 18.) / 2.,
                    rect.width - if leading { 42. } else { 20. },
                    20.,
                ),
                label,
                if control == ServiceControl::AddCustom {
                    10.5
                } else {
                    12.5
                },
                500,
                foreground,
            );
            if !matches!(
                control,
                ServiceControl::VoiceMenu
                    | ServiceControl::Voice(_)
                    | ServiceControl::ChooseModel(_)
            ) {
                if let Some(DrawCommand::Text { align, .. }) = self.scene.commands.last_mut() {
                    *align = TextAlign::Center;
                }
            }
            if leading {
                let paths: &[&[(f32, f32)]] = if control == ServiceControl::AddCustom {
                    &[&[(12., 5.), (12., 19.)], &[(5., 12.), (19., 12.)]]
                } else {
                    &[&[(7., 4.), (19., 12.), (7., 20.), (7., 4.)]]
                };
                for path in paths {
                    for pair in path.windows(2) {
                        self.scene.commands.push(DrawCommand::Line {
                            from: Point {
                                x: rect.x + 12. + pair[0].0 * 16. / 24.,
                                y: rect.y + 10. + pair[0].1 * 16. / 24.,
                            },
                            to: Point {
                                x: rect.x + 12. + pair[1].0 * 16. / 24.,
                                y: rect.y + 10. + pair[1].1 * 16. / 24.,
                            },
                            width: 1.5,
                            color: foreground,
                        });
                    }
                }
            }
            if control == ServiceControl::VoiceMenu {
                for (a, b) in [((0., 0.), (4., 4.)), ((4., 4.), (8., 0.))] {
                    self.scene.commands.push(DrawCommand::Line {
                        from: Point {
                            x: rect.x + rect.width - 28. + a.0,
                            y: rect.y + 19. + a.1,
                        },
                        to: Point {
                            x: rect.x + rect.width - 28. + b.0,
                            y: rect.y + 19. + b.1,
                        },
                        width: 1.5,
                        color: self.p.muted,
                    });
                }
            }
        }
        self.control(
            rect,
            control,
            ServicesRole::Button,
            label,
            enabled,
            selected,
            None,
        );
    }
    fn notice(&mut self, x: f32, y: &mut f32, width: f32, text: &str, error: bool) {
        let lines = (text.chars().count() as f32 * 6. / width.max(20.))
            .ceil()
            .max(1.);
        let height = lines * 20.;
        self.block(
            r(x, *y, width, height),
            text,
            11.,
            if error { self.p.danger } else { self.p.muted },
        );
        *y += height + 12.;
    }
}
impl ServicesUi {
    pub fn frame(
        &mut self,
        size: Size,
        scale: f32,
        measurer: &mut impl TextMeasurer,
    ) -> ServicesFrame {
        self.scale = if scale.is_finite() {
            scale.clamp(0.5, 3.)
        } else {
            1.
        };
        let logical = Size {
            width: (size.width / self.scale).max(1.),
            height: (size.height / self.scale).max(1.),
        };
        let mut frame = self.draw(logical, measurer);
        if self.scroll > self.scroll_limit {
            self.scroll = self.scroll_limit;
            frame = self.draw(logical, measurer);
        }
        frame.scene = frame.scene.scaled(self.scale);
        for control in &mut frame.controls {
            control.rect = control.rect.scaled(self.scale);
            control.hit_rect = control.hit_rect.scaled(self.scale);
        }
        self.frame = Some(frame.clone());
        frame
    }
    fn draw(&mut self, size: Size, measurer: &mut impl TextMeasurer) -> ServicesFrame {
        self.provider_rows.clear();
        self.input_layouts.clear();
        let p = palette(self.snapshot.shell.theme, self.snapshot.shell.dark);
        let scene =
            crate::settings::canvas::shell_frame(&self.snapshot.shell, Page::Services, size);
        let content = scene.content;
        let paper = self.snapshot.shell.theme == Theme::Paper;
        let studio = self.snapshot.shell.theme == Theme::Studio;
        let narrow = size.width <= 700.;
        let context = if !narrow && (paper || studio) {
            if size.width <= 900. {
                139.
            } else {
                164.
            }
        } else {
            0.
        };
        let pane = r(
            content.x + context,
            content.y,
            content.width - context,
            content.height,
        );
        let margin = if size.width <= 900. {
            22.
        } else if paper {
            38.
        } else if studio {
            28.
        } else {
            32.
        };
        let scrolling = self.snapshot.form.is_some()
            || (paper && self.snapshot.route == ServicesRoute::Translation);
        let scrollbar = if scrolling { 8. } else { 0. };
        let width = (pane.width - scrollbar).min(1080.) - margin * 2.;
        let x =
            pane.x + (pane.width - scrollbar - (pane.width - scrollbar).min(1080.)) / 2. + margin;
        let mut d = Draw {
            clip: r(0., 0., size.width, size.height),
            controls: vec![],
            scene,
            epoch: self.epoch,
            p,
            focus: self.focus.clone(),
        };
        for hit in d.scene.hits.clone() {
            d.control(
                hit.rect,
                ServiceControl::Shell(hit.target),
                ServicesRole::Tab,
                hit.label,
                true,
                false,
                None,
            );
        }
        let page_top = pane.y
            + if paper {
                33.
            } else if studio {
                29.
            } else {
                30.
            };
        let top = (page_top - self.scroll).max(pane.y);
        let header_bottom = top + if paper { 89. } else { 82. };
        if context > 0. {
            d.fill(
                r(content.x, content.y, context, content.height),
                0.,
                p.canvas,
            );
            d.text(
                r(
                    content.x + 18.,
                    content.y + if paper { 35. } else { 28. },
                    context - 30.,
                    30.,
                ),
                "服务",
                if paper { 22. } else { 15. },
                600,
                p.ink,
            );
            if paper {
                if let Some(DrawCommand::Text { font, weight, .. }) = d.scene.commands.last_mut() {
                    *font = FontFamily::Serif;
                    *weight = 400;
                }
            }
            d.text(
                r(
                    content.x + 18.,
                    content.y + if paper { 68. } else { 61. },
                    context - 30.,
                    18.,
                ),
                "CONTENTS",
                9.,
                400,
                p.faint,
            );
            for (index, route) in ServicesRoute::ALL.into_iter().enumerate() {
                let rect = r(
                    content.x + 9.,
                    content.y + if paper { 110. } else { 92. } + index as f32 * 32.,
                    context - 18. - if studio { 1. } else { 0. },
                    30.,
                );
                let selected = self.snapshot.route == route;
                if selected {
                    d.fill(rect, 6., p.accent_soft);
                }
                d.text(
                    r(rect.x + 10., rect.y + 6., rect.width - 20., 18.),
                    route.label(),
                    12.5,
                    if selected { 600 } else { 400 },
                    if selected { p.accent } else { p.muted },
                );
                d.control(
                    rect,
                    ServiceControl::Route(route),
                    ServicesRole::Tab,
                    route.label(),
                    !self.snapshot.busy,
                    selected,
                    None,
                );
            }
        }
        d.text(
            r(
                x,
                top + if paper {
                    0.
                } else if studio {
                    4.
                } else {
                    2.
                },
                width,
                if paper {
                    46.
                } else if studio {
                    31.
                } else {
                    35.
                },
            ),
            "服务",
            if paper {
                37.
            } else if studio {
                25.
            } else {
                28.
            },
            600,
            p.ink,
        );
        if paper {
            if let Some(DrawCommand::Text { font, weight, .. }) = d.scene.commands.last_mut() {
                *font = FontFamily::Serif;
                *weight = 400;
            }
        }
        let title_w = if paper {
            71.421875
        } else if studio {
            48.90625
        } else {
            54.78125
        };
        let dot = r(
            x + title_w + 10.,
            top + if paper { 20. } else { 16.5 },
            6.,
            6.,
        );
        d.fill(r(dot.x - 3., dot.y - 3., 12., 12.), 6., p.good_soft);
        d.fill(dot, 3., p.good);
        d.text(
            r(
                x + title_w + 22.,
                top + if paper { 15.5 } else { 12. },
                70.,
                15.,
            ),
            "自动保存",
            10.5,
            500,
            p.muted,
        );
        d.text(
            r(
                x,
                top + if paper {
                    50.
                } else if studio {
                    39.
                } else {
                    41.
                },
                width,
                18.,
            ),
            "配置 OCR、翻译与语音合成引擎",
            12.5,
            400,
            p.muted,
        );
        if context == 0. {
            let mut tx = x + width - 224.875;
            let ty = top + if paper { 21.5 } else { 18. };
            d.fill(r(tx - 3., ty - 3., 224.875 + 6., 31.), 6., p.raised);
            for (route, tab_width) in [
                (ServicesRoute::Ocr, 75.875),
                (ServicesRoute::Translation, 71.),
                (ServicesRoute::Tts, 71.),
            ] {
                let rect = r(tx, ty, tab_width, 25.);
                if self.snapshot.route == route {
                    d.fill(rect, 3., p.surface);
                }
                d.text(
                    r(tx + 10., ty + 3., tab_width - 20., 19.),
                    route.label(),
                    12.5,
                    500,
                    if self.snapshot.route == route {
                        p.ink
                    } else {
                        p.muted
                    },
                );
                d.control(
                    rect,
                    ServiceControl::Route(route),
                    ServicesRole::Tab,
                    route.label(),
                    !self.snapshot.busy,
                    self.snapshot.route == route,
                    None,
                );
                tx += tab_width + 2.;
            }
        }
        d.fill(r(x, header_bottom - 1., width, 1.), 0., p.line);
        self.viewport = r(
            pane.x,
            header_bottom,
            pane.width,
            (size.height - header_bottom).max(1.),
        );
        d.scene.viewport = self.viewport;
        d.clip = self.viewport;
        d.scene.commands.push(DrawCommand::PushClip(self.viewport));
        let mut y = page_top + if paper { 89. } else { 82. } + 24. - self.scroll;
        if let Some(error) = self.snapshot.error.clone() {
            d.notice(x, &mut y, width, &error, true);
        }
        if self.snapshot.form.is_some() {
            if self.snapshot.route == ServicesRoute::Ocr {
                self.draw_providers(&mut d, x, &mut y, width, measurer);
                y += 12.;
            }
            self.draw_form(&mut d, x, &mut y, width, measurer);
        } else {
            match self.snapshot.route {
                ServicesRoute::Ocr | ServicesRoute::Translation => {
                    self.draw_providers(&mut d, x, &mut y, width, measurer)
                }
                ServicesRoute::Tts => self.draw_tts(&mut d, x, &mut y, width),
            }
        }
        if self.snapshot.busy && self.snapshot.form.is_none() {
            d.notice(x, &mut y, width, "正在处理…", false);
        }
        let height = y + self.scroll + 56. - content.y;
        d.scene.content_height = height;
        self.scroll_limit = (height - content.height).max(0.);
        d.scene.commands.push(DrawCommand::PopClip);
        ServicesFrame {
            scene: d.scene,
            controls: d.controls,
        }
    }
    fn draw_providers(
        &mut self,
        d: &mut Draw,
        x: f32,
        y: &mut f32,
        width: f32,
        measurer: &mut impl TextMeasurer,
    ) {
        let main_x = x;
        let x = x - 21.;
        let width = width + 42.;
        *y += 1.;
        let padding = match self.snapshot.shell.theme {
            Theme::Studio => 16.,
            Theme::Paper => 18.,
            _ => 20.,
        };
        let mut providers = self.snapshot.providers.iter().collect::<Vec<_>>();
        if self.snapshot.route == ServicesRoute::Translation {
            providers.sort_by_key(|provider| !provider.active);
        }
        let start = *y;
        let base_row_h = match self.snapshot.shell.theme {
            Theme::Studio => 65.,
            Theme::Paper => 98.,
            _ => 72.,
        };
        for (index, provider) in providers.into_iter().enumerate() {
            let row_h = base_row_h
                + if self.snapshot.route == ServicesRoute::Ocr
                    && self.snapshot.shell.theme != Theme::Native
                    && index > 0
                {
                    1.
                } else {
                    0.
                };
            let row = r(x, *y, width, row_h);
            self.provider_rows.push((provider.id.clone(), row));
            let hovered = self.hover.as_ref().is_some_and(|control| match control {
                ServiceControl::Drag(id)
                | ServiceControl::Configure(id)
                | ServiceControl::TestSaved(id)
                | ServiceControl::Remove(id) => id == &provider.id,
                ServiceControl::SetActive { provider_id, .. } => provider_id == &provider.id,
                _ => false,
            });
            d.fill(
                row,
                0.,
                if provider.active {
                    let mut color = d.p.accent_soft;
                    for channel in 0..3 {
                        color[channel] = ((u16::from(d.p.accent_soft[channel]) * 3
                            + u16::from(d.p.surface[channel]) * 2)
                            / 5) as u8;
                    }
                    color
                } else if hovered {
                    d.p.sunken
                } else {
                    d.p.surface
                },
            );
            d.fill(r(x, *y + row_h - 1., width, 1.), 0., d.p.line);
            if provider.active {
                d.fill(r(x, *y + 8., 3., row_h - 16.), 1., d.p.accent);
            }
            let drag_rect = r(
                x + padding,
                *y + (row_h - 32.) / 2.
                    + if self.snapshot.shell.theme == Theme::Studio {
                        0.
                    } else if self.snapshot.shell.theme == Theme::Paper {
                        0.
                    } else {
                        -1.5
                    },
                20.,
                32.,
            );
            if self.snapshot.route == ServicesRoute::Translation {
                d.control(
                    drag_rect,
                    ServiceControl::Drag(provider.id.clone()),
                    ServicesRole::DragHandle,
                    "拖动排序",
                    !self.snapshot.busy,
                    false,
                    None,
                );
            }
            if self.snapshot.route == ServicesRoute::Ocr
                || hovered
                || self
                    .focus
                    .as_ref()
                    .is_some_and(|c| matches!(c,ServiceControl::Drag(id)if id==&provider.id))
            {
                for index in 0..6 {
                    d.fill(
                        r(
                            x + padding + 5. + (index % 2) as f32 * 6.,
                            *y + (row_h - 16.) / 2. + (index / 2) as f32 * 6.,
                            4.,
                            4.,
                        ),
                        1.5,
                        d.p.faint,
                    );
                }
            }
            let icon_color = match provider.id.as_str() {
                "google-translate" | "baidu-translate" | "baidu-ocr" => d.p.accent,
                "system-ocr" => d.p.good,
                _ => d.p.muted,
            };
            let icon_background = match provider.id.as_str() {
                "google-translate" | "baidu-translate" | "baidu-ocr" => d.p.accent_soft,
                "system-ocr" => d.p.good_soft,
                _ => d.p.sunken,
            };
            let icon_rect = r(
                x + padding + 36.,
                action_icon_y(
                    *y,
                    row_h,
                    self.snapshot.shell.theme,
                    self.snapshot.route,
                    index,
                ),
                36.,
                36.,
            );
            d.fill(icon_rect, 12., icon_background);
            d.border(
                icon_rect,
                12.,
                if matches!(
                    provider.id.as_str(),
                    "google-translate" | "baidu-translate" | "baidu-ocr" | "system-ocr"
                ) {
                    icon_background
                } else {
                    d.p.line
                },
            );
            let name = provider.name.trim();
            let latin = name.chars().take(2).all(|c| c.is_ascii_alphabetic());
            let initials = name
                .chars()
                .take(if latin { 2 } else { 1 })
                .collect::<String>();
            d.text(
                r(x + padding + 42., *y + (row_h - 20.) / 2., 30., 20.),
                initials.to_uppercase(),
                10.5,
                600,
                icon_color,
            );
            let custom = self.snapshot.route == ServicesRoute::Translation && !provider.builtin;
            let actions = if custom {
                203.5
            } else if provider.configured {
                115.5
            } else {
                36.
            };
            let text_x = x + padding + 88.;
            let available = (width - padding * 2. - 88. - 16. - actions).max(0.);
            let title_size = match self.snapshot.shell.theme {
                Theme::Studio => 14.,
                Theme::Paper => 20.,
                _ => 13.,
            };
            let title_h = if self.snapshot.shell.theme == Theme::Paper {
                30.
            } else if self.snapshot.shell.theme == Theme::Studio {
                21.
            } else {
                19.
            };
            let title_y = *y
                + if self.snapshot.shell.theme == Theme::Paper {
                    23.
                } else {
                    padding.min(14.)
                };
            let name_width = measurer
                .measure(&provider.name, title_size, 600, FontFamily::Sans)
                .caret_stops
                .iter()
                .map(|(_, x)| *x)
                .fold(0., f32::max);
            let badges: Vec<(&str, f32)> = if provider.configured {
                vec![(
                    if provider.local {
                        "本地处理"
                    } else {
                        "发送到服务"
                    },
                    if provider.local { 58.9375 } else { 69.65625 },
                )]
            } else {
                vec![("未配置", 48.203125), ("发送到服务", 69.65625)]
            };
            let badge_width = badges.iter().map(|(_, w)| w + 8.).sum::<f32>();
            let shown_name = name_width.min((available - badge_width).max(0.));
            d.text(
                r(text_x, title_y, shown_name, title_h),
                &provider.name,
                title_size,
                if self.snapshot.shell.theme == Theme::Paper {
                    400
                } else {
                    600
                },
                d.p.ink,
            );
            if self.snapshot.shell.theme == Theme::Paper {
                if let Some(DrawCommand::Text { font, .. }) = d.scene.commands.last_mut() {
                    *font = FontFamily::Serif;
                }
            }
            let mut bx = text_x + shown_name + 8.;
            let (warning, warning_soft) =
                match (self.snapshot.shell.theme, self.snapshot.shell.dark) {
                    (Theme::Native, false) => ([160, 117, 50, 255], [251, 242, 227, 255]),
                    (Theme::Native, true) => ([217, 178, 129, 255], [73, 63, 50, 255]),
                    (Theme::Studio, false) => ([155, 120, 63, 255], [240, 232, 219, 255]),
                    (Theme::Studio, true) => ([222, 184, 136, 255], [58, 51, 44, 255]),
                    (Theme::Paper, false) => ([163, 133, 79, 255], [244, 237, 219, 255]),
                    (Theme::Paper, true) => ([198, 177, 130, 255], [67, 62, 46, 255]),
                };
            for (badge, bw) in badges {
                let unconfigured = badge == "未配置";
                d.fill(
                    r(bx, title_y + (title_h - 19.) / 2., bw, 19.),
                    10.,
                    if unconfigured {
                        warning_soft
                    } else {
                        d.p.raised
                    },
                );
                d.text(
                    r(bx + 8., title_y + (title_h - 15.) / 2., bw - 16., 15.),
                    badge,
                    10.5,
                    500,
                    if unconfigured { warning } else { d.p.muted },
                );
                bx += bw + 8.;
            }
            d.text(
                r(text_x, title_y + title_h + 4., available, 18.),
                provider.subtitle(),
                if self.snapshot.shell.theme == Theme::Studio {
                    11.
                } else {
                    12.5
                },
                400,
                if provider.endpoint.is_some() {
                    d.p.accent
                } else {
                    d.p.muted
                },
            );
            let action_commands = d.scene.commands.len();
            let mut ax = x + width - padding - actions;
            let action_y = *y
                + (row_h - 36.) / 2.
                + if self.snapshot.shell.theme == Theme::Native {
                    if self.snapshot.route == ServicesRoute::Ocr && index > 0 {
                        -0.5
                    } else {
                        -1.5
                    }
                } else {
                    0.
                };
            let enabled = !self.snapshot.busy;
            // OCR is a single selection and has no deactivation operation.
            if provider.configured {
                d.button(
                    r(ax, action_y, 71.5, 36.),
                    ServiceControl::SetActive {
                        provider_id: provider.id.clone(),
                        active: self.snapshot.route == ServicesRoute::Ocr || !provider.active,
                    },
                    if provider.active { "停用" } else { "启用" },
                    enabled,
                    false,
                );
                ax += 79.5;
            }
            d.button(
                r(ax, action_y, 36., 36.),
                ServiceControl::Configure(provider.id.clone()),
                "编辑",
                enabled,
                false,
            );
            ax += 44.;
            if self.snapshot.route == ServicesRoute::Translation && !provider.builtin {
                d.button(
                    r(ax, action_y, 36., 36.),
                    ServiceControl::TestSaved(provider.id.clone()),
                    "测试",
                    enabled,
                    false,
                );
                ax += 44.;
                d.button(
                    r(ax, action_y, 36., 36.),
                    ServiceControl::Remove(provider.id.clone()),
                    "删除",
                    enabled,
                    false,
                );
            }
            let focused=self.focus.as_ref().is_some_and(|control|matches!(control,ServiceControl::Configure(id)|ServiceControl::Drag(id)|ServiceControl::TestSaved(id)|ServiceControl::Remove(id)if id==&provider.id)||matches!(control,ServiceControl::SetActive{provider_id,..}if provider_id==&provider.id));
            if !hovered && !focused {
                d.scene.commands.truncate(action_commands);
            }
            if self
                .drag
                .as_ref()
                .is_some_and(|(_, target)| target.as_ref() == Some(&provider.id))
            {
                d.border(row, 1., d.p.accent);
            }
            *y += row_h
                + if self.snapshot.route == ServicesRoute::Translation {
                    1.
                } else {
                    0.
                };
        }
        if self.snapshot.route == ServicesRoute::Ocr {
            *y += 1.;
        }
        d.border(r(x, start, width, *y - start), 8., d.p.line);
        if self.snapshot.route == ServicesRoute::Translation {
            *y += 12.;
            d.button(
                r(main_x, *y, 129.125, 36.),
                ServiceControl::AddCustom,
                "添加自定义服务",
                !self.snapshot.busy,
                false,
            );
            *y += 48.;
        }
    }
    fn draw_tts(&mut self, d: &mut Draw, x: f32, y: &mut f32, width: f32) {
        let start = *y;
        let paper = self.snapshot.shell.theme == Theme::Paper;
        let studio = self.snapshot.shell.theme == Theme::Studio;
        let narrow = d.scene.content.width + d.scene.content.x <= 700.;
        let inset = if paper { 0. } else { 22. };
        let row_x = x + inset;
        let row_w = width - inset * 2.;
        let control_x = if narrow { row_x } else { row_x + row_w - 260. };
        let heights = if narrow {
            if paper {
                [121., 104., 97.]
            } else if studio {
                [107., 90., 83.]
            } else {
                [115., 98., 91.]
            }
        } else if paper {
            [74., 70., 69.]
        } else if studio {
            [60., 56., 55.]
        } else {
            [68., 64., 63.]
        };
        let padding = if paper {
            16.
        } else if studio {
            9.
        } else {
            13.
        };
        let mut yy = start;
        let mut voice_y = start;
        for (index, (title, description)) in [
            ("语音", "使用当前系统已安装的声音"),
            ("语速", "每分钟朗读字数"),
            ("试听", ""),
        ]
        .into_iter()
        .enumerate()
        {
            let row_h = heights[index];
            if index > 0 {
                d.fill(r(row_x, yy, row_w, 1.), 0., d.p.line);
            }
            d.text(
                r(row_x, yy + padding, row_w, 18.),
                title,
                12.5,
                600,
                d.p.ink,
            );
            if !description.is_empty() {
                d.text(
                    r(row_x, yy + padding + 20., row_w, 17.),
                    description,
                    10.5,
                    400,
                    d.p.muted,
                );
            }
            let cy = if narrow {
                yy + padding + if index == 2 { 29. } else { 47. }
            } else {
                yy + padding + if index == 2 { 1. } else { 0. }
            };
            match index {
                0 => {
                    voice_y = cy;
                    let label = self
                        .snapshot
                        .voices
                        .iter()
                        .find(|v| v.id == self.snapshot.voice_id)
                        .map(|v| format!("{} · {}", v.name, v.locale))
                        .unwrap_or("系统默认".into());
                    d.button(
                        r(control_x, cy, 260., 42.),
                        ServiceControl::VoiceMenu,
                        &label,
                        true,
                        self.voice_menu,
                    );
                    let c = d.controls.last_mut().unwrap();
                    c.role = ServicesRole::ComboBox;
                    c.value = Some(label);
                }
                1 => {
                    let cy = if narrow {
                        yy + padding + 60.
                    } else {
                        yy + (row_h - 6.) / 2. + 3.5
                    };
                    let track = r(control_x, cy, 209.34375, 6.);
                    d.fill(track, 3., d.p.line);
                    let knob = track.x
                        + track.width
                            * (self.snapshot.rate.saturating_sub(80) as f32 / 320.).clamp(0., 1.);
                    d.fill(r(track.x, track.y, knob - track.x, 6.), 3., d.p.accent);
                    let thumb = track.x
                        + 9.
                        + (track.width - 18.)
                            * (self.snapshot.rate.saturating_sub(80) as f32 / 320.).clamp(0., 1.);
                    d.fill(r(thumb - 9., cy - 6., 18., 18.), 9., [255, 255, 255, 255]);
                    d.scene.commands.push(DrawCommand::StrokeRect {
                        rect: r(thumb - 8., cy - 5., 16., 16.),
                        radius: 8.,
                        width: 2.,
                        color: d.p.accent,
                    });
                    d.text(
                        r(control_x + 221.34375, cy - 5., 38.65625, 16.),
                        self.snapshot.rate.to_string(),
                        10.5,
                        500,
                        d.p.muted,
                    );
                    d.control(
                        track,
                        ServiceControl::Rate,
                        ServicesRole::Slider,
                        "语速",
                        true,
                        false,
                        Some(self.snapshot.rate.to_string()),
                    );
                    d.controls.last_mut().unwrap().range = Some((80, 400, 10));
                }
                _ => {
                    let speak_x = if narrow {
                        control_x
                    } else {
                        row_x + row_w - 83.
                    };
                    d.button(
                        r(speak_x, cy, 83., 36.),
                        ServiceControl::Speak,
                        "测试朗读",
                        !self.snapshot.busy,
                        true,
                    );
                    if let Some(status) = &self.snapshot.status {
                        d.text(
                            r(speak_x + 95., cy + 9., 160., 20.),
                            status,
                            10.5,
                            400,
                            d.p.muted,
                        );
                    }
                }
            }
            yy += row_h;
        }
        *y = yy;
        if self.voice_menu {
            let popup_h = ((self.snapshot.voices.len() + 1) as f32 * 34. + 8.)
                .min(280.)
                .min((d.clip.height - 8.).max(34.));
            let yy = (voice_y + 42.)
                .min(d.clip.y + d.clip.height - popup_h)
                .max(d.clip.y);
            let popup = r(control_x, yy, 260., popup_h);
            self.voice_view = popup;
            self.voice_limit =
                (((self.snapshot.voices.len() + 1) as f32 * 34. + 8.) - popup.height).max(0.);
            d.fill(popup, 8., d.p.surface);
            d.border(popup, 8., d.p.line);
            let clip = d.clip;
            d.clip = popup.intersect(clip);
            d.scene.commands.push(DrawCommand::PushClip(d.clip));
            let mut yy = yy + 4. - self.voice_scroll;
            for (id, label) in std::iter::once((String::new(), "系统默认".into())).chain(
                self.snapshot.voices.iter().map(|voice| {
                    (
                        voice.id.clone(),
                        format!("{} · {}", voice.name, voice.locale),
                    )
                }),
            ) {
                d.button(
                    r(control_x + 4., yy, 252., 34.),
                    ServiceControl::Voice(id.clone()),
                    &label,
                    true,
                    self.snapshot.voice_id == id,
                );
                d.controls.last_mut().unwrap().role = ServicesRole::Option;
                yy += 34.;
            }
            d.scene.commands.push(DrawCommand::PopClip);
            d.clip = clip;
        }
    }
    fn draw_form(
        &mut self,
        d: &mut Draw,
        x: f32,
        y: &mut f32,
        width: f32,
        measurer: &mut impl TextMeasurer,
    ) {
        let form = self.snapshot.form.clone().unwrap();
        let busy = self.snapshot.busy;
        let credentials = matches!(form, ServiceForm::Credentials { .. });
        let header_x = x + if credentials { 24. } else { 0. };
        let header_y = *y + if credentials { 24. } else { 0. };
        d.button(
            r(header_x, header_y, 36., 36.),
            ServiceControl::CancelForm,
            "‹",
            !busy,
            false,
        );
        let title = match &form {
            ServiceForm::Credentials { name, .. } => format!("配置 {name}"),
            ServiceForm::Custom { draft, .. } => {
                if draft.provider_id.is_some() {
                    format!("配置 {}", draft.name)
                } else {
                    "添加自定义翻译服务".into()
                }
            }
        };
        d.text(
            r(header_x + 48., header_y + 7., width - 48., 22.),
            title,
            17.,
            600,
            d.p.ink,
        );
        *y += if credentials { 113. } else { 60. };
        match form {
            ServiceForm::Credentials {
                provider_id,
                fields,
                ..
            } => {
                let x = x + 24.;
                let width = width - 48.;
                let deep = provider_id == "deeplx";
                let standard = self
                    .credentials
                    .get("mode")
                    .is_some_and(|mode| mode == "deepl");
                if deep {
                    let panel = r(x, *y, width, 65.);
                    d.fill(panel, 8., d.p.sunken);
                    d.border(panel, 8., d.p.line);
                    d.text(
                        r(x + 16., *y + 12., width - 92., 24.),
                        "标准 DeepL",
                        16.,
                        500,
                        d.p.ink,
                    );
                    d.text(
                        r(x + 16., *y + 36., width - 92., 15.),
                        "关闭时使用 DeepLX 服务地址",
                        10.5,
                        400,
                        d.p.muted,
                    );
                    let toggle = r(x + width - 61., *y + 20.5, 44., 24.);
                    d.fill(
                        toggle,
                        12.,
                        if standard {
                            d.p.accent
                        } else {
                            d.p.line_strong
                        },
                    );
                    d.fill(
                        r(
                            toggle.x + if standard { 20. } else { 2. },
                            toggle.y + 2.,
                            20.,
                            20.,
                        ),
                        10.,
                        d.p.surface,
                    );
                    d.control(
                        toggle,
                        ServiceControl::DeepLMode(!standard),
                        ServicesRole::Switch,
                        "标准 DeepL",
                        !busy,
                        standard,
                        None,
                    );
                    *y += 81.;
                }
                let visible = if deep {
                    vec![ServiceCredentialField {
                        name: if standard { "api_key" } else { "endpoint" }.into(),
                        label: if standard {
                            "DeepL API Key"
                        } else {
                            "DeepLX API 地址"
                        }
                        .into(),
                        secret: standard,
                    }]
                } else {
                    fields
                };
                for field in &visible {
                    d.text(
                        r(x, *y, width, 24.),
                        format!("{} *", field.label),
                        16.,
                        500,
                        d.p.ink,
                    );
                    *y += 32.;
                    self.draw_input(
                        d,
                        ServiceField::Credential {
                            provider_id: provider_id.clone(),
                            name: field.name.clone(),
                        },
                        r(x, *y, width, 36.),
                        field.secret,
                        !busy,
                        &if deep && !standard {
                            "例如：https://deeplx.example.com".into()
                        } else {
                            format!("请输入 {}", field.label)
                        },
                        measurer,
                    );
                    *y += 60.;
                }
                if let Some(url) = match provider_id.as_str() {
                    "baidu-translate" => Some("https://fanyi-api.baidu.com/"),
                    "baidu-ocr" => Some("https://cloud.baidu.com/product/ocr"),
                    "deeplx" => Some("https://github.com/OwO-Network/DeepLX"),
                    _ => None,
                } {
                    d.text(
                        r(x, *y, width, 15.),
                        format!("{}：{url}", if deep { "项目地址" } else { "获取地址" }),
                        10.5,
                        400,
                        d.p.accent,
                    );
                    d.control(
                        r(x, *y, width.min(360.), 15.),
                        ServiceControl::OpenUrl(url.into()),
                        ServicesRole::Button,
                        url,
                        true,
                        false,
                        None,
                    );
                    *y += 39.;
                }
                d.fill(r(x, *y, width, 74.), 8., d.p.accent_soft);
                d.border(r(x, *y, width, 74.), 8., d.p.accent);
                d.text(
                    r(x + 48., *y + 16., width - 64., 19.),
                    "安全提示",
                    12.5,
                    500,
                    d.p.accent,
                );
                d.block(
                    r(x + 48., *y + 39., width - 64., 38.),
                    "凭证仅保存在本机 SnapLingo 数据库中，不会上传到任何服务器",
                    12.5,
                    d.p.accent,
                );
                *y += 123.;
                d.fill(r(x - 24., *y - 25., width + 48., 1.), 0., d.p.line);
                let valid = visible.iter().all(|field| {
                    self.credentials
                        .get(&field.name)
                        .is_some_and(|value| !value.trim().is_empty())
                });
                d.button(
                    r(x, *y, 57.5, 34.),
                    ServiceControl::CancelForm,
                    "取消",
                    !busy,
                    false,
                );
                d.button(
                    r(x + width - 99., *y, 99., 34.),
                    ServiceControl::SaveForm,
                    "保存配置",
                    !busy && valid,
                    true,
                );
                *y += 58.;
            }
            ServiceForm::Custom { draft, prompts } => {
                self.form_field(
                    d,
                    x,
                    y,
                    width,
                    ServiceCustomField::Name,
                    "名称",
                    false,
                    "例如：我的 GPT-4",
                    measurer,
                );
                d.text(r(x, *y, width, 18.), "协议", 12.5, 500, d.p.ink);
                *y += 22.;
                d.fill(r(x, *y, width, 42.), 8., d.p.raised);
                for (index, protocol) in [
                    ServiceProtocol::OpenAi,
                    ServiceProtocol::Anthropic,
                    ServiceProtocol::Gemini,
                ]
                .into_iter()
                .enumerate()
                {
                    let selected = protocol == draft.protocol
                        || (protocol == ServiceProtocol::OpenAi
                            && draft.protocol == ServiceProtocol::OpenAiResponses);
                    let bw = (width - 24.) / 3.;
                    d.button(
                        r(x + 4. + index as f32 * (bw + 8.), *y + 4., bw, 34.),
                        ServiceControl::ProtocolFamily(protocol),
                        protocol.label(),
                        !busy,
                        selected,
                    );
                }
                *y += 58.;
                if matches!(
                    draft.protocol,
                    ServiceProtocol::OpenAi | ServiceProtocol::OpenAiResponses
                ) {
                    d.text(r(x, *y, width, 18.), "OpenAI 模式", 12.5, 500, d.p.ink);
                    *y += 22.;
                    d.fill(r(x, *y, width, 42.), 8., d.p.raised);
                    for (index, (protocol, label)) in [
                        (ServiceProtocol::OpenAi, "Chat Completions"),
                        (ServiceProtocol::OpenAiResponses, "Responses"),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let bw = (width - 16.) / 2.;
                        d.button(
                            r(x + 4. + index as f32 * (bw + 8.), *y + 4., bw, 34.),
                            ServiceControl::Protocol(protocol),
                            label,
                            !busy,
                            draft.protocol == protocol,
                        );
                    }
                    *y += 58.;
                }
                self.form_field(
                    d,
                    x,
                    y,
                    width,
                    ServiceCustomField::Endpoint,
                    "API 地址",
                    false,
                    "https://api.openai.com",
                    measurer,
                );
                let preview = endpoint_preview(draft.protocol, &draft.endpoint, &draft.model);
                if !preview.is_empty() {
                    d.text(
                        r(x, *y - 12., width, 15.),
                        format!("预览：{preview}"),
                        10.5,
                        400,
                        d.p.muted,
                    );
                    *y += 19.;
                }
                self.form_field(
                    d,
                    x,
                    y,
                    width,
                    ServiceCustomField::ApiKey,
                    "API Key",
                    true,
                    if draft.provider_id.is_some() {
                        "留空则保持现有 API Key"
                    } else {
                        "sk-..."
                    },
                    measurer,
                );
                let introspection =
                    !draft.endpoint.trim().is_empty() && !draft.api_key.trim().is_empty() && !busy;
                d.text(r(x, *y + 7., width, 18.), "模型", 12.5, 500, d.p.ink);
                d.button(
                    r(x + width - 102.5, *y, 102.5, 32.),
                    ServiceControl::LoadModels,
                    if self.snapshot.loading_models {
                        "获取中..."
                    } else {
                        "获取模型列表"
                    },
                    introspection && !self.snapshot.loading_models,
                    false,
                );
                *y += 36.;
                self.draw_input(
                    d,
                    ServiceField::Custom(ServiceCustomField::Model),
                    r(x, *y, width, 42.),
                    false,
                    !busy,
                    "gpt-4o / claude-3-5-sonnet-latest / gemini-1.5-flash",
                    measurer,
                );
                *y += 58.;
                if let Some(error) = self.snapshot.model_error.clone() {
                    d.notice(x, y, width, &error, true);
                }
                for model in self.snapshot.models.clone() {
                    d.button(
                        r(x, *y, width, 34.),
                        ServiceControl::ChooseModel(model.clone()),
                        &model,
                        !busy,
                        draft.model == model,
                    );
                    *y += 35.;
                }
                d.button(
                    r(x, *y, 77., 32.),
                    ServiceControl::TestConnection,
                    if self.snapshot.testing_connection {
                        "检测中..."
                    } else {
                        "检测连接"
                    },
                    introspection
                        && !draft.model.trim().is_empty()
                        && !self.snapshot.testing_connection,
                    false,
                );
                if let Some(status) = &self.snapshot.test_status {
                    d.text(
                        r(x + 89., *y + 7., width - 89., 18.),
                        status
                            .as_ref()
                            .map(String::as_str)
                            .unwrap_or_else(|e| e.as_str()),
                        12.5,
                        400,
                        if status.is_ok() { d.p.good } else { d.p.danger },
                    );
                }
                *y += 48.;
                d.text(
                    r(x, *y, width, 18.),
                    "Reasoning 强度 (可选)",
                    12.5,
                    500,
                    d.p.ink,
                );
                *y += 22.;
                let mut xx = x;
                for (value, label, bw) in [
                    ("", "默认", 51.5),
                    ("minimal", "Minimal", 72.328125),
                    ("low", "Low", 50.03125),
                    ("medium", "Medium", 73.8125),
                    ("high", "High", 53.875),
                    ("xhigh", "XHigh", 62.53125),
                ] {
                    if xx + bw > x + width {
                        xx = x;
                        *y += 40.;
                    }
                    d.button(
                        r(xx, *y, bw, 32.),
                        ServiceControl::Reasoning(value.into()),
                        label,
                        !busy,
                        draft.reasoning_level == value,
                    );
                    xx += bw + 8.;
                }
                *y += 48.;
                d.text(r(x, *y, width, 18.), "翻译策略", 12.5, 500, d.p.ink);
                *y += 22.;
                xx = x;
                for (id, name) in std::iter::once(("smart".into(), "智能选择".into())).chain(
                    prompts
                        .strategies
                        .iter()
                        .map(|p| (p.id.clone(), p.name.clone())),
                ) {
                    let bw = measurer
                        .measure(&name, 12.5, 500, FontFamily::Sans)
                        .caret_stops
                        .iter()
                        .map(|(_, x)| *x)
                        .fold(0., f32::max)
                        + 27.;
                    if xx + bw > x + width {
                        xx = x;
                        *y += 40.;
                    }
                    d.button(
                        r(xx, *y, bw, 32.),
                        ServiceControl::PromptStrategy(id.clone()),
                        &name,
                        !busy,
                        draft.prompt_strategy_id == id,
                    );
                    xx += bw + 8.;
                }
                *y += 48.;
                let panel = *y;
                let panel_h = if prompts.error.is_some() { 323. } else { 312. };
                d.fill(r(x, panel, width, panel_h), 8., d.p.sunken);
                d.border(r(x, panel, width, panel_h), 8., d.p.line);
                let ix = x + 13.;
                let iw = width - 26.;
                *y += 13.;
                d.text(r(ix, *y, iw, 18.), "策略编辑", 12.5, 500, d.p.ink);
                if prompts
                    .strategies
                    .iter()
                    .any(|p| p.id == draft.prompt_strategy_id && p.deletable)
                {
                    d.text(
                        r(ix + iw - 43., *y + 1.5, 43., 15.),
                        "删除策略",
                        10.5,
                        500,
                        d.p.danger,
                    );
                    d.control(
                        r(ix + iw - 43., *y + 1.5, 43., 15.),
                        ServiceControl::DeletePrompt,
                        ServicesRole::Button,
                        "删除策略",
                        !busy,
                        false,
                        None,
                    );
                }
                *y += 26.;
                self.draw_input(
                    d,
                    ServiceField::Custom(ServiceCustomField::PromptName),
                    r(ix, *y, iw, 42.),
                    false,
                    !busy,
                    "策略名称，例如：法律翻译",
                    measurer,
                );
                *y += 50.;
                self.draw_input(
                    d,
                    ServiceField::Custom(ServiceCustomField::PromptDescription),
                    r(ix, *y, iw, 42.),
                    false,
                    !busy,
                    "适用说明，例如：合同、条款、法律文本",
                    measurer,
                );
                *y += 50.;
                self.draw_input(
                    d,
                    ServiceField::Custom(ServiceCustomField::PromptText),
                    r(ix, *y, iw, 114.),
                    false,
                    !busy,
                    "系统提示词，可使用 {source_lang} 和 {target_lang}",
                    measurer,
                );
                *y += 128.;
                if let Some(error) = &prompts.error {
                    d.notice(ix, y, iw, error, true);
                }
                let selected = prompts
                    .strategies
                    .iter()
                    .any(|p| p.id == draft.prompt_strategy_id);
                d.button(
                    r(ix, *y, 77., 32.),
                    ServiceControl::SavePrompt,
                    "保存策略",
                    !busy && selected,
                    false,
                );
                d.button(
                    r(ix + 85., *y, 128., 32.),
                    ServiceControl::AddPrompt,
                    "新增为自定义策略",
                    !busy,
                    false,
                );
                *y += 69.;
                d.button(
                    r(x, *y, (width - 12.) / 2., 40.),
                    ServiceControl::CancelForm,
                    "取消",
                    !busy,
                    false,
                );
                let valid = !draft.name.trim().is_empty()
                    && !draft.endpoint.trim().is_empty()
                    && !draft.model.trim().is_empty()
                    && (draft.provider_id.is_some() || !draft.api_key.trim().is_empty());
                d.button(
                    r(x + (width + 12.) / 2., *y, (width - 12.) / 2., 40.),
                    ServiceControl::SaveForm,
                    if busy {
                        "保存中..."
                    } else if draft.provider_id.is_some() {
                        "保存配置"
                    } else {
                        "添加"
                    },
                    !busy && valid,
                    true,
                );
                *y += 40.;
            }
        }
    }
    fn form_field(
        &mut self,
        d: &mut Draw,
        x: f32,
        y: &mut f32,
        width: f32,
        field: ServiceCustomField,
        label: &str,
        secret: bool,
        placeholder: &str,
        measurer: &mut impl TextMeasurer,
    ) {
        d.text(r(x, *y, width, 18.), label, 12.5, 500, d.p.ink);
        *y += 22.;
        self.draw_input(
            d,
            ServiceField::Custom(field),
            r(x, *y, width, 42.),
            secret,
            !self.snapshot.busy,
            placeholder,
            measurer,
        );
        *y += 58.;
    }
    fn draw_input(
        &mut self,
        d: &mut Draw,
        field: ServiceField,
        rect: Rect,
        secret: bool,
        enabled: bool,
        placeholder: &str,
        measurer: &mut impl TextMeasurer,
    ) {
        let value = self.field_value(&field);
        let editing = self.editing.as_mut().filter(|editor| editor.field == field);
        let text = editing
            .as_ref()
            .map(|editor| editor.buffer.display_text())
            .unwrap_or(value);
        let multiline = field.multiline();
        let font_size = if matches!(field, ServiceField::Credential { .. }) {
            12.5
        } else {
            16.
        };
        let line_height = if matches!(field, ServiceField::Credential { .. }) {
            18.75
        } else {
            24.
        };
        let inner = r(
            rect.x + 12.,
            rect.y + 9.,
            rect.width - 24.,
            rect.height - 18.,
        );
        let (display, map) = if secret {
            let mut display = String::new();
            let mut map = vec![];
            for (byte, _) in text.grapheme_indices(true) {
                map.push((byte, display.len()));
                display.push('•');
            }
            map.push((text.len(), display.len()));
            (display, Some(map))
        } else {
            (text.clone(), None)
        };
        let mut layout = measurer.line_layout(
            &display,
            if multiline { inner.width } else { 100_000. },
            font_size,
            400,
            FontFamily::Sans,
            line_height,
        );
        if let Some(map) = &map {
            layout.caret_stops = map
                .iter()
                .filter_map(|(byte, masked)| {
                    layout
                        .caret_stops
                        .iter()
                        .find(|(index, _)| index == masked)
                        .map(|(_, point)| (*byte, *point))
                })
                .collect();
        }
        self.input_layouts
            .push((field.clone(), inner, layout.clone()));
        d.fill(rect, 8., d.p.surface);
        d.border(
            rect,
            8.,
            if editing.is_some() {
                d.p.accent
            } else {
                d.p.line_strong
            },
        );
        d.scene.commands.push(DrawCommand::PushClip(r(
            rect.x + 1.,
            rect.y + 1.,
            rect.width - 2.,
            rect.height - 2.,
        )));
        let mut offset = Point::default();
        if let Some(editor) = editing {
            editor.layout = layout.clone();
            editor.rect = inner;
            if let Some(caret) = editor.buffer.display_caret().and_then(|byte| {
                layout
                    .caret_stops
                    .iter()
                    .find(|(index, _)| *index == byte)
                    .map(|(_, point)| *point)
            }) {
                if caret.x < editor.scroll.x {
                    editor.scroll.x = caret.x;
                }
                if caret.x > editor.scroll.x + inner.width - 2. {
                    editor.scroll.x = (caret.x - inner.width + 2.).max(0.);
                }
                if caret.y < editor.scroll.y {
                    editor.scroll.y = caret.y;
                }
                if caret.y + line_height > editor.scroll.y + inner.height {
                    editor.scroll.y = (caret.y + line_height - inner.height).max(0.);
                }
            }
            offset = editor.scroll;
            if let Some(selection) = editor
                .buffer
                .display_selection()
                .filter(|selection| !selection.is_empty())
            {
                for window in layout.caret_stops.windows(2) {
                    let [(start, a), (end, b)] = window else {
                        continue;
                    };
                    if *start >= selection.start && *end <= selection.end {
                        d.fill(
                            r(
                                inner.x + a.x - offset.x,
                                inner.y + a.y - offset.y,
                                if a.y == b.y {
                                    (b.x - a.x).abs().max(1.)
                                } else {
                                    inner.width - a.x
                                },
                                line_height,
                            ),
                            0.,
                            d.p.accent_soft,
                        );
                    }
                }
            }
            if let Some(caret) = editor.buffer.display_caret().and_then(|byte| {
                layout
                    .caret_stops
                    .iter()
                    .find(|(index, _)| *index == byte)
                    .map(|(_, point)| *point)
            }) {
                let caret = r(
                    inner.x + caret.x - offset.x,
                    inner.y + caret.y - offset.y,
                    1.,
                    line_height - 2.,
                );
                d.fill(caret, 0., d.p.ink);
                d.scene.ime_cursor = Some(caret);
            }
            if let Some(range) = editor.buffer.composition_range() {
                for (byte, point) in &layout.caret_stops {
                    if range.contains(byte) {
                        d.fill(
                            r(
                                inner.x + point.x - offset.x,
                                inner.y + point.y + line_height - 1. - offset.y,
                                8.,
                                1.,
                            ),
                            0.,
                            d.p.accent,
                        );
                    }
                }
            }
        }
        d.scene.commands.push(DrawCommand::TextBlock {
            rect: r(
                inner.x - offset.x,
                inner.y - offset.y,
                if multiline { inner.width } else { 100_000. },
                if multiline {
                    layout.height.max(inner.height)
                } else {
                    line_height
                },
            ),
            text: if display.is_empty() {
                placeholder.into()
            } else {
                display.clone()
            },
            size: font_size,
            weight: 400,
            color: if display.is_empty() {
                d.p.faint
            } else {
                d.p.ink
            },
            font: FontFamily::Sans,
            line_height,
        });
        d.scene.commands.push(DrawCommand::PopClip);
        d.control(
            rect,
            ServiceControl::Field(field),
            if secret {
                ServicesRole::SecretInput
            } else if multiline {
                ServicesRole::MultilineInput
            } else {
                ServicesRole::TextInput
            },
            placeholder,
            enabled,
            false,
            Some(display),
        );
    }
}
pub(crate) fn endpoint_preview(protocol: ServiceProtocol, endpoint: &str, model: &str) -> String {
    let endpoint = endpoint.trim().trim_end_matches('/');
    if endpoint.is_empty() {
        return String::new();
    }
    let suffix = match protocol {
        ServiceProtocol::OpenAi => "/v1/chat/completions".into(),
        ServiceProtocol::OpenAiResponses => "/v1/responses".into(),
        ServiceProtocol::Anthropic => "/v1/messages".into(),
        ServiceProtocol::Gemini => format!(
            "/v1beta/models/{}:generateContent",
            if model.trim().is_empty() {
                "{model}"
            } else {
                model.trim().trim_start_matches("models/")
            }
        ),
    };
    let tail_start = endpoint.find(['?', '#']).unwrap_or(endpoint.len());
    let base = endpoint[..tail_start].trim_end_matches('/');
    let tail = &endpoint[tail_start..];
    if protocol == ServiceProtocol::Gemini
        && base.contains("/v1beta/models/")
        && base.ends_with(":generateContent")
    {
        return base.into();
    }
    let path_start = base
        .find("://")
        .and_then(|start| base[start + 3..].find('/').map(|index| start + 3 + index));
    if let Some(path_start) = path_start {
        let path = &base[path_start..];
        for (index, ch) in path.char_indices() {
            if ch == '/' && suffix.starts_with(&path[index..]) {
                return format!("{}{suffix}{tail}", &base[..path_start + index]);
            }
        }
        base.to_owned() + tail
    } else if base.contains("://") {
        format!("{base}{suffix}{tail}")
    } else {
        let mut prefix = None;
        for (index, ch) in base.char_indices() {
            if ch == '/' && suffix.starts_with(&base[index..]) {
                prefix = Some(index);
                break;
            }
        }
        prefix
            .map(|index| format!("{}{suffix}{tail}", &base[..index]))
            .unwrap_or_else(|| endpoint.into())
    }
}

fn action_icon_y(y: f32, height: f32, theme: Theme, route: ServicesRoute, index: usize) -> f32 {
    y + (height - 36.) / 2.
        + if theme == Theme::Native {
            if route == ServicesRoute::Ocr && index > 0 {
                -0.5
            } else {
                -1.5
            }
        } else if route == ServicesRoute::Ocr && index > 0 {
            0.5
        } else {
            0.
        }
}

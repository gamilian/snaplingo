use super::*;

pub(super) fn r(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect {
        x,
        y,
        width: width.max(0.),
        height: height.max(0.),
    }
}
pub(super) struct Paint {
    pub(super) scene: Scene,
    pub(super) focus: Vec<(Target, Rect)>,
    pub(super) clips: Vec<Rect>,
    pub(super) focus_target: Option<Target>,
    pub(super) hover: Option<Target>,
    pub(super) p: Palette,
}
impl Paint {
    pub(super) fn new(
        size: Size,
        p: Palette,
        focus_target: Option<Target>,
        hover: Option<Target>,
    ) -> Self {
        Self {
            scene: Scene {
                size,
                content: Rect::default(),
                ime_cursor: None,
                commands: vec![],
                hits: vec![],
                geometry: vec![],
                content_height: 0.,
                viewport: Rect::default(),
            },
            focus: vec![],
            clips: vec![r(0., 0., size.width, size.height)],
            focus_target,
            hover,
            p,
        }
    }
    pub(super) fn fill(&mut self, rect: Rect, radius: f32, color: Color) {
        self.scene.commands.push(DrawCommand::FillRect {
            rect,
            radius,
            color,
        });
    }
    pub(super) fn stroke(&mut self, rect: Rect, radius: f32, width: f32, color: Color) {
        self.scene.commands.push(DrawCommand::StrokeRect {
            rect,
            radius,
            width,
            color,
        });
    }
    pub(super) fn line(&mut self, x: f32, y: f32, x2: f32, y2: f32, color: Color) {
        self.scene.commands.push(DrawCommand::Line {
            from: Point { x, y },
            to: Point { x: x2, y: y2 },
            width: 1.,
            color,
        });
    }
    pub(super) fn text(
        &mut self,
        rect: Rect,
        text: impl Into<String>,
        size: f32,
        weight: u16,
        color: Color,
        font: FontFamily,
    ) {
        self.scene.commands.push(DrawCommand::Text {
            rect,
            text: text.into(),
            size,
            weight,
            color,
            font,
            align: TextAlign::Left,
        });
    }
    pub(super) fn center(&mut self, rect: Rect, text: impl Into<String>, size: f32, color: Color) {
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
    pub(super) fn clip(&mut self, rect: Rect) {
        let rect = rect.intersect(*self.clips.last().unwrap());
        self.clips.push(rect);
        self.scene.commands.push(DrawCommand::PushClip(rect));
    }
    pub(super) fn unclip(&mut self) {
        self.clips.pop();
        self.scene.commands.push(DrawCommand::PopClip);
    }
    pub(super) fn hit(&mut self, rect: Rect, target: Target, label: &str) {
        self.scene.geometry.push(GeometryNode {
            kind: GeometryKind::Control(target),
            rect,
        });
        self.focus.push((target, rect));
        let visible = rect.intersect(*self.clips.last().unwrap());
        if visible.width > 0. && visible.height > 0. {
            self.scene.hits.push(HitRegion {
                rect: visible,
                target,
                label: label.into(),
            });
        }
        if self.focus_target == Some(target) {
            self.stroke(
                r(rect.x - 3., rect.y - 3., rect.width + 6., rect.height + 6.),
                7.,
                2.,
                self.p.accent,
            );
        }
    }
    pub(super) fn button(&mut self, rect: Rect, target: Target, title: &str, selected: bool) {
        let hovered = self.hover == Some(target);
        self.fill(
            rect,
            6.,
            if selected {
                self.p.accent_soft
            } else if hovered {
                self.p.raised
            } else {
                self.p.surface
            },
        );
        self.stroke(
            rect,
            6.,
            1.,
            if selected { self.p.accent } else { self.p.line },
        );
        self.center(
            r(
                rect.x + 4.,
                rect.y + (rect.height - 17.) / 2.,
                rect.width - 8.,
                20.,
            ),
            title,
            10.5,
            if selected {
                self.p.accent
            } else {
                self.p.muted
            },
        );
        self.hit(rect, target, title);
    }
}
impl SettingsUi {
    pub(super) fn layout(&self) -> (Scene, Vec<(Target, Rect)>, [f32; 6], f32) {
        let size = Size {
            width: self.size.width / self.scale,
            height: self.size.height / self.scale,
        };
        let p = palette(self.snapshot.theme, self.snapshot.dark);
        let mut b = Paint::new(
            size,
            p,
            self.focus.filter(|_| self.keyboard_focus),
            self.hover,
        );
        b.fill(r(0., 0., size.width, size.height), 0., p.canvas);
        let paper = self.snapshot.theme == Theme::Paper;
        let studio = self.snapshot.theme == Theme::Studio;
        let narrow = size.width <= 700.;
        let rail = if paper {
            0.
        } else if studio {
            68.
        } else if narrow {
            65.
        } else {
            202.
        };
        let topbar = if paper { 68. } else { 0. };
        b.scene.content = r(rail, topbar, size.width - rail, size.height - topbar);
        self.navigation(&mut b, rail, topbar, narrow);
        let context = if (studio || paper) && !narrow {
            if size.width <= 900. {
                139.
            } else {
                164.
            }
        } else {
            0.
        };
        let pane_x = rail + context;
        let pane_width = (size.width - pane_x).max(1.);
        let margin = if size.width <= 900. {
            22.
        } else if paper {
            38.
        } else if studio {
            28.
        } else {
            32.
        };
        let page_width = (pane_width - 8.).min(1080.).max(1.);
        let page_left = (pane_width - 8. - page_width) * 0.5;
        let x = pane_x + page_left + margin;
        let width = (page_width - margin * 2.).max(1.);
        let top = if paper {
            33.
        } else if studio {
            29.
        } else {
            30.
        };
        let header_h = self.header_height();
        let sticky_top = (top - self.scroll).max(0.);
        let content_clip = r(
            pane_x,
            topbar + sticky_top + header_h,
            pane_width,
            (size.height - topbar - sticky_top - header_h).max(0.),
        );
        b.scene.viewport = content_clip;
        b.clip(content_clip);
        let mut y = top + header_h + 24.;
        let mut offsets = [0.; 6];
        for (index, section) in self.sections().iter().copied().enumerate() {
            offsets[index] = y;
            let cy = topbar + y - self.scroll;
            let heading_h = if paper {
                65.
            } else if section == Section::Hotkeys {
                if studio {
                    60.
                } else {
                    68.
                }
            } else if studio {
                45.
            } else {
                54.
            };
            let rows = fields(self.page, section, &self.snapshot);
            let mut row_y = cy + heading_h;
            let mut row_geometry = vec![];
            if section == Section::Hotkeys {
                row_y += self.hotkeys().len() as f32 * 59. + 7.;
                if self.page == Page::Screenshot {
                    row_y += crate::settings::screenshot_view::local_height(
                        size.width,
                        self.snapshot.theme,
                    );
                }
            }
            for (row_index, field) in rows.iter().enumerate() {
                let new_group = row_index == 0 || field.group_start();
                if new_group && !paper {
                    row_y += 1.;
                }
                let divider = if new_group { 0. } else { 1. };
                let choice = !field.choices().is_empty();
                let description = field.description();
                let padding: f32 = if paper {
                    16.
                } else if studio {
                    9.
                } else {
                    13.
                };
                let min_h: f32 = if paper {
                    67.
                } else if studio {
                    53.
                } else {
                    62.
                };
                let control_h: f32 = if choice {
                    42.
                } else if matches!(field,Field::General(g)if g.range().is_some()) {
                    40.
                } else if field.text_input() {
                    36.
                } else if !field.colors().is_empty() {
                    36.
                } else {
                    20.
                };
                let label_h: f32 = if description { 37. } else { 18. };
                let h = if *field == Field::General(GeneralField::InterfaceTheme) {
                    crate::settings::general_view::theme_height(
                        width - if paper { 0. } else { 46. },
                        narrow,
                        size.width <= 900.,
                    )
                } else if narrow {
                    padding * 2.
                        + label_h
                        + 10.
                        + if choice {
                            42.
                        } else if field.range().is_some() {
                            if self.page == Page::General {
                                40.
                            } else {
                                24.
                            }
                        } else if self.page == Page::General && field.text_input() {
                            36.
                        } else if self.page == Page::Screenshot
                            && (field.text_input() || !field.colors().is_empty())
                        {
                            36.
                        } else {
                            28.
                        }
                        + divider
                } else {
                    (padding * 2.
                        + if description {
                            if self.page == Page::Screenshot {
                                control_h.max(37.)
                            } else {
                                37.
                            }
                        } else {
                            control_h
                        }
                        + divider)
                        .max(min_h)
                };
                row_geometry.push((*field, row_y, h, padding, control_h, label_h, divider));
                row_y += h;
            }
            let inner = if paper { 0. } else { 23. };
            let extra_y = row_y;
            if self.page == Page::General {
                row_y += self.general_extra_height(section, width - if paper { 0. } else { 46. });
            }
            let height = row_y - cy + if paper { 0. } else { 1. };
            b.scene.geometry.push(GeometryNode {
                kind: GeometryKind::Card(section),
                rect: r(x, cy, width, height),
            });
            if !paper {
                b.fill(
                    r(x, cy, width, height),
                    if studio { 7. } else { 12. },
                    p.surface,
                );
                b.stroke(
                    r(x, cy, width, height),
                    if studio { 7. } else { 12. },
                    1.,
                    p.line,
                );
            }
            b.line(
                x,
                cy + heading_h - 1.,
                x + width,
                cy + heading_h - 1.,
                p.line,
            );
            if studio || paper {
                b.text(
                    r(x + inner, cy + if paper { 25. } else { 16. }, 20., 18.),
                    format!("{:02}", index + 1),
                    10.,
                    400,
                    p.faint,
                    FontFamily::Mono,
                );
            }
            b.text(
                r(
                    x + inner + if paper || studio { 24. } else { 0. },
                    cy + if index == 0 && !paper {
                        (heading_h - 20.) / 2.
                    } else if studio {
                        13.
                    } else {
                        17.
                    },
                    width - inner * 2. - 24.,
                    if paper { 32. } else { 22. },
                ),
                section.label_for(self.page),
                if paper {
                    23.
                } else if studio {
                    12.
                } else {
                    13.
                },
                if paper { 400 } else { 600 },
                p.ink,
                if paper {
                    FontFamily::Serif
                } else {
                    FontFamily::Sans
                },
            );
            if section == Section::Hotkeys {
                let tx = (x + width - inner - 156.).max(x + inner + 80.);
                let restore = r(tx, cy + (heading_h - 32.) / 2., 80., 32.);
                b.center(
                    r(restore.x + 4., restore.y + 7.5, restore.width - 8., 20.),
                    "恢复默认值",
                    10.5,
                    p.muted,
                );
                b.hit(restore, Target::ResetHotkeys, "恢复默认值");
                b.button(
                    r(tx + 88., cy + (heading_h - 32.) / 2., 68., 32.),
                    Target::CheckConflicts,
                    "检测冲突",
                    false,
                );
                self.hotkey_rows(&mut b, x, cy + heading_h, width, inner);
                if self.page == Page::Screenshot {
                    self.paint_local_shortcuts(
                        &mut b,
                        x,
                        cy + heading_h + self.hotkeys().len() as f32 * 59. + 7.,
                        width,
                        inner,
                        narrow,
                    );
                }
            }
            for (i, (field, ry, h, padding, control_h, label_h, divider)) in
                row_geometry.into_iter().enumerate()
            {
                if i > 0 {
                    b.line(x + inner, ry, x + width - inner, ry, p.line);
                }
                let content_width = width - inner * 2.;
                if field == Field::General(GeneralField::InterfaceTheme) {
                    self.paint_theme_picker(&mut b, r(x + inner, ry, content_width, h));
                    continue;
                }
                let description = field.description();
                b.text(
                    r(
                        x + inner,
                        ry + if narrow || description {
                            padding + divider
                        } else {
                            (h - 20.) / 2.
                        },
                        if narrow {
                            content_width
                        } else {
                            (content_width - 254.).max(40.)
                        },
                        20.,
                    ),
                    field.label(),
                    12.5,
                    600,
                    p.ink,
                    FontFamily::Sans,
                );
                if description {
                    b.text(
                        r(x + inner, ry + padding + divider + 21., content_width, 18.),
                        field.description_text().unwrap_or(""),
                        10.5,
                        400,
                        p.muted,
                        FontFamily::Sans,
                    );
                }
                let control_y = ry
                    + if narrow {
                        padding + divider + label_h + 10.
                    } else {
                        divider + (h - divider - control_h) / 2.
                    };
                if let Field::General(general) = field {
                    if general.text_input() {
                        let rect = r(
                            if narrow {
                                x + inner
                            } else {
                                x + width - inner - 230.
                            },
                            control_y,
                            230.0_f32.min(content_width),
                            if general.range().is_some() { 40. } else { 36. },
                        );
                        self.paint_general_edit(&mut b, general, rect);
                        continue;
                    }
                }
                if let Field::Screenshot(screenshot) = field {
                    if field.text_input() {
                        let (tw, tx) = if screenshot == ScreenshotField::SavePath {
                            (260., x + width - inner - 315.46875)
                        } else if screenshot == ScreenshotField::LongshotMaxHeight {
                            (140., x + width - inner - 159.84375)
                        } else {
                            (230., x + width - inner - 230.)
                        };
                        let rect = r(if narrow { x + inner } else { tx }, control_y, tw, 36.);
                        self.paint_edit(&mut b, screenshot, rect);
                        if screenshot == ScreenshotField::SavePath {
                            b.button(
                                r(rect.x + 268., rect.y, 47.46875, 36.),
                                Target::BrowseScreenshotDirectory,
                                "浏览",
                                false,
                            );
                        }
                        if screenshot == ScreenshotField::LongshotMaxHeight {
                            b.text(
                                r(rect.x + 148., rect.y + 9., 20., 20.),
                                "px",
                                12.,
                                400,
                                p.muted,
                                FontFamily::Sans,
                            );
                        }
                        continue;
                    }
                    if !field.colors().is_empty() {
                        self.paint_colors(
                            &mut b,
                            screenshot,
                            r(x + inner, control_y, width - inner * 2., 36.),
                            narrow,
                        );
                        continue;
                    }
                }
                match field.value(&self.snapshot) {
                    Value::Toggle(on) => {
                        let rect = r(
                            if narrow {
                                x + inner
                            } else {
                                x + width - inner - 36.
                            },
                            control_y + if narrow { 8. } else { 4. },
                            36.,
                            20.,
                        );
                        b.fill(rect, 10., if on { p.accent } else { p.line_strong });
                        b.fill(
                            r(rect.x + if on { 18. } else { 2. }, rect.y + 2., 16., 16.),
                            8.,
                            p.surface,
                        );
                        b.hit(rect, field.target(), field.label());
                    }
                    Value::Number(value) => {
                        let (min, max, _, suffix) = field.range().unwrap();
                        let range_width = if self.page == Page::Screenshot {
                            250.
                        } else {
                            270.
                        };
                        let rx = if narrow {
                            x + inner
                        } else {
                            x + width - inner - range_width
                        };
                        let track_width = range_width
                            - if self.page == Page::Screenshot {
                                50.
                            } else {
                                60.
                            };
                        let rect = r(
                            rx,
                            control_y + if narrow { 5. } else { 3. },
                            track_width,
                            20.,
                        );
                        let track = r(rect.x, rect.y + 7., rect.width, 6.);
                        b.fill(track, 3., p.line);
                        let fraction = (value - min) as f32 / (max - min) as f32;
                        let knob = rect.x + 9. + (rect.width - 18.) * fraction;
                        b.fill(r(track.x, track.y, knob - track.x, 6.), 3., p.accent);
                        b.fill(r(knob - 9., rect.y + 1., 18., 18.), 9., [255; 4]);
                        b.stroke(r(knob - 8., rect.y + 2., 16., 16.), 8., 2., p.accent);
                        b.scene.commands.push(DrawCommand::Text {
                            rect: r(
                                rx + track_width + 12.,
                                control_y,
                                if self.page == Page::Screenshot {
                                    38.
                                } else {
                                    48.
                                },
                                20.,
                            ),
                            text: format!("{value}{suffix}"),
                            size: 10.5,
                            weight: 600,
                            color: p.accent,
                            font: FontFamily::Sans,
                            align: TextAlign::Right,
                        });
                        b.hit(rect, field.target(), field.label());
                        b.scene.geometry.last_mut().unwrap().rect = track;
                    }
                    Value::Text(value) => {
                        let rect = r(
                            if narrow {
                                x + inner
                            } else {
                                x + width - inner - 230.
                            },
                            control_y,
                            230.0_f32.min(content_width),
                            42.,
                        );
                        b.fill(rect, if paper { 4. } else { 8. }, p.surface);
                        b.stroke(rect, if paper { 4. } else { 8. }, 1., p.line);
                        let title = field
                            .choices()
                            .iter()
                            .find(|(id, _)| *id == value)
                            .map(|(_, label)| *label)
                            .unwrap_or(&value);
                        b.text(
                            r(rect.x + 16., rect.y + 11., rect.width - 40., 20.),
                            title,
                            12.,
                            400,
                            p.ink,
                            FontFamily::Sans,
                        );
                        b.line(
                            rect.x + rect.width - 23.,
                            rect.y + 19.,
                            rect.x + rect.width - 19.,
                            rect.y + 23.,
                            p.faint,
                        );
                        b.line(
                            rect.x + rect.width - 19.,
                            rect.y + 23.,
                            rect.x + rect.width - 15.,
                            rect.y + 19.,
                            p.faint,
                        );
                        b.hit(rect, field.target(), field.label());
                    }
                }
            }
            if self.page == Page::General {
                self.paint_general_extra(
                    &mut b,
                    section,
                    r(x + inner, extra_y, width - inner * 2., row_y - extra_y),
                    narrow,
                );
            }
            y += height
                + if paper {
                    25.
                } else if studio {
                    15.
                } else {
                    20.
                };
        }
        b.unclip();
        let content_h = y - if paper {
            25.
        } else if studio {
            15.
        } else {
            20.
        } + if paper {
            64.
        } else if studio {
            48.
        } else {
            56.
        };
        let max_scroll = (content_h - (size.height - topbar)).max(0.);
        let max_scroll = if self.page == Page::General {
            max_scroll.ceil()
        } else {
            max_scroll
        };
        b.scene.content_height = content_h;
        if context > 0. {
            let dx = rail;
            b.fill(r(dx, topbar, context, size.height - topbar), 0., p.canvas);
            if studio {
                b.line(
                    dx + context - 1.,
                    topbar,
                    dx + context - 1.,
                    size.height,
                    p.line,
                );
            }
            b.text(
                r(
                    dx + 18.,
                    topbar + if paper { 35. } else { 28. },
                    context - 31.,
                    if paper { 33. } else { 25. },
                ),
                self.page.label(),
                if paper { 22. } else { 15. },
                if paper { 400 } else { 600 },
                p.ink,
                if paper {
                    FontFamily::Serif
                } else {
                    FontFamily::Sans
                },
            );
            b.text(
                r(
                    dx + 18.,
                    topbar + if paper { 77. } else { 59. },
                    context - 31.,
                    14.,
                ),
                "CONTENTS",
                9.,
                400,
                p.faint,
                FontFamily::Mono,
            );
            for (i, section) in self.sections().iter().copied().enumerate() {
                let rect = r(
                    dx + 9.,
                    topbar + if paper { 110. } else { 92. } + i as f32 * 39.,
                    context - 18.,
                    34.,
                );
                if self.section == section {
                    b.fill(rect, 5., p.accent_soft);
                }
                b.text(
                    r(rect.x + 9., rect.y + 9., 20., 18.),
                    format!("{:02}", i + 1),
                    10.,
                    400,
                    p.faint,
                    FontFamily::Mono,
                );
                b.text(
                    r(rect.x + 32., rect.y + 7., rect.width - 39., 20.),
                    section.label_for(self.page),
                    11.,
                    500,
                    if self.section == section {
                        p.accent
                    } else {
                        p.muted
                    },
                    FontFamily::Sans,
                );
                b.hit(rect, Target::Section(section), section.label_for(self.page));
            }
            b.fill(r(dx + 18., size.height - 32., 5., 5.), 2.5, p.good);
            b.text(
                r(dx + 30., size.height - 37., context - 40., 18.),
                "自动保存",
                10.,
                400,
                p.muted,
                FontFamily::Sans,
            );
        }
        let header_y = topbar + sticky_top;
        b.fill(
            r(pane_x, topbar, pane_width, sticky_top + header_h),
            0.,
            p.canvas,
        );
        let title_size = if paper {
            37.
        } else if studio {
            25.
        } else {
            28.
        };
        b.text(
            r(x - 2., header_y + if studio { 16. } else { 1. }, 100., 42.),
            self.page.label(),
            title_size,
            if paper { 400 } else { 600 },
            p.ink,
            if paper {
                FontFamily::Serif
            } else {
                FontFamily::Sans
            },
        );
        b.fill(
            r(x + 78., header_y + if studio { 27. } else { 11. }, 12., 12.),
            6.,
            p.good_soft,
        );
        b.fill(
            r(x + 81., header_y + if studio { 30. } else { 14. }, 6., 6.),
            3.,
            p.good,
        );
        b.text(
            r(
                x + 97.,
                header_y + if studio { 26. } else { 10. },
                (width - 97.).max(0.),
                22.,
            ),
            self.snapshot.error.as_deref().unwrap_or("自动保存"),
            10.5,
            500,
            if self.snapshot.error.is_some() {
                p.danger
            } else {
                p.muted
            },
            FontFamily::Sans,
        );
        b.line(
            x,
            header_y + header_h - 1.,
            x + width,
            header_y + header_h - 1.,
            p.line,
        );
        if context == 0. {
            let tab_width = |section: Section| match section {
                Section::Hotkeys => 58.25,
                Section::Behavior => 71.,
                Section::Window if self.page == Page::Translation => 96.5,
                _ => 83.75,
            };
            let total = self.sections().iter().map(|s| tab_width(*s)).sum::<f32>()
                + (self.sections().len() - 1) as f32 * 2.;
            let mut tx = if studio || paper {
                (x + width - total - 3.).max(x)
            } else {
                x
            };
            if studio || paper {
                b.fill(r(tx - 3., header_y + 15., total + 6., 31.), 7., p.raised);
            }
            let ty = header_y + if studio || paper { 18. } else { 52. };
            for section in self.sections().iter().copied() {
                let tw = tab_width(section);
                let rect = r(tx, ty, tw, if studio || paper { 25. } else { 34. });
                if self.section == section {
                    if studio || paper {
                        b.fill(rect, 5., p.surface);
                    } else {
                        b.fill(r(rect.x, rect.y + 32., tw, 2.), 0., p.accent);
                    }
                }
                b.center(
                    r(
                        rect.x,
                        rect.y + if studio || paper { 3. } else { 7. },
                        tw,
                        20.,
                    ),
                    section.label_for(self.page),
                    12.5,
                    if self.section == section {
                        p.accent
                    } else {
                        p.muted
                    },
                );
                b.hit(rect, Target::Section(section), section.label_for(self.page));
                tx += tw + 2.;
            }
        }
        if max_scroll > 0. {
            let track = (size.height - topbar - header_h).max(1.);
            let thumb = (track * (track / content_h)).max(30.).min(track);
            let progress = (self.scroll / max_scroll).clamp(0., 1.);
            b.fill(
                r(
                    size.width - 6.,
                    topbar + header_h + (track - thumb) * progress,
                    3.,
                    thumb,
                ),
                1.5,
                p.line_strong,
            );
        }
        if let Some(menu) = self.menu {
            self.dropdown(&mut b, menu);
        }
        (b.scene, b.focus, offsets, max_scroll)
    }
    pub(super) fn hotkey_rows(&self, b: &mut Paint, x: f32, y: f32, width: f32, inner: f32) {
        let p = b.p;
        let labels: &[&str] = if self.page == Page::Screenshot {
            &[
                "截屏",
                "截屏并自动复制",
                "贴图",
                "隐藏 / 显示所有贴图",
                "切换到另一贴图组",
            ]
        } else if self.page == Page::Translation {
            &["划词翻译", "截图翻译", "显示翻译窗口"]
        } else {
            &["截图 OCR", "静默截图 OCR", "上传图片 OCR"]
        };
        for (index, &label) in labels.iter().enumerate() {
            let ry = y + index as f32 * 59.;
            if index > 0 {
                b.line(x + inner, ry - 1., x + width - inner, ry - 1., p.line);
            }
            let recording = self.recording.as_ref().is_some_and(|r| r.index == index);
            let hotkey = &self.hotkeys()[index];
            let record_width = 156.;
            let rx = x + width - inner - 28. * 2. - 4. - 12. - record_width;
            b.text(
                r(x + inner, ry + 19., (rx - x - inner - 16.).max(0.), 22.),
                label,
                13.,
                600,
                p.ink,
                FontFamily::Sans,
            );
            let rect = r(rx, ry + 11., record_width, 36.);
            hotkey_display(b, rect, self.snapshot.key_style, &hotkey.value, recording);
            b.hit(rect, Target::Hotkey(index), label);
            for (reset, enabled) in [
                (true, hotkey.value != hotkey.default_value),
                (false, hotkey.value != "未设置"),
            ] {
                let rect = r(
                    rx + record_width + if reset { 12. } else { 44. },
                    ry + 15.,
                    28.,
                    28.,
                );
                let target = if reset {
                    Target::ResetHotkey(index)
                } else {
                    Target::ClearHotkey(index)
                };
                let color = if enabled && self.hover == Some(target) {
                    p.accent
                } else {
                    p.faint
                };
                if enabled && self.hover == Some(target) {
                    b.fill(rect, 5., p.accent_soft);
                }
                let (ix, iy) = (rect.x, rect.y);
                if reset {
                    for path in [
                        &[(19.3, 13.3), (18.5, 10.), (14., 8.), (10.3, 9.), (8.7, 12.)][..],
                        &[(8.7, 8.7), (8.7, 12.), (12., 12.)],
                        &[
                            (8.7, 14.7),
                            (9.5, 18.),
                            (14., 20.),
                            (17.7, 19.),
                            (19.3, 16.),
                        ],
                        &[(19.3, 19.3), (19.3, 16.), (16., 16.)],
                    ] {
                        for pair in path.windows(2) {
                            b.line(
                                ix + pair[0].0,
                                iy + pair[0].1,
                                ix + pair[1].0,
                                iy + pair[1].1,
                                color,
                            );
                        }
                    }
                } else {
                    b.line(ix + 10., iy + 10., ix + 18., iy + 18., color);
                    b.line(ix + 18., iy + 10., ix + 10., iy + 18., color);
                }
                if enabled {
                    b.hit(
                        rect,
                        target,
                        if reset {
                            "恢复默认值"
                        } else {
                            "清除快捷键"
                        },
                    );
                } else {
                    b.scene.geometry.push(GeometryNode {
                        kind: GeometryKind::Control(target),
                        rect,
                    });
                }
            }
        }
    }
    pub(super) fn dropdown(&self, b: &mut Paint, menu: Menu) {
        let Some((_, anchor)) = b
            .focus
            .iter()
            .find(|(target, _)| *target == menu.field.target())
            .copied()
        else {
            return;
        };
        let p = b.p;
        let count = menu.field.choices().len();
        let height = (count as f32 * 34. + 8.).min((b.scene.size.height - 16.).max(0.));
        let below = anchor.y + anchor.height + 5.;
        let y = if below + height <= b.scene.size.height - 8. {
            below
        } else {
            (anchor.y - height - 5.).max(8.)
        };
        let rect = r(
            anchor
                .x
                .clamp(8., (b.scene.size.width - anchor.width - 8.).max(8.)),
            y,
            anchor.width,
            height,
        );
        b.fill(
            r(rect.x + 2., rect.y + 3., rect.width, rect.height),
            8.,
            [0, 0, 0, 25],
        );
        b.fill(rect, 8., p.surface);
        b.stroke(rect, 8., 1., p.line);
        b.clip(rect);
        let scroll = menu.scroll.min((count as f32 * 34. + 8. - height).max(0.));
        for (index, (_, title)) in menu.field.choices().iter().enumerate() {
            let option = r(
                rect.x + 4.,
                rect.y + 4. + index as f32 * 34. - scroll,
                rect.width - 8.,
                34.,
            );
            if index == menu.selected || self.hover == Some(menu.field.option(index)) {
                b.fill(option, 5., p.accent_soft);
            }
            b.text(
                r(option.x + 10., option.y + 8., option.width - 20., 20.),
                *title,
                12.,
                400,
                if index == menu.selected {
                    p.accent
                } else {
                    p.ink
                },
                FontFamily::Sans,
            );
            b.hit(option, menu.field.option(index), title);
        }
        b.unclip();
    }
    pub(super) fn navigation(&self, b: &mut Paint, rail: f32, topbar: f32, narrow: bool) {
        let p = b.p;
        let size = b.scene.size;
        let paper = topbar > 0.;
        let studio = self.snapshot.theme == Theme::Studio;
        b.fill(
            if paper {
                r(0., 0., size.width, topbar)
            } else {
                r(0., 0., rail, size.height)
            },
            0.,
            if paper { p.surface } else { p.sidebar },
        );
        if paper {
            b.line(0., topbar - 1., size.width, topbar - 1., p.line);
        } else {
            b.line(rail - 1., 0., rail - 1., size.height, p.line);
        }
        let bx = if paper {
            if size.width <= 900. {
                17.
            } else {
                25.
            }
        } else if studio || narrow {
            (rail - 28.) / 2.
        } else {
            18.
        };
        let by = if paper {
            20.
        } else if studio {
            20.
        } else if narrow {
            17.
        } else {
            23.
        };
        b.fill(r(bx, by, 28., 28.), 6., p.accent);
        b.center(r(bx, by + 4., 28., 22.), "S", 15., p.accent_on);
        if paper || (!studio && !narrow) {
            b.text(
                r(bx + 38., by + 2., 120., 26.),
                "SnapLingo",
                if paper { 19. } else { 17. },
                if paper { 500 } else { 700 },
                p.ink,
                if paper {
                    FontFamily::Serif
                } else {
                    FontFamily::Sans
                },
            );
        }
        let paper_compact = paper && size.width <= 900.;
        let paper_widths = if paper_compact {
            [33.; 7]
        } else {
            [62., 62., 62., 61., 62., 73., 84.]
        };
        let total = paper_widths.iter().sum::<f32>() + 6. * 5. + 17.;
        let paper_x =
            (size.width - 25. - 32. - 17. - total).max(if paper_compact { 190. } else { 205. });
        if paper {
            b.clip(r(190., 0., (size.width - 239.).max(0.), topbar));
        }
        for (index, page) in Page::ALL.into_iter().enumerate() {
            let library = index >= 5;
            let rect = if paper {
                r(
                    paper_x
                        + paper_widths[..index].iter().sum::<f32>()
                        + index as f32 * 5.
                        + if library { 17. } else { 0. },
                    17.5,
                    paper_widths[index],
                    33.,
                )
            } else if studio {
                r(
                    7.,
                    80. + index as f32 * 52. + if library { 29. } else { 0. },
                    54.,
                    51.,
                )
            } else {
                r(
                    if narrow { 7. } else { 12. },
                    83. + index as f32 * 42. + if library { 37. } else { 0. },
                    rail - if narrow { 14. } else { 24. },
                    41.,
                )
            };
            let selected = page == self.page;
            let target = Target::Navigate(page);
            if selected || self.hover == Some(target) {
                b.fill(
                    rect,
                    if studio {
                        6.
                    } else if paper {
                        4.
                    } else {
                        8.
                    },
                    if selected { p.accent_soft } else { p.raised },
                );
            }
            if selected {
                b.stroke(
                    rect,
                    if studio {
                        6.
                    } else if paper {
                        4.
                    } else {
                        8.
                    },
                    1.,
                    p.line,
                );
            }
            let icon = if studio {
                r(rect.x + 18., rect.y + 5., 18., 18.)
            } else if narrow || paper_compact {
                r(
                    rect.x + (rect.width - 18.) / 2.,
                    rect.y + (rect.height - 18.) / 2.,
                    18.,
                    18.,
                )
            } else {
                r(
                    rect.x + if paper { 9. } else { 11. },
                    rect.y + (rect.height - 18.) / 2.,
                    18.,
                    18.,
                )
            };
            nav_icon(b, page, icon, if selected { p.accent } else { p.faint });
            if studio {
                b.center(
                    r(rect.x, rect.y + 29., rect.width, 15.),
                    page.label(),
                    9.,
                    if selected { p.accent } else { p.muted },
                );
            } else if (!narrow && !paper) || paper && !paper_compact {
                b.text(
                    r(
                        icon.x + if paper { 22. } else { 28. },
                        rect.y + if paper { 9. } else { 12. },
                        rect.width - 35.,
                        20.,
                    ),
                    page.label(),
                    if paper { 11. } else { 12. },
                    if selected { 600 } else { 400 },
                    if selected { p.accent } else { p.muted },
                    FontFamily::Sans,
                );
            }
            b.hit(rect, target, page.label());
        }
        if paper {
            b.unclip();
        }
        if !paper && !studio && !narrow {
            b.text(
                r(20., 83. + 5. * 42. + 17., 150., 16.),
                "资料库",
                10.,
                600,
                p.faint,
                FontFamily::Sans,
            );
        }
        if studio {
            b.line(7., 83. + 5. * 52. + 9., 61., 83. + 5. * 52. + 9., p.line);
        }
        if paper || studio || narrow && !self.embedded {
            let rect = if paper {
                r(size.width - 49., 18., 32., 32.)
            } else {
                r((rail - 32.) / 2., size.height - 44., 32., 32.)
            };
            let x = rect.x + 7.;
            let y = rect.y + 6.;
            for offset in [0., 5., 10.] {
                b.line(x, y + offset + 5., x + 9., y + offset + 10., p.muted);
                b.line(x + 9., y + offset + 10., x + 18., y + offset + 5., p.muted);
            }
            b.hit(rect, Target::Capacity, "历史与收藏");
        } else if !narrow {
            let rect = r(12., size.height - 141., rail - 24., 127.);
            b.fill(rect, 9., p.surface);
            b.stroke(rect, 9., 1., p.line);
            b.text(
                r(rect.x + 10., rect.y + 10., 100., 19.),
                "历史与收藏",
                10.5,
                600,
                p.ink,
                FontFamily::Sans,
            );
            b.button(
                r(rect.x + rect.width - 43., rect.y + 8., 33., 20.),
                Target::Capacity,
                "设置",
                false,
            );
            b.line(
                rect.x + 10.,
                rect.y + 37.,
                rect.x + rect.width - 10.,
                rect.y + 37.,
                p.line,
            );
            let c = &self.snapshot.capacity;
            for (i, (label, value)) in [
                ("保留天数", format!("{} 天", c.retention_days)),
                (
                    "历史记录",
                    format!(
                        "{} / {}",
                        c.history_used.map(|n| n.to_string()).unwrap_or("—".into()),
                        c.history_limit
                    ),
                ),
                (
                    "收藏内容",
                    format!(
                        "{} / {}",
                        c.favorites_used
                            .map(|n| n.to_string())
                            .unwrap_or("—".into()),
                        c.favorites_limit
                    ),
                ),
            ]
            .into_iter()
            .enumerate()
            {
                let y = rect.y + 45. + i as f32 * 22.;
                b.text(
                    r(rect.x + 10., y, 70., 17.),
                    label,
                    10.5,
                    400,
                    p.muted,
                    FontFamily::Sans,
                );
                b.scene.commands.push(DrawCommand::Text {
                    rect: r(rect.x + 73., y, rect.width - 83., 17.),
                    text: value,
                    size: 10.5,
                    weight: 500,
                    color: p.ink,
                    font: FontFamily::Sans,
                    align: TextAlign::Right,
                });
            }
        }
    }
}
pub(super) fn nav_icon(b: &mut Paint, page: Page, rect: Rect, color: Color) {
    let point = |x: f32, y: f32| Point {
        x: rect.x + x * rect.width / 24.,
        y: rect.y + y * rect.height / 24.,
    };
    let line = |b: &mut Paint, points: &[(f32, f32)]| {
        for pair in points.windows(2) {
            b.scene.commands.push(DrawCommand::Line {
                from: point(pair[0].0, pair[0].1),
                to: point(pair[1].0, pair[1].1),
                width: 1.5,
                color,
            });
        }
    };
    let circle = |b: &mut Paint, cx: f32, cy: f32, radius: f32| {
        let p = point(cx - radius, cy - radius);
        b.stroke(
            r(
                p.x,
                p.y,
                radius * rect.width / 12.,
                radius * rect.height / 12.,
            ),
            radius * rect.width / 24.,
            1.5,
            color,
        );
    };
    match page {
        Page::General => {
            for (y, cx) in [(6., 8.), (12., 16.), (18., 12.)] {
                line(b, &[(4., y), (20., y)]);
                circle(b, cx, y, 2.);
            }
        }
        Page::Screenshot => {
            line(
                b,
                &[
                    (1., 8.),
                    (1., 19.),
                    (3., 21.),
                    (21., 21.),
                    (23., 19.),
                    (23., 8.),
                    (21., 6.),
                    (17., 6.),
                    (15., 3.),
                    (9., 3.),
                    (7., 6.),
                    (3., 6.),
                    (1., 8.),
                ],
            );
            circle(b, 12., 13., 4.);
        }
        Page::Translation => {
            circle(b, 12., 12., 10.);
            line(b, &[(2., 12.), (22., 12.)]);
            line(
                b,
                &[
                    (12., 2.),
                    (16., 7.),
                    (16., 17.),
                    (12., 22.),
                    (8., 17.),
                    (8., 7.),
                    (12., 2.),
                ],
            );
        }
        Page::Ocr => {
            for path in [
                &[(3., 7.), (3., 5.), (5., 3.), (7., 3.)][..],
                &[(17., 3.), (19., 3.), (21., 5.), (21., 7.)],
                &[(21., 17.), (21., 19.), (19., 21.), (17., 21.)],
                &[(7., 21.), (5., 21.), (3., 19.), (3., 17.)],
                &[(8., 12.), (16., 12.)],
                &[(8., 16.), (13., 16.)],
            ] {
                line(b, path);
            }
        }
        Page::Services => {
            for (x, y, w, h) in [(8., 2., 8., 4.), (6., 8., 12., 8.), (4., 18., 16., 4.)] {
                let p = point(x, y);
                b.stroke(
                    r(p.x, p.y, w * rect.width / 24., h * rect.height / 24.),
                    1.,
                    1.5,
                    color,
                );
            }
        }
        Page::Favorites => line(
            b,
            &[
                (12., 3.),
                (14.8, 8.7),
                (21., 9.6),
                (16.5, 14.),
                (17.6, 20.2),
                (12., 17.3),
                (6.4, 20.2),
                (7.5, 14.),
                (3., 9.6),
                (9.2, 8.7),
                (12., 3.),
            ],
        ),
        Page::History => {
            circle(b, 12., 12., 9.);
            line(b, &[(12., 7.), (12., 12.), (15., 14.)]);
        }
    }
}

pub(super) fn hotkey_display(
    b: &mut Paint,
    rect: Rect,
    style: KeyStyle,
    value: &str,
    recording: bool,
) {
    let p = b.p;
    let unset = value == "未设置";
    b.fill(rect, 8., p.surface);
    if recording {
        b.stroke(
            r(rect.x - 2., rect.y - 2., rect.width + 4., rect.height + 4.),
            10.,
            2.,
            p.accent_soft,
        );
    }
    if unset && !recording {
        // Match the previous recorder's dashed unset border with display-list
        // line segments, keeping the renderer's small primitive set unchanged.
        let mut x = rect.x + 10.;
        while x < rect.x + rect.width - 10. {
            let end = (x + 6.).min(rect.x + rect.width - 10.);
            b.line(x, rect.y, end, rect.y, p.line_strong);
            b.line(
                x,
                rect.y + rect.height,
                end,
                rect.y + rect.height,
                p.line_strong,
            );
            x += 12.;
        }
        for x in [rect.x, rect.x + rect.width] {
            b.line(x, rect.y + 10., x, rect.y + 16., p.line_strong);
            b.line(x, rect.y + 22., x, rect.y + 26., p.line_strong);
        }
        for (x, sign) in [(rect.x, 1.), (rect.x + rect.width, -1.)] {
            b.line(
                x + 2. * sign,
                rect.y + 4.,
                x + 5. * sign,
                rect.y + 1.,
                p.line_strong,
            );
            b.line(
                x + 2. * sign,
                rect.y + rect.height - 4.,
                x + 5. * sign,
                rect.y + rect.height - 1.,
                p.line_strong,
            );
        }
    } else {
        b.stroke(
            rect,
            8.,
            if unset { 2. } else { 1. },
            if recording {
                p.accent
            } else if unset {
                p.line_strong
            } else {
                p.line
            },
        );
    }
    let main = value.trim_start_matches(['⇧', '⌥', '⌘', '⌃', '⊞']);
    if style == KeyStyle::Windows {
        if recording || unset {
            b.center(
                r(rect.x + 6., rect.y + 10., rect.width - 12., 20.),
                if recording {
                    "请按下快捷键  Esc 取消"
                } else {
                    "未设置"
                },
                10.5,
                p.muted,
            );
            return;
        }
        let parts = [("⊞", "Win"), ("⌘", "Ctrl"), ("⌥", "Alt"), ("⇧", "Shift")];
        let mut keys = parts
            .iter()
            .filter(|(glyph, _)| value.contains(glyph))
            .map(|(_, label)| (*label).to_string())
            .collect::<Vec<_>>();
        if value.contains('⌃') && !value.contains('⌘') {
            keys.insert(0, "Ctrl".into());
        }
        keys.push(main.into());
        let widths = keys
            .iter()
            .enumerate()
            .map(|(i, key)| key.len() as f32 * if i + 1 == keys.len() { 8. } else { 6. })
            .collect::<Vec<_>>();
        let total = widths.iter().sum::<f32>() + (keys.len() - 1) as f32 * 12.;
        let mut x = rect.x + (rect.width - total) / 2.;
        for (i, (key, width)) in keys.iter().zip(widths).enumerate() {
            if i > 0 {
                b.text(
                    r(x - 10., rect.y + 10., 8., 20.),
                    "+",
                    10.,
                    400,
                    p.faint,
                    FontFamily::Sans,
                );
            }
            b.text(
                r(x, rect.y + 9., width + 2., 20.),
                key,
                if i + 1 == keys.len() { 14. } else { 11. },
                if i + 1 == keys.len() { 600 } else { 500 },
                if i + 1 == keys.len() { p.accent } else { p.ink },
                FontFamily::Sans,
            );
            x += width + 12.;
        }
    } else {
        let text = if recording {
            "按下快捷键..."
        } else if unset {
            "未设置"
        } else {
            main
        };
        let text_width = if recording {
            64.
        } else if unset {
            33.
        } else {
            main.len() as f32 * 8.
        };
        let mut x = rect.x + (rect.width - 64. - text_width) / 2.;
        for glyph in ['⇧', '⌥', '⌘', '⌃'] {
            b.text(
                r(x, rect.y + 9., 14., 20.),
                glyph.to_string(),
                13.,
                400,
                if !recording && !unset && value.contains(glyph) {
                    p.ink
                } else {
                    p.faint
                },
                FontFamily::Sans,
            );
            x += 16.;
        }
        b.text(
            r(
                x + 2.,
                rect.y + if recording || unset { 10. } else { 9. },
                text_width + 2.,
                20.,
            ),
            text,
            if recording || unset { 10.5 } else { 13. },
            if recording || unset { 400 } else { 600 },
            if recording || unset {
                p.faint
            } else {
                p.accent
            },
            FontFamily::Sans,
        );
    }
}

use super::*;
use super::{r, Paint};
use crate::settings::field::Field;

pub(super) fn theme_height(width: f32, narrow: bool, compact: bool) -> f32 {
    if narrow {
        3. * 83.625 + 18. + 48.
    } else {
        ((width - if compact { 18. } else { 30. }) / 3. - 18.) / 1.6 + 119.
    }
}

impl SettingsUi {
    pub(super) fn paint_general_edit(&self, b: &mut Paint, field: GeneralField, rect: Rect) {
        self.paint_field_edit(b, Field::General(field), rect);
        if let Some((min, max, _, suffix)) = field.range() {
            let p = b.p;
            let value = match field.value(&self.snapshot.general) {
                Value::Number(n) => n,
                _ => 0,
            };
            if !suffix.is_empty() {
                b.fill(
                    r(
                        rect.x + rect.width - 92.,
                        rect.y + 1.,
                        48.,
                        rect.height - 2.,
                    ),
                    0.,
                    p.surface,
                );
                b.text(
                    r(rect.x + rect.width - 87., rect.y + 13., 40., 20.),
                    suffix,
                    12.,
                    400,
                    p.muted,
                    FontFamily::Sans,
                );
            }
            b.fill(
                r(
                    rect.x + rect.width - 34.,
                    rect.y + 1.,
                    32.,
                    rect.height - 2.,
                ),
                0.,
                p.surface,
            );
            for (increase, y, enabled) in [
                (true, rect.y, value < max),
                (false, rect.y + 20., value > min),
            ] {
                let target = Target::GeneralStep(field, increase);
                let x = rect.x + rect.width - 26.;
                let yy = y + 8.;
                let color = if enabled { p.muted } else { p.faint };
                let dy = if increase { -3. } else { 3. };
                b.line(x, yy, x + 4., yy + dy, color);
                b.line(x + 4., yy + dy, x + 8., yy, color);
                if enabled {
                    b.hit(
                        r(rect.x + rect.width - 32., y, 24., 20.),
                        target,
                        if increase { "增加" } else { "减少" },
                    );
                }
            }
        }
    }
    pub(super) fn paint_theme_picker(&self, b: &mut Paint, area: Rect) {
        let p = b.p;
        let narrow = self.size.width / self.scale <= 700.;
        let gap = if self.size.width / self.scale <= 900. {
            9.
        } else {
            15.
        };
        let width = if narrow {
            area.width
        } else {
            (area.width - gap * 2.) / 3.
        };
        let mini_width = if narrow { 105. } else { width - 18. };
        let mini_height = mini_width / 1.6;
        let height = if narrow {
            mini_height + 18.
        } else {
            mini_height + 71.
        };
        let names = [
            ("澄明", "Native", "清透蓝调 · 经典侧栏"),
            ("夜航", "Studio", "琥珀强调 · 紧凑工作台"),
            ("松间", "Paper", "暖纸松绿 · 舒展阅读"),
        ];
        for (index, (name, english, description)) in names.iter().enumerate() {
            let x = area.x
                + if narrow {
                    0.
                } else {
                    index as f32 * (width + gap)
                };
            let y = area.y
                + 24.
                + if narrow {
                    index as f32 * (height + gap)
                } else {
                    0.
                };
            let selected = GeneralField::InterfaceTheme.choices()[index].0
                == self.snapshot.general.interface_theme;
            let rect = r(x, y, width, height);
            let mini = r(x + 9., y + 9., mini_width, mini_height);
            let colors = palette(
                [Theme::Native, Theme::Studio, Theme::Paper][index],
                index == 1,
            );
            b.fill(rect, 10., p.surface);
            b.stroke(rect, 10., 1., if selected { p.accent } else { p.line });
            if selected {
                b.stroke(
                    r(x - 2., y - 2., width + 4., height + 4.),
                    12.,
                    2.,
                    p.accent_soft,
                );
            }
            b.fill(mini, 6., colors.canvas);
            b.stroke(mini, 6., 1., colors.line);
            let chrome = mini.height * 0.15;
            b.line(
                mini.x,
                mini.y + chrome,
                mini.x + mini.width,
                mini.y + chrome,
                colors.line,
            );
            for i in 0..3 {
                b.fill(
                    r(mini.x + 5. + i as f32 * 6., mini.y + 5., 3., 3.),
                    1.5,
                    colors.accent,
                );
            }
            let (nav_width, content_x, content_y) = if index == 1 {
                (mini.width * 0.16, mini.width * 0.24, mini.height * 0.25)
            } else if index == 2 {
                (mini.width, mini.width * 0.13, mini.height * 0.41)
            } else {
                (mini.width * 0.25, mini.width * 0.32, mini.height * 0.25)
            };
            b.fill(
                r(
                    mini.x,
                    mini.y + chrome,
                    nav_width,
                    if index == 2 {
                        mini.height * 0.16
                    } else {
                        mini.height - chrome
                    },
                ),
                0.,
                colors.sidebar,
            );
            for i in 0..4 {
                let xx = mini.x
                    + if index == 2 {
                        mini.width * (0.09 + i as f32 * 0.22)
                    } else {
                        mini.width * 0.04
                    };
                let yy = mini.y
                    + chrome
                    + if index == 2 {
                        mini.height * 0.05
                    } else {
                        mini.height * 0.12 + i as f32 * 9.
                    };
                b.fill(
                    r(
                        xx,
                        yy,
                        if index == 2 {
                            mini.width * 0.15
                        } else {
                            nav_width - mini.width * 0.08
                        },
                        if index == 2 { 2. } else { 3. },
                    ),
                    0.,
                    if i == 1 {
                        colors.accent
                    } else {
                        colors.line_strong
                    },
                );
            }
            let content_w =
                mini.width - content_x - mini.width * if index == 2 { 0.13 } else { 0.07 };
            let content_h = mini.height - content_y - mini.height * 0.07;
            b.fill(
                r(
                    mini.x + content_x,
                    mini.y + content_y,
                    content_w * 0.29,
                    content_h * 0.08,
                ),
                0.,
                colors.accent,
            );
            for i in 0..3 {
                let row = r(
                    mini.x + content_x,
                    mini.y + content_y + content_h * 0.23 + i as f32 * content_h * 0.26,
                    content_w,
                    content_h * 0.18,
                );
                if index == 2 {
                    b.line(
                        row.x,
                        row.y + row.height,
                        row.x + row.width,
                        row.y + row.height,
                        colors.line,
                    );
                } else {
                    b.fill(row, if index == 1 { 2. } else { 3. }, colors.surface);
                    b.stroke(row, if index == 1 { 2. } else { 3. }, 1., colors.line);
                }
            }
            let tx = if narrow { x + 9. + 105. + 12. } else { x + 13. };
            let ty = if narrow {
                y + 20.
            } else {
                y + 9. + mini_height + 12.
            };
            b.text(
                r(tx, ty, width - (tx - x) - 18., 20.),
                *name,
                13.,
                600,
                p.ink,
                FontFamily::Sans,
            );
            if self.size.width / self.scale > 900. {
                b.text(
                    r(tx + 32., ty + 3., width - 70., 16.),
                    *english,
                    10.,
                    400,
                    p.faint,
                    FontFamily::Sans,
                );
            }
            if selected {
                b.text(
                    r(x + width - 22., ty + 1., 14., 18.),
                    "✓",
                    12.,
                    400,
                    p.accent,
                    FontFamily::Sans,
                );
            }
            b.text(
                r(tx, ty + 23., width - (tx - x) - 13., 18.),
                *description,
                if self.size.width / self.scale <= 900. {
                    9.
                } else {
                    10.
                },
                400,
                p.muted,
                FontFamily::Sans,
            );
            b.hit(
                rect,
                Target::GeneralTheme(index),
                GeneralField::InterfaceTheme.choices()[index].1,
            );
        }
    }
    fn general_rows(&self, section: Section) -> Vec<GeneralRow> {
        use GeneralCommand::*;
        let runtime = &self.snapshot.general_runtime;
        let mut rows = vec![];
        match section {
            Section::Permissions if !runtime.permissions_required => rows.push(GeneralRow::Note(
                "当前系统无需单独开启屏幕录制或辅助功能权限。".into(),
            )),
            Section::Permissions => {
                if runtime.needs_installation {
                    rows.push(GeneralRow::Note("当前运行的是临时副本。请将 SnapLingo 拖入“应用程序”后打开，再检测是否需要授权。".into()));
                }
                for (label, detail, status, command) in [
                    (
                        "屏幕录制",
                        "截图与屏幕识别",
                        runtime.screen_recording,
                        RequestScreenRecording,
                    ),
                    (
                        "辅助功能",
                        "选中文本与界面元素检测（可选）",
                        runtime.accessibility,
                        RequestAccessibility,
                    ),
                ] {
                    rows.push(GeneralRow::Permission {
                        label,
                        detail,
                        status,
                        command,
                        enabled: !runtime.needs_installation && !runtime.permission_busy,
                    });
                }
                if let Some(error) = &runtime.permission_error {
                    rows.push(GeneralRow::Note(error.clone()));
                }
                if runtime.screen_recording == Some(false) || runtime.accessibility == Some(false) {
                    rows.push(GeneralRow::Note("如果你此前已授权，重装或更新时的应用签名变化可能使旧版本的授权未对当前应用生效。仅凭系统检测无法判断是尚未授权，还是旧授权未生效。若系统中已开启，请先重新检测；仅在 macOS 要求时重启应用，仍不可用再重新授权。".into()));
                }
                rows.push(GeneralRow::Disclosure(
                    "授权遇到问题？",
                    false,
                    self.general_permission_help,
                ));
                if self.general_permission_help {
                    rows.push(GeneralRow::Actions("重新检测", vec![RefreshPermissions]));
                    if let Some(path) = &runtime.app_path {
                        rows.push(GeneralRow::Note(format!("当前应用：{path}")));
                    }
                    if runtime.can_reset_permissions && !runtime.needs_installation {
                        let mut commands = vec![];
                        if runtime.screen_recording == Some(false) {
                            commands.push(ResetScreenRecording);
                        }
                        if runtime.accessibility == Some(false) {
                            commands.push(ResetAccessibility);
                        }
                        if !commands.is_empty() {
                            rows.push(GeneralRow::Actions("重新授权", commands));
                        }
                    }
                    rows.push(GeneralRow::Actions(
                        "如果 macOS 要求，请重启一次。",
                        vec![Restart],
                    ));
                }
            }
            Section::Maintenance => {
                rows.push(GeneralRow::Value(
                    "日志存储",
                    format!("最近 {} 条", runtime.logs.len()),
                ));
                rows.push(GeneralRow::Actions(
                    "日志操作",
                    vec![RefreshLogs, ClearLogs],
                ));
                if !runtime.logs.is_empty() {
                    rows.push(GeneralRow::Logs(runtime.logs.clone()));
                }
                if self.snapshot.theme != Theme::Paper {
                    rows.push(GeneralRow::Spacer(1.));
                }
                rows.push(GeneralRow::Actions("历史记录", vec![ClearHistory]));
                rows.push(GeneralRow::Disabled("应用设置", "暂未开放"));
            }
            Section::About => {
                if self.snapshot.theme != Theme::Paper {
                    rows.push(GeneralRow::Spacer(1.));
                }
                rows.push(GeneralRow::Value(
                    "当前版本",
                    if runtime.version.is_empty() {
                        "未知".into()
                    } else {
                        runtime.version.clone()
                    },
                ));
                rows.push(GeneralRow::Value("开源协议", "MIT License".into()));
                rows.push(GeneralRow::Actions("项目主页", vec![OpenProject]));
                rows.push(GeneralRow::Actions("第三方组件", vec![ShowNotices]));
                rows.push(GeneralRow::Update);
                rows.push(GeneralRow::Spacer(1.));
                if runtime.update_download_available {
                    rows.push(GeneralRow::Actions("安装包", vec![DownloadUpdate]));
                    if let Some(size) = runtime.update_size {
                        rows.push(GeneralRow::Note(format!("安装包大小：{} MB。安装包打开后，退出 SnapLingo，将新版拖入“应用程序”替换旧版，再打开。设置和历史记录会保留。",size.div_ceil(1_000_000))));
                    }
                } else if runtime.update_available {
                    rows.push(GeneralRow::Note(
                        "此版本暂未提供可验证的适配安装包，请前往发布页面查看。".into(),
                    ));
                }
                if let Some(path) = &runtime.downloaded_path {
                    rows.push(GeneralRow::Note(format!("安装包位置：{path}")));
                }
                if runtime.update_available && !runtime.update_notes.is_empty() {
                    rows.push(GeneralRow::Disclosure(
                        "更新说明",
                        true,
                        self.general_release_notes,
                    ));
                    if self.general_release_notes {
                        rows.push(GeneralRow::Note(runtime.update_notes.clone()));
                    }
                }
            }
            _ => {}
        }
        rows
    }
    pub(super) fn general_extra_height(&self, section: Section, width: f32) -> f32 {
        self.general_rows(section)
            .iter()
            .map(|row| {
                row.height(
                    width,
                    self.size.width / self.scale <= 700.,
                    self.snapshot.theme,
                )
            })
            .sum()
    }
    pub(super) fn paint_general_extra(
        &self,
        b: &mut Paint,
        section: Section,
        area: Rect,
        narrow: bool,
    ) {
        use GeneralCommand::*;
        let p = b.p;
        let mut y = area.y;
        for row in self.general_rows(section) {
            if let GeneralRow::Spacer(height) = row {
                y += height;
                continue;
            }
            let height = row.height(area.width, narrow, self.snapshot.theme);
            b.line(area.x, y, area.x + area.width, y, p.line);
            let padding = match self.snapshot.theme {
                Theme::Studio => 9.,
                Theme::Paper => 16.,
                _ => 13.,
            };
            let top = y + padding;
            let content_y = if narrow { top + 28. } else { top };
            match row {
                GeneralRow::Spacer(_) => {}
                GeneralRow::Update => {
                    let runtime = &self.snapshot.general_runtime;
                    let x = area.x + area.width - 145.875;
                    b.text(
                        r(area.x, top + 1., area.width - 170., 20.),
                        "软件更新",
                        12.5,
                        600,
                        p.ink,
                        FontFamily::Sans,
                    );
                    b.text(
                        r(area.x, top + 22., area.width - 170., 18.),
                        if runtime.update_description.is_empty() {
                            "打开设置时自动查找最新正式版"
                        } else {
                            &runtime.update_description
                        },
                        10.5,
                        400,
                        p.muted,
                        FontFamily::Sans,
                    );
                    for (index, command) in [CheckUpdates, OpenReleasePage].into_iter().enumerate()
                    {
                        let rect = r(
                            x + index as f32 * 76.9375,
                            if narrow {
                                y + padding + 48.
                            } else {
                                y + (height - 33.) / 2. + 0.5
                            },
                            68.9375,
                            33.,
                        );
                        if !command.enabled(runtime) {
                            b.fill(rect, 8., p.raised);
                            b.scene.geometry.push(GeometryNode {
                                kind: GeometryKind::Control(Target::GeneralCommand(command)),
                                rect,
                            });
                            b.center(
                                r(rect.x + 4., rect.y + 9., rect.width - 8., 20.),
                                command.label(),
                                10.5,
                                p.faint,
                            );
                        } else {
                            b.button(
                                rect,
                                Target::GeneralCommand(command),
                                command.label(),
                                false,
                            );
                        }
                    }
                }
                GeneralRow::Disclosure(label, notes, expanded) => {
                    let rect = r(area.x, top, area.width, 24.);
                    b.text(
                        r(area.x, top, area.width, 24.),
                        format!("{} {label}", if expanded { "▾" } else { "▸" }),
                        12.,
                        500,
                        p.ink,
                        FontFamily::Sans,
                    );
                    b.hit(rect, Target::GeneralDisclosure(notes), label);
                }
                GeneralRow::Note(text) => b.text(
                    r(area.x, top, area.width, height - 18.),
                    text,
                    11.,
                    400,
                    p.muted,
                    FontFamily::Sans,
                ),
                GeneralRow::Logs(logs) => {
                    b.fill(r(area.x, top, area.width, height - 26.), 8., p.raised);
                    for (index, line) in logs.iter().take(8).enumerate() {
                        b.text(
                            r(
                                area.x + 12.,
                                top + 8. + index as f32 * 20.,
                                area.width - 24.,
                                20.,
                            ),
                            line,
                            10.5,
                            400,
                            p.muted,
                            FontFamily::Mono,
                        );
                    }
                }
                GeneralRow::Value(label, value) => {
                    b.text(
                        r(area.x, top + 8., area.width * 0.5, 20.),
                        label,
                        12.5,
                        600,
                        p.ink,
                        FontFamily::Sans,
                    );
                    b.text(
                        r(
                            if narrow {
                                area.x
                            } else {
                                area.x + area.width - 230.
                            },
                            content_y + 8.,
                            230.,
                            20.,
                        ),
                        value,
                        12.,
                        400,
                        p.muted,
                        FontFamily::Sans,
                    );
                }
                GeneralRow::Disabled(label, value) => {
                    b.text(
                        r(area.x, top + 8., area.width * 0.5, 20.),
                        label,
                        12.5,
                        600,
                        p.ink,
                        FontFamily::Sans,
                    );
                    let rect = r(
                        if narrow {
                            area.x
                        } else {
                            area.x + area.width - 132.
                        },
                        if narrow {
                            content_y + 1.
                        } else {
                            y + (height - 36.) / 2. + 0.5
                        },
                        132.,
                        36.,
                    );
                    b.fill(rect, 8., p.raised);
                    b.stroke(rect, 8., 1., p.line);
                    b.scene.geometry.push(GeometryNode {
                        kind: GeometryKind::Control(Target::GeneralUnavailable),
                        rect,
                    });
                    b.center(
                        r(rect.x + 4., rect.y + 10., rect.width - 8., 20.),
                        value,
                        12.,
                        p.faint,
                    );
                }
                GeneralRow::Actions(label, commands) => {
                    b.text(
                        r(
                            area.x,
                            top + 8.,
                            if narrow {
                                area.width
                            } else {
                                area.width - 285.
                            },
                            20.,
                        ),
                        label,
                        12.5,
                        600,
                        p.ink,
                        FontFamily::Sans,
                    );
                    let total = commands.iter().map(|c| command_width(*c)).sum::<f32>()
                        + (commands.len().saturating_sub(1)) as f32 * 8.;
                    let mut x = if narrow {
                        area.x
                    } else {
                        area.x + area.width - total
                    };
                    for command in commands {
                        let width = command_width(command);
                        let control_y = if narrow {
                            content_y + if label == "历史记录" { 0. } else { 1. }
                        } else {
                            y + (height - 36.) / 2. + if label == "历史记录" { 0. } else { 0.5 }
                        };
                        let rect = r(x, control_y, width, 36.);
                        if !command.enabled(&self.snapshot.general_runtime) {
                            b.fill(rect, 8., p.raised);
                            b.scene.geometry.push(GeometryNode {
                                kind: GeometryKind::Control(Target::GeneralCommand(command)),
                                rect,
                            });
                            b.center(
                                r(x + 4., content_y + 10., width - 8., 20.),
                                command.label(),
                                11.,
                                p.faint,
                            );
                        } else {
                            b.button(
                                rect,
                                Target::GeneralCommand(command),
                                command.label(),
                                false,
                            );
                        }
                        x += width + 8.;
                    }
                }
                GeneralRow::Permission {
                    label,
                    detail,
                    status,
                    command,
                    enabled,
                } => {
                    b.text(
                        r(area.x, top, area.width - 210., 20.),
                        label,
                        12.5,
                        600,
                        p.ink,
                        FontFamily::Sans,
                    );
                    b.text(
                        r(area.x, top + 23., area.width - 210., 20.),
                        detail,
                        10.5,
                        400,
                        p.muted,
                        FontFamily::Sans,
                    );
                    let text = if self.snapshot.general_runtime.permission_error.is_some() {
                        "检测失败"
                    } else {
                        match status {
                            None => "检测中…",
                            Some(true) => "已授权",
                            Some(false) => "待授权",
                        }
                    };
                    b.text(
                        r(area.x + area.width - 200., top + 10., 80., 20.),
                        text,
                        11.,
                        400,
                        if status == Some(true) {
                            p.good
                        } else {
                            p.muted
                        },
                        FontFamily::Sans,
                    );
                    if status == Some(false) && enabled {
                        b.button(
                            r(area.x + area.width - 112., top, 112., 36.),
                            Target::GeneralCommand(command),
                            "去授权",
                            false,
                        );
                    }
                }
            }
            y += height;
        }
    }
}
enum GeneralRow {
    Value(&'static str, String),
    Disabled(&'static str, &'static str),
    Actions(&'static str, Vec<GeneralCommand>),
    Update,
    Spacer(f32),
    Note(String),
    Logs(Vec<String>),
    Disclosure(&'static str, bool, bool),
    Permission {
        label: &'static str,
        detail: &'static str,
        status: Option<bool>,
        command: GeneralCommand,
        enabled: bool,
    },
}
impl GeneralRow {
    fn height(&self, width: f32, narrow: bool, theme: Theme) -> f32 {
        let padding = match theme {
            Theme::Studio => 9.,
            Theme::Paper => 16.,
            _ => 13.,
        };
        let min = match theme {
            Theme::Studio => 53.,
            Theme::Paper => 67.,
            _ => 62.,
        };
        match self {
            Self::Spacer(height) => *height,
            Self::Update => {
                if narrow {
                    padding * 2. + 81.
                } else {
                    padding * 2. + 38.
                }
            }
            Self::Value(label, _) => {
                if narrow {
                    padding * 2. + 53. - if *label == "当前版本" { 1. } else { 0. }
                } else {
                    min
                }
            }
            Self::Actions(label, _) => {
                padding * 2.
                    + 36.
                    + if narrow { 28. } else { 0. }
                    + if *label == "历史记录" { 0. } else { 1. }
            }
            Self::Disabled(..) => padding * 2. + 37. + if narrow { 28. } else { 0. },
            Self::Disclosure(..) => {
                if theme == Theme::Paper {
                    49.
                } else {
                    50.
                }
            }
            Self::Note(text) => {
                ((text.chars().count() as f32 * 11. / width.max(60.)).ceil() * 18.).max(38.) + 26.
            }
            Self::Logs(logs) => logs.len().min(8) as f32 * 20. + 42.,
            Self::Permission { .. } => 72.,
        }
    }
}
fn command_width(command: GeneralCommand) -> f32 {
    match command {
        GeneralCommand::ResetScreenRecording | GeneralCommand::ResetAccessibility => 150.,
        GeneralCommand::DownloadUpdate => 164.,
        _ => 132.,
    }
}

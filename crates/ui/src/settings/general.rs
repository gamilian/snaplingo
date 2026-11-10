use crate::{Action, Section, Snapshot, Theme, Value};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneralPreferences {
    pub interface_theme: String,
    pub language: String,
    pub appearance: String,
    pub start_on_boot: bool,
    pub proxy_mode: String,
    pub proxy_url: String,
    pub request_timeout_ms: u32,
    pub retry_count: u32,
    pub log_level: String,
    pub log_retention_days: u32,
    pub gpu_composition: bool,
    pub performance_monitoring: bool,
}
impl Default for GeneralPreferences {
    fn default() -> Self {
        Self {
            interface_theme: "native".into(),
            language: "zh-CN".into(),
            appearance: "system".into(),
            start_on_boot: false,
            proxy_mode: "system".into(),
            proxy_url: String::new(),
            request_timeout_ms: 10_000,
            retry_count: 1,
            log_level: "info".into(),
            log_retention_days: 7,
            gpu_composition: false,
            performance_monitoring: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeneralField {
    InterfaceTheme,
    Language,
    Appearance,
    StartOnBoot,
    ProxyMode,
    ProxyUrl,
    RequestTimeout,
    RetryCount,
    LogLevel,
    LogRetention,
    GpuComposition,
    PerformanceMonitoring,
}
impl GeneralField {
    pub const ALL: [Self; 12] = [
        Self::InterfaceTheme,
        Self::Language,
        Self::Appearance,
        Self::StartOnBoot,
        Self::ProxyMode,
        Self::ProxyUrl,
        Self::RequestTimeout,
        Self::RetryCount,
        Self::LogLevel,
        Self::LogRetention,
        Self::GpuComposition,
        Self::PerformanceMonitoring,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::InterfaceTheme => "界面主题",
            Self::Language => "界面语言",
            Self::Appearance => "外观模式",
            Self::StartOnBoot => "开机自启",
            Self::ProxyMode => "代理设置",
            Self::ProxyUrl => "代理地址",
            Self::RequestTimeout => "请求超时时间",
            Self::RetryCount => "失败重试次数",
            Self::LogLevel => "日志级别",
            Self::LogRetention => "日志保留天数",
            Self::GpuComposition => "GPU 合成",
            Self::PerformanceMonitoring => "性能监控",
        }
    }
    pub fn choices(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Self::InterfaceTheme => &[
                ("native", "澄明 · Native"),
                ("studio", "夜航 · Studio"),
                ("paper", "松间 · Paper"),
            ],
            Self::Language => &[("zh-CN", "中文简体")],
            Self::Appearance => &[("system", "跟随系统"), ("light", "浅色"), ("dark", "深色")],
            Self::ProxyMode => &[
                ("system", "跟随系统"),
                ("none", "不使用代理"),
                ("manual", "手动代理"),
            ],
            Self::LogLevel => &[
                ("debug", "Debug"),
                ("info", "Info"),
                ("warn", "Warn"),
                ("error", "Error"),
            ],
            _ => &[],
        }
    }
    pub fn range(self) -> Option<(u32, u32, u32, &'static str)> {
        match self {
            Self::RequestTimeout => Some((1_000, 120_000, 1_000, "ms")),
            Self::RetryCount => Some((0, 5, 1, "")),
            Self::LogRetention => Some((1, 365, 1, "天")),
            _ => None,
        }
    }
    pub fn text_input(self) -> bool {
        self == Self::ProxyUrl || self.range().is_some()
    }
    pub fn accepts(self, value: &Value) -> bool {
        match value {
            Value::Text(v) => {
                self == Self::ProxyUrl || self.choices().iter().any(|(id, _)| *id == v)
            }
            Value::Number(v) => self
                .range()
                .is_some_and(|(min, max, _, _)| (min..=max).contains(v)),
            Value::Toggle(_) => matches!(
                self,
                Self::StartOnBoot | Self::GpuComposition | Self::PerformanceMonitoring
            ),
        }
    }
    pub fn value(self, s: &GeneralPreferences) -> Value {
        match self {
            Self::InterfaceTheme => Value::Text(s.interface_theme.clone()),
            Self::Language => Value::Text(s.language.clone()),
            Self::Appearance => Value::Text(s.appearance.clone()),
            Self::StartOnBoot => Value::Toggle(s.start_on_boot),
            Self::ProxyMode => Value::Text(s.proxy_mode.clone()),
            Self::ProxyUrl => Value::Text(s.proxy_url.clone()),
            Self::RequestTimeout => Value::Number(s.request_timeout_ms),
            Self::RetryCount => Value::Number(s.retry_count),
            Self::LogLevel => Value::Text(s.log_level.clone()),
            Self::LogRetention => Value::Number(s.log_retention_days),
            Self::GpuComposition => Value::Toggle(s.gpu_composition),
            Self::PerformanceMonitoring => Value::Toggle(s.performance_monitoring),
        }
    }
    pub(crate) fn edit(self, s: &mut Snapshot, value: Value) -> Option<Action> {
        if self.value(&s.general) == value {
            return None;
        }
        if !self.accepts(&value) {
            return None;
        }
        match (self, &value) {
            (Self::InterfaceTheme, Value::Text(v)) => {
                s.general.interface_theme = v.clone();
                s.theme = match v.as_str() {
                    "studio" => Theme::Studio,
                    "paper" => Theme::Paper,
                    _ => Theme::Native,
                };
            }
            (Self::Language, Value::Text(v)) => s.general.language = v.clone(),
            (Self::Appearance, Value::Text(v)) => {
                s.general.appearance = v.clone();
                if v != "system" {
                    s.dark = v == "dark";
                }
            }
            (Self::StartOnBoot, Value::Toggle(v)) => s.general.start_on_boot = *v,
            (Self::ProxyMode, Value::Text(v)) => s.general.proxy_mode = v.clone(),
            (Self::ProxyUrl, Value::Text(v)) => s.general.proxy_url = v.clone(),
            (Self::RequestTimeout, Value::Number(v)) => s.general.request_timeout_ms = *v,
            (Self::RetryCount, Value::Number(v)) => s.general.retry_count = *v,
            (Self::LogLevel, Value::Text(v)) => s.general.log_level = v.clone(),
            (Self::LogRetention, Value::Number(v)) => s.general.log_retention_days = *v,
            (Self::GpuComposition, Value::Toggle(v)) => s.general.gpu_composition = *v,
            (Self::PerformanceMonitoring, Value::Toggle(v)) => {
                s.general.performance_monitoring = *v
            }
            _ => return None,
        }
        Some(Action::EditGeneral { field: self, value })
    }
}

pub(crate) fn fields(section: Section, s: &GeneralPreferences) -> Vec<GeneralField> {
    use GeneralField::*;
    match section {
        Section::Interface => vec![InterfaceTheme, Language, Appearance, StartOnBoot],
        Section::Network => {
            let mut fields = vec![ProxyMode];
            if s.proxy_mode == "manual" {
                fields.push(ProxyUrl);
            }
            fields.extend([RequestTimeout, RetryCount]);
            fields
        }
        Section::Maintenance => vec![LogLevel, LogRetention],
        Section::Experimental => vec![GpuComposition, PerformanceMonitoring],
        _ => vec![],
    }
}

/// System operations use native effects; shared Settings owns clearing/reset confirmation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeneralCommand {
    RefreshPermissions,
    RequestScreenRecording,
    RequestAccessibility,
    ResetScreenRecording,
    ResetAccessibility,
    Restart,
    RefreshLogs,
    ClearLogs,
    ClearHistory,
    OpenProject,
    ShowNotices,
    CheckUpdates,
    OpenReleasePage,
    DownloadUpdate,
}
impl GeneralCommand {
    pub fn enabled(self, runtime: &GeneralRuntime) -> bool {
        match self {
            Self::CheckUpdates | Self::OpenReleasePage => !runtime.update_busy,
            Self::DownloadUpdate => !runtime.update_busy && runtime.update_download_available,
            Self::RefreshPermissions => !runtime.permission_busy,
            Self::RequestScreenRecording | Self::RequestAccessibility | Self::Restart => {
                !runtime.permission_busy
                    && !runtime.needs_installation
                    && runtime.screen_recording.is_some()
                    && runtime.accessibility.is_some()
            }
            Self::ResetScreenRecording | Self::ResetAccessibility => {
                !runtime.permission_busy
                    && !runtime.needs_installation
                    && runtime.can_reset_permissions
                    && runtime.screen_recording.is_some()
                    && runtime.accessibility.is_some()
            }
            _ => true,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::RefreshPermissions => "重新检测",
            Self::RequestScreenRecording => "授权屏幕录制",
            Self::RequestAccessibility => "授权辅助功能",
            Self::ResetScreenRecording => "重新授权屏幕录制",
            Self::ResetAccessibility => "重新授权辅助功能",
            Self::Restart => "重启 SnapLingo",
            Self::RefreshLogs => "刷新日志",
            Self::ClearLogs => "清空日志",
            Self::ClearHistory => "清除所有历史记录",
            Self::OpenProject => "GitHub",
            Self::ShowNotices => "查看许可声明",
            Self::CheckUpdates => "检查更新",
            Self::OpenReleasePage => "发布页面",
            Self::DownloadUpdate => "下载并打开安装包",
        }
    }
}

/// Runtime data supplied by the host, separate from durable preferences.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GeneralRuntime {
    pub permissions_required: bool,
    pub screen_recording: Option<bool>,
    pub accessibility: Option<bool>,
    pub permission_error: Option<String>,
    pub permission_busy: bool,
    pub app_path: Option<String>,
    pub needs_installation: bool,
    pub can_reset_permissions: bool,
    pub logs: Vec<String>,
    pub version: String,
    pub update_description: String,
    pub update_busy: bool,
    pub update_download_available: bool,
    pub update_available: bool,
    pub update_size: Option<u64>,
    pub update_notes: String,
    pub downloaded_path: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    fn ui() -> SettingsUi {
        let mut ui = SettingsUi::new(Snapshot::default());
        ui.update(Event::Activate(Target::Navigate(Page::General)));
        ui
    }
    #[test]
    fn six_sections_all_preferences_and_validated_dynamic_network_fields_share_one_page() {
        let mut ui = ui();
        assert_eq!(Section::for_page(ui.page()).len(), 6);
        assert!(!ui
            .frame
            .geometry
            .iter()
            .any(|n| n.kind == GeometryKind::Control(Target::Hotkey(0))));
        assert!(!ui
            .frame
            .geometry
            .iter()
            .any(|n| n.kind == GeometryKind::Control(Target::General(GeneralField::ProxyUrl))));
        assert_eq!(
            ui.update(Event::SetValue(
                Target::General(GeneralField::ProxyMode),
                Value::Text("manual".into())
            )),
            [Action::EditGeneral {
                field: GeneralField::ProxyMode,
                value: Value::Text("manual".into())
            }]
        );
        assert!(ui
            .frame
            .geometry
            .iter()
            .any(|n| n.kind == GeometryKind::Control(Target::General(GeneralField::ProxyUrl))));
        let before = ui.snapshot().clone();
        for (field, value) in [
            (GeneralField::Appearance, Value::Text("invalid".into())),
            (GeneralField::RequestTimeout, Value::Number(999)),
            (GeneralField::RetryCount, Value::Toggle(true)),
        ] {
            assert!(ui
                .update(Event::SetValue(Target::General(field), value))
                .is_empty());
        }
        assert_eq!(ui.snapshot(), &before);
    }
    #[test]
    fn theme_cards_and_appearance_switch_the_shared_raster_palette_without_navigation() {
        let mut ui = ui();
        assert_eq!(
            ui.update(Event::Activate(Target::GeneralTheme(1))),
            [Action::EditGeneral {
                field: GeneralField::InterfaceTheme,
                value: Value::Text("studio".into())
            }]
        );
        assert_eq!(ui.snapshot().theme, Theme::Studio);
        assert_eq!(ui.page(), Page::General);
        assert_eq!(
            ui.update(Event::SetValue(
                Target::General(GeneralField::Appearance),
                Value::Text("dark".into())
            ))
            .len(),
            1
        );
        assert!(ui.snapshot().dark);
    }
    #[test]
    fn numeric_input_is_not_a_slider_and_keeps_arrow_step_and_text_commits() {
        let mut ui = ui();
        let target = Target::General(GeneralField::RequestTimeout);
        ui.update(Event::Focus(target));
        assert_eq!(ui.text_edit().unwrap().text(), "10000");
        assert_eq!(
            ui.update(Event::KeyDown {
                key: Key::ArrowUp,
                modifiers: Modifiers::default(),
                repeat: false
            }),
            [Action::EditGeneral {
                field: GeneralField::RequestTimeout,
                value: Value::Number(11000)
            }]
        );
        assert_eq!(ui.text_edit().unwrap().text(), "11000");
        assert_eq!(
            ui.update(Event::Activate(Target::GeneralStep(
                GeneralField::RequestTimeout,
                false
            ))),
            [Action::EditGeneral {
                field: GeneralField::RequestTimeout,
                value: Value::Number(10000)
            }]
        );
        ui.update(Event::Focus(target));
        ui.update(Event::TextSelection(0..5));
        assert_eq!(
            ui.update(Event::TextInput("24000".into())),
            [Action::EditGeneral {
                field: GeneralField::RequestTimeout,
                value: Value::Number(24000)
            }]
        );
        assert!(ui.update(Event::FocusLost).is_empty());
        assert_eq!(ui.snapshot().general.request_timeout_ms, 24000);
    }
    #[test]
    fn proxy_input_uses_composition_and_defers_save_until_focus_loss() {
        let mut ui = ui();
        ui.update(Event::SetValue(
            Target::General(GeneralField::ProxyMode),
            Value::Text("manual".into()),
        ));
        ui.update(Event::Focus(Target::General(GeneralField::ProxyUrl)));
        assert!(ui
            .update(Event::ImePreedit {
                text: "中".into(),
                selection: Some(3..3)
            })
            .is_empty());
        assert_eq!(ui.text_edit().unwrap().text(), "");
        assert!(ui
            .update(Event::ImeCommit("http://代理:7890".into()))
            .is_empty());
        assert_eq!(
            ui.update(Event::FocusLost),
            [Action::EditGeneral {
                field: GeneralField::ProxyUrl,
                value: Value::Text("http://代理:7890".into())
            }]
        );
    }
    #[test]
    fn maintenance_host_actions_preserve_real_intents_and_permission_help_is_disclosed() {
        let mut snapshot = Snapshot::default();
        snapshot.general_runtime.permissions_required = true;
        snapshot.general_runtime.screen_recording = Some(false);
        snapshot.general_runtime.accessibility = Some(true);
        snapshot.general_runtime.can_reset_permissions = true;
        let mut ui = SettingsUi::new(snapshot);
        ui.update(Event::Activate(Target::Navigate(Page::General)));
        assert_eq!(
            ui.update(Event::Activate(Target::GeneralCommand(
                GeneralCommand::ClearLogs
            ))),
            [Action::GeneralCommand(GeneralCommand::ClearLogs)]
        );
        assert!(!ui.frame.geometry.iter().any(|n| n.kind
            == GeometryKind::Control(Target::GeneralCommand(
                GeneralCommand::ResetScreenRecording
            ))));
        ui.update(Event::Activate(Target::GeneralDisclosure(false)));
        assert!(ui.frame.geometry.iter().any(|n| n.kind
            == GeometryKind::Control(Target::GeneralCommand(
                GeneralCommand::ResetScreenRecording
            ))));
    }
}

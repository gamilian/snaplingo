use std::collections::HashMap;
use std::sync::OnceLock;

use crate::domain::GeneralSettings;

pub fn settings_ui_text(key: &str) -> &'static str {
    static TEXT: OnceLock<HashMap<String, String>> = OnceLock::new();
    TEXT.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../../../src/domain/settingsUiText.json"
        ))
        .expect("Bundled Settings text is valid")
    })[key]
        .as_str()
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
    SystemTtsVoice,
    SystemTtsRate,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GeneralValue {
    Text(String),
    Number(u32),
    Toggle(bool),
}

impl GeneralField {
    pub fn value(self, settings: &GeneralSettings) -> GeneralValue {
        use GeneralValue::*;
        match self {
            Self::InterfaceTheme => Text(settings.interface_theme.clone()),
            Self::Language => Text(settings.language.clone()),
            Self::Appearance => Text(settings.theme.clone()),
            Self::StartOnBoot => Toggle(settings.start_on_boot),
            Self::ProxyMode => Text(settings.proxy_mode.clone()),
            Self::ProxyUrl => Text(settings.proxy_url.clone()),
            Self::RequestTimeout => Number(settings.request_timeout_ms),
            Self::RetryCount => Number(settings.retry_count.into()),
            Self::LogLevel => Text(settings.log_level.clone()),
            Self::LogRetention => Number(settings.log_retention_days.into()),
            Self::GpuComposition => Toggle(settings.experimental_gpu_acceleration),
            Self::PerformanceMonitoring => Toggle(settings.performance_monitoring),
            Self::SystemTtsVoice => Text(settings.system_tts_voice.clone()),
            Self::SystemTtsRate => Number(settings.system_tts_rate.into()),
        }
    }

    pub(super) fn apply(
        self,
        settings: &mut GeneralSettings,
        value: GeneralValue,
    ) -> crate::Result<()> {
        use GeneralValue::*;
        match (self, value) {
            (Self::InterfaceTheme, Text(value)) => settings.interface_theme = value,
            (Self::Language, Text(value)) => settings.language = value,
            (Self::Appearance, Text(value)) => settings.theme = value,
            (Self::StartOnBoot, Toggle(value)) => settings.start_on_boot = value,
            (Self::ProxyMode, Text(value)) => settings.proxy_mode = value,
            (Self::ProxyUrl, Text(value)) => settings.proxy_url = value,
            (Self::RequestTimeout, Number(value)) => settings.request_timeout_ms = value,
            (Self::RetryCount, Number(value)) => settings.retry_count = value.min(5) as u8,
            (Self::LogLevel, Text(value)) => settings.log_level = value,
            (Self::LogRetention, Number(value)) => {
                settings.log_retention_days = value.clamp(1, 365) as u16
            }
            (Self::GpuComposition, Toggle(value)) => settings.experimental_gpu_acceleration = value,
            (Self::PerformanceMonitoring, Toggle(value)) => settings.performance_monitoring = value,
            (Self::SystemTtsVoice, Text(value)) => settings.system_tts_voice = value,
            (Self::SystemTtsRate, Number(value)) => {
                settings.system_tts_rate = value.clamp(80, 400) as u16
            }
            _ => return Err("Setting value does not match its control".into()),
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeneralSection {
    Interface,
    Permissions,
    Network,
    Maintenance,
    Experimental,
    About,
}

impl GeneralSection {
    pub const ALL: [Self; 6] = [
        Self::Interface,
        Self::Permissions,
        Self::Network,
        Self::Maintenance,
        Self::Experimental,
        Self::About,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::Interface => "interface",
            Self::Permissions => "permissions",
            Self::Network => "network",
            Self::Maintenance => "maintenance",
            Self::Experimental => "experimental",
            Self::About => "about",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Interface => settings_ui_text("interfaceAndStartup"),
            Self::Permissions => "系统权限",
            Self::Network => settings_ui_text("network"),
            Self::Maintenance => settings_ui_text("logsAndMaintenance"),
            Self::Experimental => settings_ui_text("experimental"),
            Self::About => settings_ui_text("about"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn general_sections_keep_the_current_routes_and_shared_ui_labels() {
        assert_eq!(
            GeneralSection::ALL.map(GeneralSection::id),
            [
                "interface",
                "permissions",
                "network",
                "maintenance",
                "experimental",
                "about"
            ]
        );
        assert_eq!(settings_ui_text("general"), "通用");
        assert_eq!(GeneralSection::Interface.label(), "界面与启动");
    }
}

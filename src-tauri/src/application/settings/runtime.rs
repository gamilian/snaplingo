use super::SettingsConfiguration;
use crate::application::History;
use crate::domain::{GeneralSettings, HistorySettings, SettingsSnapshot};
use crate::Result;
use serde::Serialize;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize)]
pub struct AppLogEntry {
    pub id: i64,
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
}
pub trait StartOnBoot: Send + Sync {
    fn set_enabled(&self, enabled: bool) -> Result<()>;
}
pub trait AppLogRepository: Send + Sync {
    fn list(&self, limit: usize) -> Result<Vec<AppLogEntry>>;
    fn clear(&self) -> Result<()>;
}

pub struct SettingsApplication {
    configuration: Arc<SettingsConfiguration>,
    start_on_boot: Arc<dyn StartOnBoot>,
    history: Arc<History>,
    logs: Arc<dyn AppLogRepository>,
}
impl SettingsApplication {
    pub fn new(
        configuration: Arc<SettingsConfiguration>,
        start_on_boot: Arc<dyn StartOnBoot>,
        history: Arc<History>,
        logs: Arc<dyn AppLogRepository>,
    ) -> Self {
        Self {
            configuration,
            start_on_boot,
            history,
            logs,
        }
    }
    pub fn update_general(&self, input: GeneralSettings) -> Result<SettingsSnapshot> {
        let previous = self.configuration.snapshot()?.general;
        let changed = input.start_on_boot != previous.start_on_boot;
        if changed {
            self.start_on_boot.set_enabled(input.start_on_boot)?;
        }
        self.configuration.update_general(input).inspect_err(|_| {
            if changed {
                let _ = self.start_on_boot.set_enabled(previous.start_on_boot);
            }
        })
    }
    pub async fn update_history(&self, input: HistorySettings) -> Result<SettingsSnapshot> {
        let snapshot = self.configuration.update_history(input)?;
        if let Err(error) = self.history.run_cleanup().await {
            log::warn!("Failed to apply history cleanup after settings update: {error}");
        }
        Ok(snapshot)
    }
    pub fn list_logs(&self, limit: usize) -> Result<Vec<AppLogEntry>> {
        self.logs.list(limit)
    }
    pub fn clear_logs(&self) -> Result<()> {
        self.logs.clear()
    }
}

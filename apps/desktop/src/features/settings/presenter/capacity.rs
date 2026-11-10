//! Capacity changes stay in a draft until the explicit Save action succeeds.
use snaplingo_ui::{capacity::*, Snapshot};
use std::sync::Arc;

pub struct CapacitySession {
    settings: Arc<snaplingo_core::settings::SettingsConfiguration>,
    pub ui: CapacityUi,
}
impl CapacitySession {
    pub fn new(
        settings: Arc<snaplingo_core::settings::SettingsConfiguration>,
        shell: Snapshot,
    ) -> Result<Self, String> {
        let saved = settings
            .snapshot()
            .map_err(|error| error.to_string())?
            .history;
        let ui = CapacityUi::new(
            shell,
            CapacityValue {
                automatic_cleanup: saved.auto_cleanup_enabled,
                retention_days: saved.retention_days,
                history_limit: saved.maximum_records,
                favorites_limit: saved.maximum_favorites,
            },
        );
        Ok(Self { settings, ui })
    }
    /// True closes the draft after cancellation or a durable successful save.
    pub fn input(&mut self, input: CapacityInput) -> bool {
        match self.ui.update(input) {
            Some(CapacityIntent::Cancel) => true,
            Some(CapacityIntent::Save(value)) => {
                match self.settings.update_history(
                    snaplingo_core::settings::model::HistorySettings {
                        auto_cleanup_enabled: value.automatic_cleanup,
                        retention_days: value.retention_days,
                        maximum_records: value.history_limit,
                        maximum_favorites: value.favorites_limit,
                    },
                ) {
                    Ok(_) => true,
                    Err(error) => {
                        self.ui.error(error.to_string());
                        false
                    }
                }
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snaplingo_storage::{Database, SqliteConfigStore};
    #[test]
    fn save_persists_all_fields_while_cancel_and_drop_leave_sqlite_unchanged() {
        let database = Arc::new(Database::in_memory().unwrap());
        let settings = Arc::new(snaplingo_core::settings::SettingsConfiguration::new(
            Arc::new(SqliteConfigStore::new(database.clone())),
        ));
        let saved = settings.snapshot().unwrap().history;
        {
            let mut draft = CapacitySession::new(settings.clone(), Snapshot::default()).unwrap();
            draft.input(CapacityInput::SetValue(CapacityTarget::Days, "365".into()));
            assert!(draft.input(CapacityInput::Activate(CapacityTarget::Cancel)));
        }
        {
            let mut draft = CapacitySession::new(settings.clone(), Snapshot::default()).unwrap();
            draft.input(CapacityInput::SetValue(
                CapacityTarget::History,
                "8000".into(),
            ));
        }
        assert_eq!(settings.snapshot().unwrap().history, saved);
        let mut draft = CapacitySession::new(settings.clone(), Snapshot::default()).unwrap();
        draft.input(CapacityInput::SetValue(CapacityTarget::Days, "999".into()));
        assert!(!draft.input(CapacityInput::Activate(CapacityTarget::Save)));
        assert_eq!(settings.snapshot().unwrap().history, saved);
        draft.input(CapacityInput::SetValue(CapacityTarget::Days, "40".into()));
        draft.input(CapacityInput::SetValue(
            CapacityTarget::History,
            "6000".into(),
        ));
        draft.input(CapacityInput::SetValue(
            CapacityTarget::Favorites,
            "2000".into(),
        ));
        draft.input(CapacityInput::Activate(CapacityTarget::Cleanup));
        assert!(draft.input(CapacityInput::Activate(CapacityTarget::Save)));
        let reopened = snaplingo_core::settings::SettingsConfiguration::new(Arc::new(
            SqliteConfigStore::new(database),
        ));
        let history = reopened.snapshot().unwrap().history;
        assert_eq!(history.retention_days, 40);
        assert_eq!(history.maximum_records, 6000);
        assert_eq!(history.maximum_favorites, 2000);
        assert_eq!(history.auto_cleanup_enabled, !saved.auto_cleanup_enabled);
    }
}

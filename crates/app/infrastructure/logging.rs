use crate::infrastructure::storage::SqliteAppLogRepository;

pub(crate) fn initialize(repository: SqliteAppLogRepository) -> Result<(), log::SetLoggerError> {
    dispatch(repository).apply()
}

fn dispatch(repository: SqliteAppLogRepository) -> fern::Dispatch {
    fern::Dispatch::new()
        .level(log::LevelFilter::Debug)
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Utc::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ));
        })
        .chain(fern::Output::call(move |record| {
            let _ = repository.record(
                record.level().as_str(),
                record.target(),
                &record.args().to_string(),
            );
        }))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use log::{Level, Metadata, Record};

    use crate::infrastructure::storage::{Database, SqliteConfigStore};
    use snaplingo_core::domain::SettingsSnapshot;

    use super::{dispatch, SqliteAppLogRepository};

    #[test]
    fn local_logger_preserves_records_and_reloads_the_durable_level() {
        let database = Arc::new(Database::in_memory().unwrap());
        let repository = SqliteAppLogRepository::new(database.clone());
        let settings = SqliteConfigStore::new(database.clone());
        let (maximum, logger) = dispatch(SqliteAppLogRepository::new(database)).into_log();
        assert_eq!(maximum, log::LevelFilter::Debug);
        assert!(logger.enabled(&Metadata::builder().level(Level::Debug).build()));
        assert!(!logger.enabled(&Metadata::builder().level(Level::Trace).build()));

        let record = |level| {
            logger.log(
                &Record::builder()
                    .level(level)
                    .target("capture::native")
                    .args(format_args!("原生选区\nsecond line"))
                    .build(),
            );
        };
        record(Level::Info);
        record(Level::Debug);
        let entries = repository.list(20).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].level, "INFO");
        assert_eq!(entries[0].target, "capture::native");
        assert!(entries[0]
            .message
            .ends_with("[capture::native][INFO] 原生选区\nsecond line"));
        chrono::NaiveDateTime::parse_from_str(&entries[0].message[..22], "[%Y-%m-%d][%H:%M:%S]")
            .unwrap();

        let mut snapshot = SettingsSnapshot::default();
        snapshot.general.log_level = "debug".into();
        settings.save_settings(&snapshot).unwrap();
        record(Level::Debug);
        record(Level::Trace);
        assert_eq!(repository.list(20).unwrap().len(), 2);
        assert_eq!(repository.list(20).unwrap()[0].level, "DEBUG");

        snapshot.general.log_level = "warn".into();
        settings.save_settings(&snapshot).unwrap();
        record(Level::Info);
        record(Level::Warn);
        assert_eq!(repository.list(20).unwrap().len(), 3);
        assert_eq!(repository.list(20).unwrap()[0].level, "WARN");
    }
}

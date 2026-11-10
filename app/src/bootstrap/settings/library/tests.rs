use super::*;
use snaplingo_core::selection::SelectedTextAcquirer;
use snaplingo_core::selection::SelectionContextProvider;
use snaplingo_core::selection::SelectionScheme;
use snaplingo_storage::Database;

struct Context;
impl SelectionContextProvider for Context {
    fn context(&self) -> snaplingo_core::selection::model::SelectionContext {
        Default::default()
    }
}
pub(in crate::bootstrap::settings) struct Fixture {
    pub(in crate::bootstrap::settings) app: Arc<crate::AppState>,
    _directory: tempfile::TempDir,
}
pub(in crate::bootstrap::settings) fn fixture() -> Fixture {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("library.db");
    let database = Arc::new(Database::open(&path).unwrap());
    database.with_connection(|connection| {
        for text in ["old", "new"] {
            connection.execute("INSERT INTO history_records(kind,timestamp) VALUES ('translation','2026-11-08T01:00:00+00:00')", []).map_err(snaplingo_core::AppError::database)?;
            let id = connection.last_insert_rowid();
            connection.execute("INSERT INTO translation_history(history_id,source_text,source_lang,target_lang,providers_used,results,duration_ms) VALUES (?1,?2,'en','zh-CN','[]','[]',1)", rusqlite::params![id,text]).map_err(snaplingo_core::AppError::database)?;
        }
        Ok(())
    }).unwrap();
    let state = crate::bootstrap::build_app_state(
        database.clone(),
        path,
        crate::bootstrap::CompositionOptions {
            identifier: "com.snaplingo.library-adapter-test".into(),
            version: "0.1.23".into(),
            ocr_resource_root: None,
            cache_dir: Ok(directory.path().join("cache")),
            selected_text: Arc::new(SelectedTextAcquirer::new(
                SelectionScheme::new(vec![]),
                vec![],
                Arc::new(Context),
            )),
        },
    );
    Fixture {
        _directory: directory,
        app: Arc::new(state),
    }
}

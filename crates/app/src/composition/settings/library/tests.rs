use super::core::{LibraryIndexPage, LibraryIndexQuery, LibraryIndexRepository};
use super::*;
use crate::application::selected_text::{
    SelectedTextAcquirer, SelectionContextProvider, SelectionScheme,
};
use crate::infrastructure::storage::{Database, SqliteLibraryIndexRepository};
use async_trait::async_trait;

struct Context;
impl SelectionContextProvider for Context {
    fn context(&self) -> crate::domain::SelectionContext {
        Default::default()
    }
}
struct Index {
    real: SqliteLibraryIndexRepository,
    entered: tokio::sync::Notify,
    release: tokio::sync::Notify,
}
#[async_trait]
impl LibraryIndexRepository for Index {
    async fn query_history(&self, query: &LibraryIndexQuery) -> crate::Result<LibraryIndexPage> {
        let page = self.real.query_history(query).await?;
        if query.search == "old" {
            self.entered.notify_one();
            self.release.notified().await;
        }
        Ok(page)
    }
    async fn query_favorites(&self, query: &LibraryIndexQuery) -> crate::Result<LibraryIndexPage> {
        self.real.query_favorites(query).await
    }
}
pub(in crate::composition::settings) struct Fixture {
    _directory: tempfile::TempDir,
    database: Arc<Database>,
    session: Arc<LibrarySession>,
    index: Arc<Index>,
    pub(in crate::composition::settings) app: Arc<crate::AppState>,
}
pub(in crate::composition::settings) fn fixture() -> Fixture {
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
    let state = crate::composition::build_app_state(
        database.clone(),
        path,
        crate::composition::CompositionOptions {
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
    let index = Arc::new(Index {
        real: SqliteLibraryIndexRepository::new(database.clone()),
        entered: Default::default(),
        release: Default::default(),
    });
    let pages = core::LibraryPages::new(
        state.history.history.clone(),
        state.favorites.favorites.clone(),
        state.screenshot_favorites.favorites.clone(),
        Arc::new(core::LibraryIndex::new(index.clone())),
        state.history.ocr_replay.clone(),
        state.favorites.ocr_application.clone(),
    );
    let session = LibrarySession::new(pages, Arc::new(super::Clipboard), ui::LibraryKind::History);
    Fixture {
        _directory: directory,
        database,
        session: Arc::new(session),
        index,
        app: Arc::new(state),
    }
}

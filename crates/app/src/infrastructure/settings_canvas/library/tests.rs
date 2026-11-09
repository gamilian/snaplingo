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
pub(in crate::infrastructure::settings_canvas) struct Fixture {
    _directory: tempfile::TempDir,
    database: Arc<Database>,
    session: Arc<LibrarySession>,
    index: Arc<Index>,
    pub(in crate::infrastructure::settings_canvas) app: Arc<crate::AppState>,
}
pub(in crate::infrastructure::settings_canvas) fn fixture() -> Fixture {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("library.db");
    let database = Arc::new(Database::open(&path).unwrap());
    database.with_connection(|connection| {
        for text in ["old", "new"] {
            connection.execute("INSERT INTO history_records(kind,timestamp) VALUES ('translation','2026-11-08T01:00:00+00:00')", [])?;
            let id = connection.last_insert_rowid();
            connection.execute("INSERT INTO translation_history(history_id,source_text,source_lang,target_lang,providers_used,results,duration_ms) VALUES (?1,?2,'en','zh-CN','[]','[]',1)", rusqlite::params![id,text])?;
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
    let mut session = LibrarySession::from_state(&state, ui::LibraryKind::History);
    session.pages = core::LibraryPages::new(
        state.history.history.clone(),
        state.favorites.favorites.clone(),
        state.screenshot_favorites.favorites.clone(),
        Arc::new(core::LibraryIndex::new(index.clone())),
        state.history.ocr_replay.clone(),
        state.favorites.ocr_application.clone(),
    );
    Fixture {
        _directory: directory,
        database,
        session: Arc::new(session),
        index,
        app: Arc::new(state),
    }
}
fn query(revision: u64, search: &str) -> ui::LibraryIntent {
    ui::LibraryIntent::Query {
        revision,
        kind: ui::LibraryKind::History,
        query: ui::LibraryQuery {
            search: search.into(),
            ..Default::default()
        },
    }
}

#[tokio::test]
async fn preparing_a_new_search_rejects_the_old_result_before_the_new_future_is_polled() {
    let f = fixture();
    let old = tokio::spawn(f.session.dispatch(query(1, "old"), || {}));
    f.index.entered.notified().await;
    let new = f.session.dispatch(query(2, "new"), || {});
    f.index.release.notify_one();
    old.await.unwrap().unwrap();
    let pending = f.session.snapshot(Snapshot::default());
    assert!(pending.loading);
    assert!(
        pending.items.is_empty(),
        "The old result must not replace the new search draft"
    );
    new.await.unwrap();
    let ready = f.session.snapshot(Snapshot::default());
    assert!(!ready.loading);
    assert_eq!(ready.query.search, "new");
    assert_eq!(ready.items.len(), 1);
    assert_eq!(ready.items[0].key.id, 2);
}

#[tokio::test]
async fn metadata_blurs_preserve_input_order_when_worker_futures_are_polled_in_reverse() {
    let f = fixture();
    let content =
        FavoriteContent::Translation(crate::application::favorites::TranslationFavoriteSnapshot {
            source_text: "source".into(),
            source_lang: "en".into(),
            target_lang: "zh-CN".into(),
            result: crate::domain::translation::TranslationResult::success(
                "system",
                "translated",
                None,
                None,
            ),
        });
    f.database.with_connection(|connection| {
        connection.execute("INSERT INTO favorites(kind,created_at,fingerprint,content_json) VALUES ('translation','2026-11-08T01:00:00+00:00','fixture',?1)", [serde_json::to_string(&content)?])?;
        Ok(())
    }).unwrap();
    f.session
        .dispatch(
            ui::LibraryIntent::Query {
                revision: 1,
                kind: ui::LibraryKind::Favorites,
                query: Default::default(),
            },
            || {},
        )
        .await
        .unwrap();
    let key = f.session.snapshot(Snapshot::default()).items[0].key;
    let first = f.session.dispatch(
        ui::LibraryIntent::SaveMetadata {
            key,
            note: Some("first".into()),
            tags: vec!["one".into()],
        },
        || {},
    );
    let last = f.session.dispatch(
        ui::LibraryIntent::SaveMetadata {
            key,
            note: Some("最后🙂".into()),
            tags: vec!["two".into()],
        },
        || {},
    );
    last.await.unwrap();
    first.await.unwrap();
    let reopened = Database::open(f._directory.path().join("library.db")).unwrap();
    let saved: (Option<String>, String) = reopened
        .with_connection(|connection| {
            Ok(connection.query_row(
                "SELECT note,(SELECT group_concat(name) FROM tags JOIN favorite_tags ON tags.id=favorite_tags.tag_id WHERE favorite_tags.favorite_id=favorites.id) FROM favorites WHERE id=?1",
                [key.id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?)
        })
        .unwrap();
    assert_eq!(saved.0.as_deref(), Some("最后🙂"));
    assert_eq!(saved.1, "two");
}

#[tokio::test]
async fn disposing_the_page_cancels_queued_mutations_before_the_worker_starts() {
    let f = fixture();
    f.session.dispatch(query(1, ""), || {}).await.unwrap();
    let key = f.session.snapshot(Snapshot::default()).items[0].key;
    let pending = f.session.dispatch(ui::LibraryIntent::Delete(key), || {});
    f.session.dispose();
    pending.await.unwrap();
    let count: i64 = f
        .database
        .with_connection(|connection| {
            Ok(connection.query_row("SELECT COUNT(*) FROM history_records", [], |r| r.get(0))?)
        })
        .unwrap();
    assert_eq!(count, 2);
    assert!(f.session.state.lock().unwrap().pending.is_empty());
}

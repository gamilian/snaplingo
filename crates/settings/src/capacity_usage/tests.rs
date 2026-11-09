use super::*;
use async_trait::async_trait;
use snaplingo_core::application::library_index::{LibraryIndexPage, LibraryIndexRepository};
use snaplingo_storage::{Database, SqliteLibraryIndexRepository};
use std::sync::atomic::AtomicUsize;
use tokio::sync::Notify;

fn counts(usage: &CapacityUsage) -> (Option<u32>, Option<u32>) {
    let mut snapshot = Snapshot::default();
    usage.apply(&mut snapshot);
    (
        snapshot.capacity.history_used,
        snapshot.capacity.favorites_used,
    )
}
async fn wait(notify: &Notify) {
    tokio::time::timeout(std::time::Duration::from_secs(5), notify.notified())
        .await
        .expect("Capacity totals must publish without UI polling");
}

#[tokio::test]
async fn navigation_totals_follow_real_database_insert_and_delete() {
    let database = Arc::new(Database::in_memory().unwrap());
    let notify = Arc::new(Notify::new());
    let changed = notify.clone();
    let index = Arc::new(LibraryIndex::new(Arc::new(
        SqliteLibraryIndexRepository::new(database.clone()),
    )));
    let usage = CapacityUsage::new(
        index,
        tokio::runtime::Handle::current(),
        Arc::new(move || changed.notify_one()),
    );
    wait(&notify).await;
    assert_eq!(counts(&usage), (Some(0), Some(0)));
    database.with_connection(|connection|{
        connection.execute("INSERT INTO history_records(kind,timestamp) VALUES ('translation','2026-11-09T01:00:00Z')",[])?;
        let id=connection.last_insert_rowid();connection.execute("INSERT INTO translation_history(history_id,source_text,source_lang,target_lang,providers_used,results,duration_ms) VALUES (?1,'text','en','zh-CN','[]','[]',1)",[id])?;
        connection.execute("INSERT INTO favorites(kind,created_at,fingerprint,content_json) VALUES ('translation','2026-11-09T01:00:00Z','fixture','{}')",[])?;
        connection.execute("INSERT INTO screenshot_favorites(created_at,asset_path,thumbnail_path,width,height) VALUES ('2026-11-09T01:00:00Z','image.png','thumb.png',1,1)",[])?;
        Ok(())
    }).unwrap();
    usage.refresh();
    wait(&notify).await;
    assert_eq!(counts(&usage), (Some(1), Some(2)));
    database
        .with_connection(|connection| {
            connection.execute("DELETE FROM history_records", [])?;
            connection.execute("DELETE FROM favorites", [])?;
            Ok(())
        })
        .unwrap();
    usage.refresh();
    wait(&notify).await;
    assert_eq!(counts(&usage), (Some(0), Some(1)));
    database
        .with_connection(|connection| {
            connection.execute("DELETE FROM screenshot_favorites", [])?;
            Ok(())
        })
        .unwrap();
    usage.refresh();
    wait(&notify).await;
    assert_eq!(counts(&usage), (Some(0), Some(0)));
}

struct Delayed {
    calls: AtomicUsize,
    entered: Notify,
    release: Notify,
}
#[async_trait]
impl LibraryIndexRepository for Delayed {
    async fn query_history(
        &self,
        query: &LibraryIndexQuery,
    ) -> snaplingo_core::Result<LibraryIndexPage> {
        assert_eq!(query.limit, 1);
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        if call % 2 == 0 {
            self.entered.notify_one();
            self.release.notified().await;
        }
        Ok(LibraryIndexPage {
            items: vec![],
            total: if call % 2 == 0 { 99 } else { 3 },
        })
    }
    async fn query_favorites(
        &self,
        query: &LibraryIndexQuery,
    ) -> snaplingo_core::Result<LibraryIndexPage> {
        assert_eq!(query.limit, 1);
        Ok(LibraryIndexPage {
            items: vec![],
            total: 4,
        })
    }
}
#[tokio::test]
async fn newer_events_and_disposal_reject_a_delayed_capacity_query() {
    let index = Arc::new(Delayed {
        calls: AtomicUsize::new(0),
        entered: Notify::new(),
        release: Notify::new(),
    });
    let notify = Arc::new(Notify::new());
    let changed = notify.clone();
    let usage = Arc::new(CapacityUsage {
        index: Arc::new(LibraryIndex::new(index.clone())),
        executor: tokio::runtime::Handle::current(),
        revision: AtomicU64::new(0),
        disposed: AtomicBool::new(false),
        totals: Mutex::new(Totals::default()),
        changed: Arc::new(move || changed.notify_one()),
    });
    let old = usage.refresh();
    wait(&index.entered).await;
    usage.refresh().await.unwrap();
    assert_eq!(counts(&usage), (Some(3), Some(4)));
    index.release.notify_one();
    old.await.unwrap();
    assert_eq!(
        counts(&usage),
        (Some(3), Some(4)),
        "A slow older query must not overwrite the latest event"
    );
    let pending = usage.refresh();
    wait(&index.entered).await;
    usage.dispose();
    index.release.notify_one();
    pending.await.unwrap();
    assert_eq!(
        counts(&usage),
        (Some(3), Some(4)),
        "A disposed page must ignore its last count query"
    );
}

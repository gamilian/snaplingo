//! Cached navigation totals loaded on workers through the real library index.
use snaplingo_core::library::index::LibraryIndex;
use snaplingo_core::library::index::LibraryIndexQuery;
use snaplingo_ui::Snapshot;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex,
};

#[derive(Default)]
struct Totals {
    history: Option<u32>,
    favorites: Option<u32>,
}
pub struct CapacityUsage {
    index: Arc<LibraryIndex>,
    revision: AtomicU64,
    disposed: AtomicBool,
    totals: Mutex<Totals>,
    changed: Arc<dyn Fn() + Send + Sync>,
    executor: tokio::runtime::Handle,
}
impl CapacityUsage {
    pub fn new(
        index: Arc<LibraryIndex>,
        executor: tokio::runtime::Handle,
        changed: Arc<dyn Fn() + Send + Sync>,
    ) -> Arc<Self> {
        let usage = Arc::new(Self {
            index,
            revision: AtomicU64::new(0),
            disposed: AtomicBool::new(false),
            totals: Mutex::new(Totals::default()),
            changed,
            executor,
        });
        usage.refresh();
        usage
    }
    pub fn refresh(self: &Arc<Self>) -> tokio::task::JoinHandle<()> {
        let revision = self.revision.fetch_add(1, Ordering::AcqRel) + 1;
        let usage = self.clone();
        self.executor.spawn(async move {
            // LibraryIndex clamps pages to at least one item. Loading one key
            // retains its existing query contract and avoids loading bodies.
            let query = LibraryIndexQuery {
                limit: 1,
                ..Default::default()
            };
            let (history, favorites) = tokio::join!(
                usage.index.query_history(query.clone()),
                usage.index.query_favorites(query)
            );
            let mut totals = usage.totals.lock().unwrap();
            if usage.disposed.load(Ordering::Acquire)
                || usage.revision.load(Ordering::Acquire) != revision
            {
                return;
            }
            totals.history = history
                .ok()
                .map(|page| page.total.min(u32::MAX as usize) as u32);
            totals.favorites = favorites
                .ok()
                .map(|page| page.total.min(u32::MAX as usize) as u32);
            drop(totals);
            (usage.changed)();
        })
    }
    pub fn apply(&self, snapshot: &mut Snapshot) {
        let totals = self.totals.lock().unwrap();
        snapshot.capacity.history_used = totals.history;
        snapshot.capacity.favorites_used = totals.favorites;
    }
    pub fn dispose(&self) {
        self.disposed.store(true, Ordering::Release);
        self.revision.fetch_add(1, Ordering::AcqRel);
    }
}

#[cfg(test)]
mod tests;

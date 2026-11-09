//! Cached navigation totals loaded on workers through the real library index.
use crate::application::library_index::{LibraryIndex, LibraryIndexQuery};
use crate::infrastructure::desktop_events::{DesktopEvent, DesktopEvents, DesktopSubscription};
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
pub(crate) struct CapacityUsage {
    index: Arc<LibraryIndex>,
    revision: AtomicU64,
    disposed: AtomicBool,
    totals: Mutex<Totals>,
    changed: Arc<dyn Fn() + Send + Sync>,
}
impl CapacityUsage {
    pub(crate) fn subscribe(
        index: Arc<LibraryIndex>,
        events: &Arc<DesktopEvents>,
        changed: impl Fn() + Send + Sync + 'static,
    ) -> (Arc<Self>, DesktopSubscription) {
        let usage = Arc::new(Self {
            index,
            revision: AtomicU64::new(0),
            disposed: AtomicBool::new(false),
            totals: Mutex::new(Totals::default()),
            changed: Arc::new(changed),
        });
        let weak = Arc::downgrade(&usage);
        let subscription = events.subscribe(move |event| {
            if matches!(
                event,
                DesktopEvent::HistoryChanged
                    | DesktopEvent::FavoritesChanged
                    | DesktopEvent::ScreenshotFavoritesChanged
            ) {
                if let Some(usage) = weak.upgrade() {
                    usage.refresh();
                }
            }
        });
        usage.refresh();
        (usage, subscription)
    }
    fn refresh(self: &Arc<Self>) -> tokio::task::JoinHandle<()> {
        let revision = self.revision.fetch_add(1, Ordering::AcqRel) + 1;
        let usage = self.clone();
        crate::infrastructure::runtime::spawn(async move {
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
    pub(crate) fn apply(&self, snapshot: &mut Snapshot) {
        let totals = self.totals.lock().unwrap();
        snapshot.capacity.history_used = totals.history;
        snapshot.capacity.favorites_used = totals.favorites;
    }
    pub(crate) fn dispose(&self) {
        self.disposed.store(true, Ordering::Release);
        self.revision.fetch_add(1, Ordering::AcqRel);
    }
}

#[cfg(test)]
mod tests;

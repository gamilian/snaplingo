use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, Mutex};

use base64::Engine;
use chrono::{DateTime, Local, Utc};
use snaplingo_core::library::favorites::FavoriteContent;
use snaplingo_core::library::index as core;
use snaplingo_ui::{library as ui, RasterImage, Snapshot};

#[cfg(test)]
pub(crate) mod tests;

struct State {
    snapshot: ui::LibrarySnapshot,
    records: Vec<core::LibraryRecord>,
    request: u64,
    mutation_worker: bool,
    pending: VecDeque<(ui::LibraryIntent, Option<core::LibraryRecord>)>,
    disposed: bool,
}

pub struct LibrarySession {
    pages: core::LibraryPages,
    clipboard: Arc<dyn core::LibraryClipboard>,
    state: Mutex<State>,
}
impl LibrarySession {
    pub fn new(
        pages: core::LibraryPages,
        clipboard: Arc<dyn core::LibraryClipboard>,
        kind: ui::LibraryKind,
    ) -> Self {
        Self {
            pages,
            clipboard,
            state: Mutex::new(State {
                snapshot: ui::LibrarySnapshot {
                    kind,
                    ..Default::default()
                },
                records: vec![],
                request: 0,
                mutation_worker: false,
                pending: VecDeque::new(),
                disposed: false,
            }),
        }
    }
    pub fn snapshot(&self, shell: Snapshot) -> ui::LibrarySnapshot {
        let mut snapshot = self.state.lock().unwrap().snapshot.clone();
        snapshot.shell = shell;
        snapshot
    }
    pub fn dispose(&self) {
        let mut state = self.state.lock().unwrap();
        state.disposed = true;
        state.request += 1;
    }
    /// Delete and clear intents reach this interface only after shared Settings confirmation.
    /// Clipboard reads are supplied by the view's OS adapter.
    pub fn dispatch(
        self: &Arc<Self>,
        intent: ui::LibraryIntent,
        changed: impl Fn() + Send + Sync + 'static,
    ) -> impl std::future::Future<Output = Result<(), String>> + Send + 'static {
        // Prepare on the UI thread before spawning. Keystrokes and blur saves
        // must retain their order even when worker futures are polled out of order.
        let work = {
            let mut state = self.state.lock().unwrap();
            if state.disposed {
                Work::Skip
            } else {
                match intent {
                    ui::LibraryIntent::Query {
                        revision,
                        kind,
                        query,
                    } => {
                        if state.disposed || revision < state.snapshot.revision {
                            Work::Skip
                        } else {
                            state.snapshot.revision = revision;
                            state.snapshot.kind = kind;
                            state.snapshot.query = query;
                            Work::Query(begin_query(&mut state).unwrap())
                        }
                    }
                    intent => {
                        let key = match &intent {
                            ui::LibraryIntent::Favorite { key, .. }
                            | ui::LibraryIntent::SaveMetadata { key, .. } => Some(*key),
                            ui::LibraryIntent::Delete(key)
                            | ui::LibraryIntent::RerunOcrAndCopy(key)
                            | ui::LibraryIntent::CopyScreenshot(key)
                            | ui::LibraryIntent::RevealScreenshot(key) => Some(*key),
                            _ => None,
                        };
                        let record = key
                            .map(|key| {
                                state
                                    .records
                                    .iter()
                                    .find(|record| record_key(record) == key)
                                    .cloned()
                                    .ok_or_else(|| "该资料库记录已不在当前页面".to_string())
                            })
                            .transpose();
                        match record {
                            Err(error) => Work::Error(error),
                            Ok(record) => {
                                state.pending.push_back((intent, record));
                                if state.mutation_worker {
                                    Work::Skip
                                } else {
                                    state.mutation_worker = true;
                                    Work::Drain
                                }
                            }
                        }
                    }
                }
            }
        };
        let session = self.clone();
        async move {
            match work {
                Work::Skip => Ok(()),
                Work::Error(error) => Err(error),
                Work::Query(query) => session.load(query, &changed).await,
                Work::Drain => {
                    let mut error = None;
                    loop {
                        let next = {
                            let mut state = session.state.lock().unwrap();
                            if state.disposed {
                                state.pending.clear();
                            }
                            let next = state.pending.pop_front();
                            if next.is_none() {
                                state.mutation_worker = false;
                            }
                            next
                        };
                        let Some((intent, record)) = next else {
                            break;
                        };
                        if let Err(value) = session.mutate(intent, record, &changed).await {
                            error = Some(value);
                        }
                    }
                    error.map_or(Ok(()), Err)
                }
            }
        }
    }
    async fn mutate(
        &self,
        intent: ui::LibraryIntent,
        record: Option<core::LibraryRecord>,
        changed: &impl Fn(),
    ) -> Result<(), String> {
        let reload = matches!(
            &intent,
            ui::LibraryIntent::Favorite { .. }
                | ui::LibraryIntent::Delete(_)
                | ui::LibraryIntent::ConfirmClearHistory(_)
                | ui::LibraryIntent::SaveMetadata { .. }
                | ui::LibraryIntent::RerunOcrAndCopy(_)
        );
        let result = match intent {
            ui::LibraryIntent::CopyText(text) => self.clipboard.write_text(&text).await,
            ui::LibraryIntent::Favorite { result, .. } => {
                self.pages.favorite(record.as_ref().unwrap(), result).await
            }
            ui::LibraryIntent::Delete(_) => self.pages.delete(record.as_ref().unwrap()).await,
            ui::LibraryIntent::ConfirmClearHistory(filter) => {
                self.pages.clear_history(filter_value(filter)).await
            }
            ui::LibraryIntent::SaveMetadata { note, tags, .. } => {
                self.pages
                    .update_metadata(record.as_ref().unwrap(), note, tags)
                    .await
            }
            ui::LibraryIntent::RerunOcrAndCopy(_) => {
                self.pages
                    .rerun_ocr_and_copy(record.as_ref().unwrap(), self.clipboard.as_ref())
                    .await
            }
            ui::LibraryIntent::CopyScreenshot(_) => {
                self.pages.copy_screenshot(record.as_ref().unwrap()).await
            }
            ui::LibraryIntent::RevealScreenshot(_) => {
                self.pages.reveal_screenshot(record.as_ref().unwrap()).await
            }
            ui::LibraryIntent::ReadClipboard => Err("读取剪贴板由原生窗口处理".into()),
            ui::LibraryIntent::Query { .. } => unreachable!(),
        }
        .map_err(|e| e.to_string());
        {
            let mut state = self.state.lock().unwrap();
            if state.disposed {
                return result;
            }
            state.snapshot.error = result.as_ref().err().cloned();
        }
        if result.is_ok() && reload {
            self.reload(changed).await?;
        } else {
            changed();
        }
        result
    }
    async fn reload(&self, changed: &impl Fn()) -> Result<(), String> {
        let query = begin_query(&mut self.state.lock().unwrap());
        match query {
            Some(query) => self.load(query, changed).await,
            None => Ok(()),
        }
    }
    async fn load(&self, (token, kind, query): Query, changed: &impl Fn()) -> Result<(), String> {
        {
            let state = self.state.lock().unwrap();
            if state.disposed || state.request != token {
                return Ok(());
            }
        }
        changed();
        let kind = match kind {
            ui::LibraryKind::History => core::LibraryKind::History,
            ui::LibraryKind::Favorites => core::LibraryKind::Favorites,
        };
        let result = self
            .pages
            .query(
                kind,
                &core::LibraryPageRequest {
                    filter: filter_value(query.filter),
                    search: query.search,
                    page: query.page,
                },
            )
            .await
            .map_err(|e| e.to_string());
        // Decode thumbnails outside the snapshot lock so UI reads stay short.
        let result = result.map(|page| {
            let now = Utc::now();
            let items = page
                .items
                .iter()
                .map(|r| project_record(r, &page.favorite_keys, now))
                .collect();
            (page, items)
        });
        let mut state = self.state.lock().unwrap();
        if state.disposed || state.request != token {
            return Ok(());
        }
        state.snapshot.loading = false;
        let outcome = match result {
            Ok((page, items)) => {
                state.snapshot.items = items;
                state.snapshot.total = page.total;
                state.snapshot.query.page = page.page;
                state.records = page.items;
                Ok(())
            }
            Err(error) => {
                state.snapshot.error = Some(error.clone());
                Err(error)
            }
        };
        drop(state);
        changed();
        outcome
    }
}
enum Work {
    Skip,
    Query(Query),
    Drain,
    Error(String),
}
type Query = (u64, ui::LibraryKind, ui::LibraryQuery);
fn begin_query(state: &mut State) -> Option<Query> {
    if state.disposed {
        return None;
    }
    state.request += 1;
    state.snapshot.loading = true;
    state.snapshot.error = None;
    Some((
        state.request,
        state.snapshot.kind,
        state.snapshot.query.clone(),
    ))
}
fn filter_value(filter: ui::LibraryFilter) -> core::LibraryFilter {
    match filter {
        ui::LibraryFilter::All => core::LibraryFilter::All,
        ui::LibraryFilter::Translation => core::LibraryFilter::Translation,
        ui::LibraryFilter::Ocr => core::LibraryFilter::Ocr,
        ui::LibraryFilter::Screenshot => core::LibraryFilter::Screenshot,
    }
}
fn record_key(record: &core::LibraryRecord) -> ui::ItemKey {
    ui::ItemKey {
        id: record.id(),
        kind: match record.kind() {
            core::LibraryIndexKind::Translation => ui::ContentKind::Translation,
            core::LibraryIndexKind::Ocr => ui::ContentKind::Ocr,
            core::LibraryIndexKind::Screenshot => ui::ContentKind::Screenshot,
        },
    }
}
fn image(value: Option<&str>) -> Option<RasterImage> {
    let (_, encoded) = value?.split_once(",")?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .ok()?;
    let image = image::load_from_memory(&bytes).ok()?.to_rgba8();
    RasterImage::from_rgba(image.width(), image.height(), image.into_raw())
}
fn project_record(
    record: &core::LibraryRecord,
    favorites: &HashSet<String>,
    now: DateTime<Utc>,
) -> ui::LibraryItem {
    let keys = record.favorite_keys();
    let translation =
        |source: &str,
         from: &str,
         to: &str,
         results: &[snaplingo_core::providers::translation::model::TranslationResult]| {
            ui::LibraryContent::Translation {
                source: source.into(),
                source_language: from.into(),
                target_language: to.into(),
                results: results
                    .iter()
                    .enumerate()
                    .map(|(i, r)| ui::TranslationResult {
                        provider: r.provider_id.clone(),
                        text: r.translated_text.clone(),
                        favorite: keys.get(i).is_some_and(|k| favorites.contains(k)),
                    })
                    .collect(),
            }
        };
    let (content, time, note, tags, thumbnail) = match record {
        core::LibraryRecord::TranslationHistory(r) => (
            translation(&r.source_text, &r.source_lang, &r.target_lang, &r.results),
            r.timestamp,
            r.note.clone(),
            r.tags.clone(),
            None,
        ),
        core::LibraryRecord::OcrHistory(r) => (
            ui::LibraryContent::Ocr {
                text: r.recognized_text.clone(),
                language: r.language.clone(),
                provider: r.provider_used.clone(),
                confidence: r.confidence.map(|v| v as f32),
                can_rerun: r.source_asset_path.is_some(),
                favorite: keys.iter().any(|k| favorites.contains(k)),
            },
            r.timestamp,
            r.note.clone(),
            r.tags.clone(),
            image(r.thumbnail_data_url.as_deref()),
        ),
        core::LibraryRecord::Screenshot(r) => (
            ui::LibraryContent::Screenshot {
                width: r.width,
                height: r.height,
            },
            r.created_at,
            r.note.clone(),
            r.tags.clone(),
            image(Some(&r.thumbnail_data_url)),
        ),
        core::LibraryRecord::Favorite(r) => {
            let content = match &r.content {
                FavoriteContent::Translation(t) => translation(
                    &t.source_text,
                    &t.source_lang,
                    &t.target_lang,
                    std::slice::from_ref(&t.result),
                ),
                FavoriteContent::Ocr(o) => ui::LibraryContent::Ocr {
                    text: o.recognized_text.clone(),
                    language: o.language.clone(),
                    provider: o.provider_used.clone(),
                    confidence: o.confidence,
                    can_rerun: o.source_asset_path.is_some(),
                    favorite: true,
                },
            };
            (
                content,
                r.created_at,
                r.note.clone(),
                r.tags.clone(),
                image(r.thumbnail_data_url.as_deref()),
            )
        }
    };
    let age = (now - time).num_seconds();
    let local = time.with_timezone(&Local);
    let relative_time = if age < 60 {
        "刚刚".into()
    } else if age < 3600 {
        format!("{} 分钟前", age / 60)
    } else if age < 86400 {
        format!("{} 小时前", age / 3600)
    } else if age < 7 * 86400 {
        format!("{} 天前", age / 86400)
    } else {
        local.format("%Y-%m-%d").to_string()
    };
    ui::LibraryItem {
        key: record_key(record),
        relative_time,
        date: local.format("%Y-%m-%d %H:%M:%S").to_string(),
        content,
        thumbnail,
        note,
        tags,
    }
}

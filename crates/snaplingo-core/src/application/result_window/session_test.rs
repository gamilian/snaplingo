use super::*;
use crate::application::providers::common::Provider;
use crate::application::providers::translation::TranslationProvider;
use crate::infrastructure::storage::{SqliteConfigStore, SqliteHistoryRepository};
use tokio::sync::{mpsc, oneshot};

type Request = (TranslationRequest, oneshot::Sender<TranslationResult>);

struct ControlledProvider {
    id: &'static str,
    requests: mpsc::UnboundedSender<Request>,
}
impl Provider for ControlledProvider {
    fn id(&self) -> &str {
        self.id
    }
    fn name(&self) -> &str {
        self.id
    }
    fn is_configured(&self) -> bool {
        true
    }
    fn requires_api_key(&self) -> bool {
        false
    }
}
#[async_trait::async_trait]
impl TranslationProvider for ControlledProvider {
    async fn translate(&self, request: &TranslationRequest) -> Result<TranslationResult> {
        let (sender, receiver) = oneshot::channel();
        self.requests.send((request.clone(), sender)).unwrap();
        receiver.await.map_err(|error| error.to_string().into())
    }
}

#[derive(Default)]
struct Presentation(Mutex<Vec<ResultWindowSnapshot>>);
impl ResultWindowPresentationPort for Presentation {
    fn publish(&self, view: ResultWindowSnapshot) {
        self.0.lock().unwrap().push(view);
    }
}

fn session(ids: &[&'static str]) -> (Arc<ResultWindowSession>, mpsc::UnboundedReceiver<Request>) {
    let config = Arc::new(SqliteConfigStore::new_temp());
    let settings = Arc::new(SettingsConfiguration::new(config.clone()));
    let translation = Arc::new(TranslationCoordinator::new(config));
    let (sender, receiver) = mpsc::unbounded_channel();
    for id in ids {
        translation
            .register(ControlledProvider {
                id,
                requests: sender.clone(),
            })
            .unwrap();
        translation.activate(id).unwrap();
    }
    let history = Arc::new(History::new(Arc::new(
        SqliteHistoryRepository::new_in_memory().unwrap(),
    )));
    let session = Arc::new(
        ResultWindowSession::new(
            settings,
            translation,
            history,
            Arc::new(Presentation::default()),
        )
        .unwrap(),
    );
    (session, receiver)
}

fn payload(text: &str, auto_translate: bool) -> ResultWindowPayload {
    ResultWindowPayload {
        mode: ResultWindowMode::Translation,
        origin: ResultWindowOrigin::Input,
        text: text.into(),
        auto_translate,
        ocr_intent: None,
        image_base64: None,
        confidence: None,
        detected_language: None,
    }
}
fn success(text: &str) -> TranslationResult {
    TranslationResult::success("fixture", text, Some("en".into()), None)
}
async fn request(receiver: &mut mpsc::UnboundedReceiver<Request>) -> Request {
    tokio::time::timeout(Duration::from_secs(3), receiver.recv())
        .await
        .unwrap()
        .unwrap()
}

#[test]
fn automatic_target_and_swaps_match_translation_language_rules() {
    assert_eq!(resolved_target("hello", "auto", "auto"), "zh-CN");
    assert_eq!(resolved_target("你好", "auto", "zh-TW"), "en");
    assert_eq!(resolved_target("bonjour", "fr", "fr"), "zh-CN");
    let (session, _) = session(&[]);
    session.apply_payload(payload("hello", false)).unwrap();
    session.source_language("zh-TW".into());
    assert_eq!(session.snapshot().target_lang, "en");
    session.target_language("auto".into());
    session.source_language("fr".into());
    assert_eq!(session.snapshot().target_lang, "auto");
    session.source_language("auto".into());
    session.target_language("zh-CN".into());
    session.swap_languages();
    assert_eq!(session.snapshot().source_lang, "zh-CN");
    assert_eq!(session.snapshot().target_lang, "en");
}

#[tokio::test]
async fn plans_are_invalidated_by_edits_close_and_settings_and_wait_for_ime_commit() {
    let (session, mut requests) = session(&["fixture"]);
    session.apply_payload(payload("hello", false)).unwrap();
    let old = session.auto_plan(false).unwrap();
    assert_eq!(old.delay, Duration::from_millis(500));
    session.edit_source("ni".into(), true);
    assert!(!session.plan_is_current(&old));
    assert!(session.auto_plan(true).is_none());
    assert!(session.translate().await.unwrap().is_none());
    session.edit_source("你".into(), false);
    assert!(session.auto_plan(false).is_some());
    session.close();
    assert!(session.auto_plan(true).is_none());
    session.edit_source("ni".into(), true);
    session.apply_payload(payload("hello", true)).unwrap();
    assert!(
        session.auto_plan(true).is_some(),
        "New payload resets marked-text state"
    );
    let stale = session.auto_plan(false).unwrap();
    let mut settings = session.settings.snapshot().unwrap().translation;
    settings.auto_translate = false;
    settings.incremental_translation = true;
    session.settings.update_translation(settings).unwrap();
    session.refresh_settings().unwrap();
    assert!(session.auto_plan(false).is_none());
    assert!(session.translate_plan(stale).await.unwrap().is_none());
    assert!(requests.try_recv().is_err());
    assert_eq!(session.auto_plan(true).unwrap().delay, Duration::ZERO);
}

#[tokio::test]
async fn superseded_and_closed_requests_cannot_publish_record_history_or_auto_copy() {
    let (session, mut requests) = session(&["fixture"]);
    let mut settings = session.settings.snapshot().unwrap().translation;
    settings.auto_copy = true;
    session.settings.update_translation(settings).unwrap();
    session.apply_payload(payload("older", true)).unwrap();
    let older = tokio::spawn({
        let session = session.clone();
        async move { session.translate().await.unwrap() }
    });
    let (_, response) = request(&mut requests).await;
    session.edit_source("newer".into(), false);
    assert!(older.await.unwrap().is_none());
    assert!(
        response.send(success("obsolete")).is_err(),
        "Old network future was cancelled"
    );
    assert!(session
        .history
        .get_translation_history(10, 0)
        .await
        .unwrap()
        .is_empty());
    let newer = tokio::spawn({
        let session = session.clone();
        async move { session.translate().await.unwrap() }
    });
    let (input, response) = request(&mut requests).await;
    assert_eq!(input.text, "newer");
    response.send(success("  最新结果  ")).unwrap();
    let copied = newer.await.unwrap().unwrap();
    assert!(session.auto_copy_is_current(&copied));
    assert_eq!(copied.text, "最新结果");
    assert_eq!(
        session.snapshot().rows[0].result.translated_text,
        "最新结果"
    );
    assert_eq!(
        session
            .history
            .get_translation_history(10, 0)
            .await
            .unwrap()
            .len(),
        1
    );
    let retry = tokio::spawn({
        let session = session.clone();
        async move { session.retry("fixture".into()).await }
    });
    let (_, response) = request(&mut requests).await;
    assert!(
        !session.auto_copy_is_current(&copied),
        "A retry invalidates the previous provider's auto-copy"
    );
    session.close();
    retry.await.unwrap();
    assert!(response.send(success("late retry")).is_err());
    assert!(!session.is_current(copied.epoch));
    assert!(!session.snapshot().visible);
    assert_eq!(
        session.snapshot().rows[0].result.translated_text,
        "最新结果"
    );
}

#[tokio::test]
async fn retry_tokens_and_favorite_markers_follow_the_exact_result() {
    let (session, mut requests) = session(&["fixture"]);
    session.apply_payload(payload("hello", true)).unwrap();
    let translate = tokio::spawn({
        let session = session.clone();
        async move { session.translate().await }
    });
    request(&mut requests)
        .await
        .1
        .send(success("first"))
        .unwrap();
    translate.await.unwrap().unwrap();
    let original = session.snapshot();
    let older = tokio::spawn({
        let session = session.clone();
        async move { session.retry("fixture".into()).await }
    });
    let (_, old_response) = request(&mut requests).await;
    let latest = tokio::spawn({
        let session = session.clone();
        async move { session.retry("fixture".into()).await }
    });
    request(&mut requests)
        .await
        .1
        .send(success("latest"))
        .unwrap();
    latest.await.unwrap();
    old_response.send(success("older")).unwrap();
    older.await.unwrap();
    assert_eq!(session.snapshot().rows[0].result.translated_text, "latest");
    session.mark_favorite(&original, &["fixture".into()]);
    assert!(!session.snapshot().rows[0].favorite);
    session.mark_favorite(&session.snapshot(), &["fixture".into()]);
    assert!(session.snapshot().rows[0].favorite);
    session.apply_payload(payload("", false)).unwrap();
    assert_eq!(session.snapshot().source_text, "hello");
    assert_eq!(session.snapshot().rows[0].result.translated_text, "latest");
    let previous_epoch = session.snapshot().epoch;
    session.apply_payload(payload("new", false)).unwrap();
    session
        .retry_if_current("fixture".into(), previous_epoch)
        .await;
    assert!(
        requests.try_recv().is_err(),
        "Deferred retries must not start against a newer input"
    );
    session.apply_payload(payload("new", true)).unwrap();
    assert!(session.snapshot().rows.is_empty());
}

#[tokio::test]
async fn a_retry_during_the_provider_batch_records_and_copies_the_latest_response() {
    let (session, mut requests) = session(&["first", "second"]);
    let mut settings = session.settings.snapshot().unwrap().translation;
    settings.auto_copy = true;
    session.settings.update_translation(settings).unwrap();
    session.apply_payload(payload("hello", true)).unwrap();
    let task = tokio::spawn({
        let session = session.clone();
        async move { session.translate().await.unwrap() }
    });
    let (_, first) = request(&mut requests).await;
    let (_, second) = request(&mut requests).await;
    first.send(success("obsolete")).unwrap();
    tokio::time::timeout(Duration::from_secs(3), async {
        while !session
            .snapshot()
            .rows
            .iter()
            .any(|row| row.status == TranslationStatus::Success)
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let id = session
        .snapshot()
        .rows
        .iter()
        .find(|row| row.result.translated_text == "obsolete")
        .unwrap()
        .id
        .clone();
    let retry = tokio::spawn({
        let session = session.clone();
        async move { session.retry(id).await }
    });
    request(&mut requests)
        .await
        .1
        .send(success("latest"))
        .unwrap();
    retry.await.unwrap();
    second.send(success("other")).unwrap();
    let copy = task.await.unwrap().unwrap();
    assert_ne!(copy.text, "obsolete");
    assert!(session.auto_copy_is_current(&copy));
    let history = session
        .history
        .get_translation_history(10, 0)
        .await
        .unwrap();
    assert_eq!(history[0].results.len(), 2);
    assert!(history[0]
        .results
        .iter()
        .any(|result| result.translated_text == "latest"));
    assert!(history[0]
        .results
        .iter()
        .all(|result| result.translated_text != "obsolete"));
}

#[tokio::test]
async fn provider_failures_are_retryable_cards_and_history_keeps_original_line_breaks() {
    let (session, mut requests) = session(&["first", "second"]);
    let mut settings = session.settings.snapshot().unwrap().translation;
    settings.preserve_line_breaks = false;
    session.settings.update_translation(settings).unwrap();
    session
        .apply_payload(payload("hello\nworld", false))
        .unwrap();
    let task = tokio::spawn({
        let session = session.clone();
        async move { session.translate().await }
    });
    let (input, response) = request(&mut requests).await;
    assert_eq!(input.text, "hello world");
    assert_eq!(input.target_lang, "zh-CN");
    response.send(success("result")).unwrap();
    request(&mut requests)
        .await
        .1
        .send(TranslationResult::failure(
            "fixture",
            "timeout",
            "network timeout",
            true,
        ))
        .unwrap();
    task.await.unwrap().unwrap();
    let rows = session.snapshot().rows;
    assert_eq!(
        rows.iter()
            .filter(|row| row.status == TranslationStatus::Success)
            .count(),
        1
    );
    assert_eq!(
        rows.iter()
            .filter(|row| row.status == TranslationStatus::Error)
            .count(),
        1
    );
    assert!(!session.snapshot().translating);
    let history = session
        .history
        .get_translation_history(10, 0)
        .await
        .unwrap();
    assert_eq!(history[0].source_text, "hello\nworld");
    assert_eq!(history[0].results.len(), 1);
}

#[test]
fn file_ocr_generations_ignore_completions_after_switch_or_close() {
    let (session, _) = session(&[]);
    let obsolete = session.begin_file_ocr();
    assert!(session.snapshot().dialog_open);
    session.apply_payload(payload("hello", false)).unwrap();
    assert!(!session.ocr_started(obsolete));
    session.finish_file_ocr(
        obsolete,
        Ok((
            crate::domain::ocr::OcrResult::from_text("late", None),
            "image".into(),
        )),
    );
    assert!(session.snapshot().ocr_text.is_empty());
    let current = session.begin_file_ocr();
    assert!(session.ocr_started(current));
    session.finish_file_ocr(
        current,
        Ok((
            crate::domain::ocr::OcrResult::from_text("你 好", Some(0.9)),
            "image".into(),
        )),
    );
    assert_eq!(session.snapshot().ocr_text, "你好");
    assert!(!session.snapshot().ocr_running);
    let original = session.snapshot();
    session.close();
    session.mark_favorite(&original, &[]);
    assert!(!session.snapshot().ocr_favorite);
    session.finish_file_ocr(current, Err("late error".into()));
    assert!(session.snapshot().error.is_none());
}

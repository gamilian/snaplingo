use super::*;
use crate::infrastructure::storage::{
    Database, FilesystemOcrHistoryAssets, SqliteConfigStore, SqliteFavoriteCapacityRepository,
    SqliteFavoriteRepository, SqliteHistoryRepository,
};
use snaplingo_core::application::favorite_capacity::FavoriteCapacity;
use snaplingo_core::application::providers::common::Provider;
use snaplingo_core::application::providers::ocr::ImageFileReader;
use snaplingo_core::application::providers::translation::{
    TranslationCoordinator, TranslationProvider,
};
use snaplingo_core::application::result_window::{
    ResultWindowNotifierPort, ResultWindowOpenRequest, ResultWindowPresentationPort,
    ResultWindowWindowPort,
};
use snaplingo_core::application::{History, SystemTtsHost, SystemTtsVoice, TtsRuntime};
use snaplingo_core::domain::translation::TranslationResult;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};

type Request = (TranslationRequest, oneshot::Sender<TranslationResult>);

struct Translator(mpsc::UnboundedSender<Request>);
impl Provider for Translator {
    fn id(&self) -> &str {
        "fixture"
    }
    fn name(&self) -> &str {
        "Fixture"
    }
    fn is_configured(&self) -> bool {
        true
    }
    fn requires_api_key(&self) -> bool {
        false
    }
}
#[async_trait::async_trait]
impl TranslationProvider for Translator {
    async fn translate(&self, request: &TranslationRequest) -> crate::Result<TranslationResult> {
        let (sender, receiver) = oneshot::channel();
        self.0.send((request.clone(), sender)).unwrap();
        receiver.await.map_err(|error| error.to_string().into())
    }
}
struct Effects {
    picks: mpsc::UnboundedSender<oneshot::Sender<Option<PathBuf>>>,
    copies: mpsc::UnboundedSender<String>,
}
#[async_trait::async_trait]
impl ResultWindowEffects for Effects {
    async fn pick_image(&self) -> crate::Result<Option<PathBuf>> {
        let (sender, receiver) = oneshot::channel();
        self.picks.send(sender).unwrap();
        receiver.await.map_err(|error| error.to_string().into())
    }
    fn copy_text(&self, text: String) -> crate::Result<()> {
        self.copies
            .send(text)
            .map_err(|error| error.to_string().into())
    }
}
struct Host;
#[async_trait::async_trait]
impl ResultWindowWindowPort for Host {
    async fn show_or_create(&self) -> crate::Result<()> {
        Ok(())
    }
}
#[async_trait::async_trait]
impl ResultWindowNotifierPort for Host {
    async fn notify_payload_ready(&self, _: ResultWindowRequestId) -> crate::Result<()> {
        Ok(())
    }
}
impl ResultWindowPresentationPort for Host {
    fn publish(&self, _: ResultWindowSnapshot) {}
}
#[async_trait::async_trait]
impl SystemTtsHost for Host {
    async fn list_voices(&self) -> crate::Result<Vec<SystemTtsVoice>> {
        Ok(vec![])
    }
    async fn speak(&self, _: &str, _: &str, _: u16) -> crate::Result<()> {
        Ok(())
    }
}
#[derive(Default)]
struct Reader(AtomicUsize);
#[async_trait::async_trait]
impl ImageFileReader for Reader {
    async fn read(&self, _: &str) -> crate::Result<Vec<u8>> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Err("Unexpected stale image read".into())
    }
}
struct Fixture {
    controller: ResultWindowController,
    window: Arc<ResultWindowRuntime>,
    requests: mpsc::UnboundedReceiver<Request>,
    picks: mpsc::UnboundedReceiver<oneshot::Sender<Option<PathBuf>>>,
    copies: mpsc::UnboundedReceiver<String>,
    reader: Arc<Reader>,
    history: Arc<History>,
    _directory: tempfile::TempDir,
}
fn fixture() -> Fixture {
    let database = Arc::new(Database::in_memory().unwrap());
    let config = Arc::new(SqliteConfigStore::new(database.clone()));
    let settings = Arc::new(snaplingo_core::application::SettingsConfiguration::new(
        config.clone(),
    ));
    let mut translation_settings = settings.snapshot().unwrap().translation;
    translation_settings.auto_translate = true;
    translation_settings.incremental_translation = true;
    translation_settings.auto_copy = true;
    settings.update_translation(translation_settings).unwrap();
    let translation = Arc::new(TranslationCoordinator::new(config.clone()));
    let (requests_tx, requests) = mpsc::unbounded_channel();
    translation.register(Translator(requests_tx)).unwrap();
    translation.activate("fixture").unwrap();
    let history = Arc::new(History::new(Arc::new(SqliteHistoryRepository::new(
        database.clone(),
    ))));
    let session = Arc::new(
        ResultWindowSession::new(
            settings.clone(),
            translation,
            history.clone(),
            Arc::new(Host),
        )
        .unwrap(),
    );
    let window = Arc::new(ResultWindowRuntime::new(Arc::new(Host), Arc::new(Host)));
    let ocr = Arc::new(snaplingo_core::application::providers::ocr::OcrCoordinator::new(config));
    let reader = Arc::new(Reader::default());
    let directory = tempfile::tempdir().unwrap();
    let favorites = Arc::new(snaplingo_core::application::Favorites::new(
        Arc::new(SqliteFavoriteRepository::new(database.clone())),
        Arc::new(FilesystemOcrHistoryAssets::new(
            directory.path().to_path_buf(),
        )),
        Arc::new(FavoriteCapacity::new(
            Arc::new(SqliteFavoriteCapacityRepository::new(database)),
            settings.clone(),
        )),
    ));
    let (picks_tx, picks) = mpsc::unbounded_channel();
    let (copies_tx, copies) = mpsc::unbounded_channel();
    let controller = ResultWindowController::new(Services {
        window: Arc::downgrade(&window),
        session,
        settings: settings.clone(),
        file_ocr: Arc::new(snaplingo_core::application::providers::ocr::FileOcr::new(
            reader.clone(),
            ocr.clone(),
        )),
        ocr,
        favorites,
        tts: Arc::new(TtsRuntime::new(Arc::new(Host), settings)),
        effects: Arc::new(Effects {
            picks: picks_tx,
            copies: copies_tx,
        }),
    });
    Fixture {
        controller,
        window,
        requests,
        picks,
        copies,
        reader,
        history,
        _directory: directory,
    }
}
async fn next<T>(receiver: &mut mpsc::UnboundedReceiver<T>) -> T {
    tokio::time::timeout(Duration::from_secs(3), receiver.recv())
        .await
        .unwrap()
        .unwrap()
}
async fn wait_until(mut condition: impl FnMut() -> bool) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while !condition() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
}
async fn open(fixture: &Fixture, request: ResultWindowOpenRequest) -> ResultWindowRequestId {
    let id = fixture.window.reserve_request().unwrap();
    fixture.window.open_reserved(id, request).await.unwrap();
    id
}
fn success(text: &str) -> TranslationResult {
    TranslationResult::success("fixture", text, Some("en".into()), None)
}

#[tokio::test]
async fn latest_handoff_runs_translation_history_and_auto_copy_through_the_controller() {
    let mut f = fixture();
    let old = open(
        &f,
        ResultWindowOpenRequest::selection_translation("older".into()),
    )
    .await;
    let latest = open(
        &f,
        ResultWindowOpenRequest::selection_translation("latest".into()),
    )
    .await;
    f.controller.receive_payload(old).unwrap();
    assert!(f.requests.try_recv().is_err());
    f.controller.receive_payload(latest).unwrap();
    let (request, response) = next(&mut f.requests).await;
    assert_eq!(request.text, "latest");
    response.send(success("  最新译文  ")).unwrap();
    assert_eq!(next(&mut f.copies).await, "最新译文");
    assert_eq!(
        f.controller.snapshot().rows[0].result.translated_text,
        "最新译文"
    );
    assert_eq!(
        f.history
            .get_translation_history(10, 0)
            .await
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn marked_input_debounces_only_the_commit_and_close_cancels_work_and_copy() {
    let mut f = fixture();
    let id = open(&f, ResultWindowOpenRequest::manual_translation("".into())).await;
    f.controller.receive_payload(id).unwrap();
    f.controller.dispatch(Action::Edit("ni".into(), true));
    assert!(
        tokio::time::timeout(Duration::from_millis(190), f.requests.recv())
            .await
            .is_err()
    );
    f.controller.dispatch(Action::Edit("你".into(), false));
    tokio::time::sleep(Duration::from_millis(50)).await;
    f.controller.dispatch(Action::Edit("你好".into(), false));
    assert!(
        tokio::time::timeout(Duration::from_millis(80), f.requests.recv())
            .await
            .is_err()
    );
    let (request, response) = next(&mut f.requests).await;
    assert_eq!(request.text, "你好");
    f.controller.dispatch(Action::Close);
    wait_until(|| response.is_closed()).await;
    assert!(response.send(success("must not copy")).is_err());
    assert!(f.copies.try_recv().is_err());
    assert!(f
        .history
        .get_translation_history(10, 0)
        .await
        .unwrap()
        .is_empty());
    assert!(!f.controller.snapshot().visible);

    let id = open(
        &f,
        ResultWindowOpenRequest::manual_translation("new input".into()),
    )
    .await;
    f.controller.receive_payload(id).unwrap();
    let mut preferences = f
        .controller
        .services
        .settings
        .snapshot()
        .unwrap()
        .translation;
    preferences.auto_translate = false;
    f.controller
        .services
        .settings
        .update_translation(preferences)
        .unwrap();
    f.controller.settings_changed();
    assert!(
        tokio::time::timeout(Duration::from_millis(190), f.requests.recv())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn cancelled_and_superseded_file_pickers_do_not_start_ocr_or_change_new_content() {
    let mut f = fixture();
    let id = open(&f, ResultWindowOpenRequest::file_ocr()).await;
    f.controller.receive_payload(id).unwrap();
    let picker = next(&mut f.picks).await;
    assert!(f.controller.snapshot().dialog_open);
    picker.send(None).unwrap();
    wait_until(|| !f.controller.snapshot().dialog_open).await;
    assert!(!f.controller.snapshot().ocr_running);

    f.controller.dispatch(Action::FileOcr);
    let picker = next(&mut f.picks).await;
    let id = open(
        &f,
        ResultWindowOpenRequest::display_ocr("new OCR result".into()),
    )
    .await;
    f.controller.receive_payload(id).unwrap();
    picker.send(Some(PathBuf::from("stale.png"))).unwrap();
    wait_until(|| !f.controller.snapshot().dialog_open).await;
    tokio::time::sleep(Duration::from_millis(25)).await;
    assert_eq!(f.reader.0.load(Ordering::SeqCst), 0);
    assert_eq!(f.controller.snapshot().ocr_text, "new OCR result");
    assert!(f.controller.snapshot().error.is_none());
}

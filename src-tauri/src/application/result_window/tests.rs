#![allow(dead_code)]

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tokio::sync::Notify;

use super::{
    ResultWindowClipboardPort, ResultWindowMode, ResultWindowNotifierPort, ResultWindowOcrIntent,
    ResultWindowOpenRequest, ResultWindowPayload, ResultWindowRequestId, ResultWindowRuntime,
    ResultWindowWindowPort,
};

#[derive(Clone, Debug)]
enum WindowOpenOutcome {
    Succeeds,
    Fails(String),
    BlocksThenSucceeds,
    BlocksThenFails(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum PresentationEvent {
    WindowOpened,
    PayloadReadyNotified,
}

#[derive(Clone, Debug)]
enum NotificationOutcome {
    Succeeds,
    Fails(String),
    BlocksThenSucceeds,
}

impl From<std::result::Result<(), String>> for NotificationOutcome {
    fn from(outcome: std::result::Result<(), String>) -> Self {
        match outcome {
            Ok(()) => Self::Succeeds,
            Err(message) => Self::Fails(message),
        }
    }
}

struct FakeResultWindow {
    open_outcomes: Mutex<VecDeque<WindowOpenOutcome>>,
    open_calls: Mutex<usize>,
    events: Arc<Mutex<Vec<PresentationEvent>>>,
    opening_started: Notify,
    unblock_open: Notify,
}

impl FakeResultWindow {
    fn new(
        open_outcomes: impl IntoIterator<Item = WindowOpenOutcome>,
        events: Arc<Mutex<Vec<PresentationEvent>>>,
    ) -> Self {
        Self {
            open_outcomes: Mutex::new(open_outcomes.into_iter().collect()),
            open_calls: Mutex::new(0),
            events,
            opening_started: Notify::new(),
            unblock_open: Notify::new(),
        }
    }

    fn open_calls(&self) -> usize {
        *self.open_calls.lock().unwrap()
    }

    async fn wait_until_opening_starts(&self) {
        self.opening_started.notified().await;
    }

    fn unblock_open(&self) {
        self.unblock_open.notify_one();
    }

    fn events(&self) -> Vec<PresentationEvent> {
        self.events.lock().unwrap().clone()
    }
}

#[async_trait]
impl ResultWindowWindowPort for FakeResultWindow {
    async fn show_or_create(&self) -> crate::Result<()> {
        *self.open_calls.lock().unwrap() += 1;
        self.events
            .lock()
            .unwrap()
            .push(PresentationEvent::WindowOpened);
        self.opening_started.notify_one();

        let outcome = self
            .open_outcomes
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(WindowOpenOutcome::Succeeds);

        match outcome {
            WindowOpenOutcome::Succeeds => Ok(()),
            WindowOpenOutcome::Fails(message) => Err(message.into()),
            WindowOpenOutcome::BlocksThenSucceeds => {
                self.unblock_open.notified().await;
                Ok(())
            }
            WindowOpenOutcome::BlocksThenFails(message) => {
                self.unblock_open.notified().await;
                Err(message.into())
            }
        }
    }
}

struct FakeClipboard {
    outcome: std::result::Result<String, String>,
    reads: Mutex<usize>,
}

impl FakeClipboard {
    fn new(text: &str) -> Self {
        Self::with_outcome(Ok(text.to_string()))
    }

    fn with_read_error(message: &str) -> Self {
        Self::with_outcome(Err(message.to_string()))
    }

    fn with_outcome(outcome: std::result::Result<String, String>) -> Self {
        Self {
            outcome,
            reads: Mutex::new(0),
        }
    }

    fn reads(&self) -> usize {
        *self.reads.lock().unwrap()
    }
}

#[async_trait]
impl ResultWindowClipboardPort for FakeClipboard {
    async fn read_text(&self) -> crate::Result<String> {
        *self.reads.lock().unwrap() += 1;
        self.outcome.clone().map_err(Into::into)
    }
}

struct BlockingClipboard {
    text: String,
    reading_started: Notify,
    unblock_read: Notify,
}

impl BlockingClipboard {
    fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            reading_started: Notify::new(),
            unblock_read: Notify::new(),
        }
    }

    async fn wait_until_reading_starts(&self) {
        self.reading_started.notified().await;
    }

    fn unblock_read(&self) {
        self.unblock_read.notify_one();
    }
}

#[async_trait]
impl ResultWindowClipboardPort for BlockingClipboard {
    async fn read_text(&self) -> crate::Result<String> {
        self.reading_started.notify_one();
        self.unblock_read.notified().await;
        Ok(self.text.clone())
    }
}

struct FakePayloadNotifier {
    outcomes: Mutex<VecDeque<NotificationOutcome>>,
    notification_ids: Mutex<Vec<ResultWindowRequestId>>,
    events: Arc<Mutex<Vec<PresentationEvent>>>,
    notification_started: Notify,
    unblock_notification: Notify,
}

impl FakePayloadNotifier {
    fn new(
        outcomes: impl IntoIterator<Item = std::result::Result<(), String>>,
        events: Arc<Mutex<Vec<PresentationEvent>>>,
    ) -> Self {
        Self {
            outcomes: Mutex::new(outcomes.into_iter().map(Into::into).collect()),
            notification_ids: Mutex::new(Vec::new()),
            events,
            notification_started: Notify::new(),
            unblock_notification: Notify::new(),
        }
    }

    fn with_outcomes(
        outcomes: impl IntoIterator<Item = NotificationOutcome>,
        events: Arc<Mutex<Vec<PresentationEvent>>>,
    ) -> Self {
        Self {
            outcomes: Mutex::new(outcomes.into_iter().collect()),
            notification_ids: Mutex::new(Vec::new()),
            events,
            notification_started: Notify::new(),
            unblock_notification: Notify::new(),
        }
    }

    fn notifications(&self) -> usize {
        self.notification_ids.lock().unwrap().len()
    }

    fn notification_ids(&self) -> Vec<ResultWindowRequestId> {
        self.notification_ids.lock().unwrap().clone()
    }

    async fn wait_until_notification_starts(&self) -> ResultWindowRequestId {
        self.notification_started.notified().await;
        *self.notification_ids.lock().unwrap().last().unwrap()
    }

    fn unblock_notification(&self) {
        self.unblock_notification.notify_one();
    }
}

#[async_trait]
impl ResultWindowNotifierPort for FakePayloadNotifier {
    async fn notify_payload_ready(&self, request_id: ResultWindowRequestId) -> crate::Result<()> {
        self.notification_ids.lock().unwrap().push(request_id);
        self.events
            .lock()
            .unwrap()
            .push(PresentationEvent::PayloadReadyNotified);
        self.notification_started.notify_one();

        let outcome = self
            .outcomes
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(NotificationOutcome::Succeeds);

        match outcome {
            NotificationOutcome::Succeeds => Ok(()),
            NotificationOutcome::Fails(message) => Err(message.into()),
            NotificationOutcome::BlocksThenSucceeds => {
                self.unblock_notification.notified().await;
                Ok(())
            }
        }
    }
}

fn translation_request(text: &str) -> ResultWindowOpenRequest {
    ResultWindowOpenRequest::Translation {
        text: text.to_string(),
        auto_translate: false,
    }
}

fn translation_payload(text: &str, auto_translate: bool) -> ResultWindowPayload {
    ResultWindowPayload {
        mode: ResultWindowMode::Translation,
        text: text.to_string(),
        auto_translate,
        ocr_intent: None,
        image_base64: None,
    }
}

fn ocr_payload(
    text: &str,
    intent: ResultWindowOcrIntent,
    image_base64: Option<&str>,
) -> ResultWindowPayload {
    ResultWindowPayload {
        mode: ResultWindowMode::Ocr,
        text: text.to_string(),
        auto_translate: false,
        ocr_intent: Some(intent),
        image_base64: image_base64.map(str::to_string),
    }
}

fn make_runtime(
    window_outcomes: impl IntoIterator<Item = WindowOpenOutcome>,
    notification_outcomes: impl IntoIterator<Item = std::result::Result<(), String>>,
) -> (
    Arc<ResultWindowRuntime>,
    Arc<FakeResultWindow>,
    Arc<FakeClipboard>,
    Arc<FakePayloadNotifier>,
) {
    make_runtime_with_clipboard(
        window_outcomes,
        notification_outcomes,
        Arc::new(FakeClipboard::new("clipboard text")),
    )
}

fn make_runtime_with_clipboard(
    window_outcomes: impl IntoIterator<Item = WindowOpenOutcome>,
    notification_outcomes: impl IntoIterator<Item = std::result::Result<(), String>>,
    clipboard: Arc<FakeClipboard>,
) -> (
    Arc<ResultWindowRuntime>,
    Arc<FakeResultWindow>,
    Arc<FakeClipboard>,
    Arc<FakePayloadNotifier>,
) {
    let events = Arc::new(Mutex::new(Vec::new()));
    let window = Arc::new(FakeResultWindow::new(window_outcomes, events.clone()));
    let notifier = Arc::new(FakePayloadNotifier::new(notification_outcomes, events));
    let runtime = Arc::new(ResultWindowRuntime::new(
        window.clone(),
        clipboard.clone(),
        notifier.clone(),
    ));

    (runtime, window, clipboard, notifier)
}

#[tokio::test]
async fn open_reads_clipboard_stores_input_translation_payload_opens_window_and_notifies() {
    let (runtime, window, clipboard, notifier) = make_runtime([], []);

    runtime
        .open(ResultWindowOpenRequest::InputTranslation)
        .await
        .unwrap();

    assert_eq!(clipboard.reads(), 1);
    assert_eq!(window.open_calls(), 1);
    assert_eq!(notifier.notifications(), 1);
    assert_eq!(
        runtime
            .take_if_current(notifier.notification_ids()[0])
            .unwrap(),
        Some(translation_payload("clipboard text", true))
    );
}

#[tokio::test]
async fn input_translation_uses_empty_text_when_clipboard_read_fails() {
    let (runtime, window, clipboard, notifier) = make_runtime_with_clipboard(
        [],
        [],
        Arc::new(FakeClipboard::with_read_error("clipboard unavailable")),
    );

    runtime
        .open(ResultWindowOpenRequest::InputTranslation)
        .await
        .unwrap();

    assert_eq!(clipboard.reads(), 1);
    assert_eq!(window.open_calls(), 1);
    assert_eq!(notifier.notifications(), 1);
    assert_eq!(
        runtime
            .take_if_current(notifier.notification_ids()[0])
            .unwrap(),
        Some(translation_payload("", true))
    );
}

#[tokio::test]
async fn newer_input_translation_invalidates_an_older_pending_payload_while_clipboard_is_pending() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let window = Arc::new(FakeResultWindow::new([], events.clone()));
    let clipboard = Arc::new(BlockingClipboard::new("new clipboard text"));
    let notifier = Arc::new(FakePayloadNotifier::new([], events));
    let runtime = Arc::new(ResultWindowRuntime::new(
        window,
        clipboard.clone(),
        notifier.clone(),
    ));

    runtime.open(translation_request("older")).await.unwrap();

    let newer_runtime = runtime.clone();
    let newer_open = tokio::spawn(async move {
        newer_runtime
            .open(ResultWindowOpenRequest::InputTranslation)
            .await
    });
    clipboard.wait_until_reading_starts().await;

    assert_eq!(
        runtime
            .take_if_current(notifier.notification_ids()[0])
            .unwrap(),
        None
    );

    clipboard.unblock_read();
    newer_open.await.unwrap().unwrap();

    assert_eq!(
        runtime
            .take_if_current(notifier.notification_ids()[1])
            .unwrap(),
        Some(translation_payload("new clipboard text", true))
    );
}

#[tokio::test]
async fn open_stores_the_pending_payload_before_opening_and_notifies_afterward() {
    let (runtime, window, _clipboard, notifier) =
        make_runtime([WindowOpenOutcome::BlocksThenSucceeds], []);
    let opening_runtime = runtime.clone();

    let opening =
        tokio::spawn(async move { opening_runtime.open(translation_request("hello")).await });
    window.wait_until_opening_starts().await;

    window.unblock_open();
    opening.await.unwrap().unwrap();

    assert_eq!(
        window.events(),
        vec![
            PresentationEvent::WindowOpened,
            PresentationEvent::PayloadReadyNotified,
        ]
    );
    assert_eq!(
        runtime
            .take_if_current(notifier.notification_ids()[0])
            .unwrap(),
        Some(translation_payload("hello", false))
    );
}

#[tokio::test]
async fn take_transfers_the_pending_payload_once() {
    let (runtime, _window, _clipboard, notifier) = make_runtime([], []);
    let payload = translation_payload("hello", false);

    runtime.open(translation_request("hello")).await.unwrap();

    let request_id = notifier.notification_ids()[0];
    assert_eq!(runtime.take_if_current(request_id).unwrap(), Some(payload));
    assert_eq!(runtime.take_if_current(request_id).unwrap(), None);
}

#[tokio::test]
async fn newer_open_request_replaces_an_untaken_payload() {
    let (runtime, _window, _clipboard, notifier) = make_runtime([], []);

    runtime.open(translation_request("older")).await.unwrap();
    runtime.open(translation_request("newer")).await.unwrap();

    assert_eq!(
        runtime
            .take_if_current(notifier.notification_ids()[1])
            .unwrap(),
        Some(translation_payload("newer", false))
    );
}

#[tokio::test]
async fn ocr_open_requests_preserve_each_intent_and_source_image() {
    let cases = [
        (
            ResultWindowOpenRequest::Ocr {
                text: String::new(),
                intent: ResultWindowOcrIntent::Show,
                image_base64: None,
            },
            ocr_payload("", ResultWindowOcrIntent::Show, None),
        ),
        (
            ResultWindowOpenRequest::Ocr {
                text: "recognized text".to_string(),
                intent: ResultWindowOcrIntent::DisplayText,
                image_base64: Some("source-image".to_string()),
            },
            ocr_payload(
                "recognized text",
                ResultWindowOcrIntent::DisplayText,
                Some("source-image"),
            ),
        ),
        (
            ResultWindowOpenRequest::Ocr {
                text: String::new(),
                intent: ResultWindowOcrIntent::File,
                image_base64: None,
            },
            ocr_payload("", ResultWindowOcrIntent::File, None),
        ),
    ];

    for (request, expected_payload) in cases {
        let (runtime, _window, _clipboard, notifier) = make_runtime([], []);

        runtime.open(request).await.unwrap();

        assert_eq!(
            runtime
                .take_if_current(notifier.notification_ids()[0])
                .unwrap(),
            Some(expected_payload)
        );
    }
}

#[tokio::test]
async fn failed_open_removes_its_still_current_payload() {
    let (runtime, _window, _clipboard, notifier) = make_runtime(
        [WindowOpenOutcome::Fails("window unavailable".to_string())],
        [],
    );

    let error = runtime
        .open(translation_request("unavailable"))
        .await
        .unwrap_err();

    assert!(error.to_string().contains("window unavailable"));
    assert_eq!(notifier.notifications(), 0);
    assert_eq!(
        runtime.take_if_current(ResultWindowRequestId(1)).unwrap(),
        None
    );
}

#[tokio::test]
async fn failed_notification_retains_the_pending_payload() {
    let (runtime, window, _clipboard, notifier) =
        make_runtime([], [Err("notification unavailable".to_string())]);

    let error = runtime
        .open(translation_request("still pending"))
        .await
        .unwrap_err();

    assert!(error.to_string().contains("notification unavailable"));
    assert_eq!(window.open_calls(), 1);
    assert_eq!(notifier.notifications(), 1);
    assert_eq!(
        runtime
            .take_if_current(notifier.notification_ids()[0])
            .unwrap(),
        Some(translation_payload("still pending", false))
    );
}

#[tokio::test]
async fn newer_request_survives_an_older_concurrent_open_failure() {
    let (runtime, window, _clipboard, notifier) = make_runtime(
        [
            WindowOpenOutcome::BlocksThenFails("older open failed".to_string()),
            WindowOpenOutcome::Succeeds,
        ],
        [],
    );
    let older_runtime = runtime.clone();

    // The payloads are deliberately identical: failed-open cleanup must use a
    // monotonic request identity rather than payload equality.
    let older_open =
        tokio::spawn(async move { older_runtime.open(translation_request("same text")).await });
    window.wait_until_opening_starts().await;

    runtime
        .open(translation_request("same text"))
        .await
        .unwrap();
    window.unblock_open();

    let error = older_open.await.unwrap().unwrap_err();
    assert!(error.to_string().contains("older open failed"));
    assert_eq!(
        runtime
            .take_if_current(notifier.notification_ids()[0])
            .unwrap(),
        Some(translation_payload("same text", false))
    );
}

#[tokio::test]
async fn older_successful_open_does_not_notify_after_a_newer_request_starts() {
    let (runtime, window, _clipboard, notifier) = make_runtime(
        [
            WindowOpenOutcome::BlocksThenSucceeds,
            WindowOpenOutcome::Succeeds,
        ],
        [],
    );
    let older_runtime = runtime.clone();

    let older_open =
        tokio::spawn(async move { older_runtime.open(translation_request("older")).await });
    window.wait_until_opening_starts().await;

    runtime.open(translation_request("newer")).await.unwrap();
    window.unblock_open();
    older_open.await.unwrap().unwrap();

    assert_eq!(notifier.notifications(), 1);
    assert_eq!(
        runtime
            .take_if_current(notifier.notification_ids()[0])
            .unwrap(),
        Some(translation_payload("newer", false))
    );
}

#[tokio::test]
async fn stale_notification_cannot_take_a_newer_payload() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let window = Arc::new(FakeResultWindow::new(
        [
            WindowOpenOutcome::Succeeds,
            WindowOpenOutcome::BlocksThenSucceeds,
        ],
        events.clone(),
    ));
    let clipboard = Arc::new(FakeClipboard::new("clipboard text"));
    let notifier = Arc::new(FakePayloadNotifier::with_outcomes(
        [
            NotificationOutcome::BlocksThenSucceeds,
            NotificationOutcome::Succeeds,
        ],
        events,
    ));
    let runtime = Arc::new(ResultWindowRuntime::new(
        window.clone(),
        clipboard,
        notifier.clone(),
    ));

    let older_runtime = runtime.clone();
    let older_open =
        tokio::spawn(async move { older_runtime.open(translation_request("older")).await });
    window.wait_until_opening_starts().await;
    let older_request_id = notifier.wait_until_notification_starts().await;

    let newer_runtime = runtime.clone();
    let newer_open =
        tokio::spawn(async move { newer_runtime.open(translation_request("newer")).await });
    window.wait_until_opening_starts().await;

    notifier.unblock_notification();
    older_open.await.unwrap().unwrap();

    assert_eq!(runtime.take_if_current(older_request_id).unwrap(), None);

    window.unblock_open();
    newer_open.await.unwrap().unwrap();

    let notification_ids = notifier.notification_ids();
    assert_eq!(notification_ids.len(), 2);
    assert_eq!(
        runtime.take_if_current(notification_ids[1]).unwrap(),
        Some(translation_payload("newer", false))
    );
}

#[tokio::test]
async fn late_subscriber_bootstraps_from_the_current_pending_request_id() {
    let (runtime, _window, _clipboard, _notifier) = make_runtime([], []);

    runtime
        .open(translation_request("missed event"))
        .await
        .unwrap();

    let request_id = runtime.current_request_id().unwrap().unwrap();
    assert_eq!(
        runtime.take_if_current(request_id).unwrap(),
        Some(translation_payload("missed event", false))
    );
}

#[tokio::test]
async fn stale_bootstrap_id_cannot_take_a_newer_pending_payload() {
    let (runtime, _window, _clipboard, _notifier) = make_runtime([], []);

    runtime.open(translation_request("older")).await.unwrap();
    let older_request_id = runtime.current_request_id().unwrap().unwrap();

    runtime.open(translation_request("newer")).await.unwrap();

    assert_eq!(runtime.take_if_current(older_request_id).unwrap(), None);

    let newer_request_id = runtime.current_request_id().unwrap().unwrap();
    assert_eq!(
        runtime.take_if_current(newer_request_id).unwrap(),
        Some(translation_payload("newer", false))
    );
}

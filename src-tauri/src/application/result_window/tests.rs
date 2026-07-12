#![allow(dead_code)]

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tokio::sync::Notify;

use super::{
    ResultWindowClipboardPort, ResultWindowMode, ResultWindowNotifierPort, ResultWindowOpenRequest,
    ResultWindowPayload, ResultWindowRuntime, ResultWindowWindowPort,
};

#[derive(Clone, Debug)]
enum WindowOpenOutcome {
    Succeeds,
    Fails(String),
    BlocksThenFails(String),
}

struct FakeResultWindow {
    open_outcomes: Mutex<VecDeque<WindowOpenOutcome>>,
    open_calls: Mutex<usize>,
    opening_started: Notify,
    unblock_open: Notify,
}

impl FakeResultWindow {
    fn new(open_outcomes: impl IntoIterator<Item = WindowOpenOutcome>) -> Self {
        Self {
            open_outcomes: Mutex::new(open_outcomes.into_iter().collect()),
            open_calls: Mutex::new(0),
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
}

#[async_trait]
impl ResultWindowWindowPort for FakeResultWindow {
    async fn show_or_create(&self) -> crate::Result<()> {
        *self.open_calls.lock().unwrap() += 1;
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
            WindowOpenOutcome::BlocksThenFails(message) => {
                self.unblock_open.notified().await;
                Err(message.into())
            }
        }
    }
}

struct FakeClipboard {
    text: String,
    reads: Mutex<usize>,
}

impl FakeClipboard {
    fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
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
        Ok(self.text.clone())
    }
}

struct FakePayloadNotifier {
    outcomes: Mutex<VecDeque<std::result::Result<(), String>>>,
    notifications: Mutex<usize>,
}

impl FakePayloadNotifier {
    fn new(outcomes: impl IntoIterator<Item = std::result::Result<(), String>>) -> Self {
        Self {
            outcomes: Mutex::new(outcomes.into_iter().collect()),
            notifications: Mutex::new(0),
        }
    }

    fn notifications(&self) -> usize {
        *self.notifications.lock().unwrap()
    }
}

#[async_trait]
impl ResultWindowNotifierPort for FakePayloadNotifier {
    async fn notify_payload_ready(&self) -> crate::Result<()> {
        *self.notifications.lock().unwrap() += 1;
        self.outcomes
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(Ok(()))
            .map_err(Into::into)
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

fn make_runtime(
    window_outcomes: impl IntoIterator<Item = WindowOpenOutcome>,
    notification_outcomes: impl IntoIterator<Item = std::result::Result<(), String>>,
) -> (
    Arc<ResultWindowRuntime>,
    Arc<FakeResultWindow>,
    Arc<FakeClipboard>,
    Arc<FakePayloadNotifier>,
) {
    let window = Arc::new(FakeResultWindow::new(window_outcomes));
    let clipboard = Arc::new(FakeClipboard::new("clipboard text"));
    let notifier = Arc::new(FakePayloadNotifier::new(notification_outcomes));
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
        runtime.take().unwrap(),
        Some(translation_payload("clipboard text", true))
    );
}

#[tokio::test]
async fn take_transfers_the_pending_payload_once() {
    let (runtime, _window, _clipboard, _notifier) = make_runtime([], []);
    let payload = translation_payload("hello", false);

    runtime.open(translation_request("hello")).await.unwrap();

    assert_eq!(runtime.take().unwrap(), Some(payload));
    assert_eq!(runtime.take().unwrap(), None);
}

#[tokio::test]
async fn newer_open_request_replaces_an_untaken_payload() {
    let (runtime, _window, _clipboard, _notifier) = make_runtime([], []);

    runtime.open(translation_request("older")).await.unwrap();
    runtime.open(translation_request("newer")).await.unwrap();

    assert_eq!(
        runtime.take().unwrap(),
        Some(translation_payload("newer", false))
    );
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
    assert_eq!(runtime.take().unwrap(), None);
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
        runtime.take().unwrap(),
        Some(translation_payload("still pending", false))
    );
}

#[tokio::test]
async fn newer_request_survives_an_older_concurrent_open_failure() {
    let (runtime, window, _clipboard, _notifier) = make_runtime(
        [
            WindowOpenOutcome::BlocksThenFails("older open failed".to_string()),
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

    let error = older_open.await.unwrap().unwrap_err();
    assert!(error.to_string().contains("older open failed"));
    assert_eq!(
        runtime.take().unwrap(),
        Some(translation_payload("newer", false))
    );
}

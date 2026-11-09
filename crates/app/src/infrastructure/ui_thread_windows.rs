//! Native windows share the application-owned winit event loop and UI thread.
use snaplingo_desktop::HostEvent;
use std::sync::{mpsc, Mutex};
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};

type Work = Box<dyn FnOnce(&ActiveEventLoop) + Send>;
struct Dispatcher {
    sender: mpsc::Sender<Work>,
    proxy: EventLoopProxy<HostEvent>,
}
static DISPATCHER: Mutex<Option<Dispatcher>> = Mutex::new(None);

/// Lives alongside the application's event loop. Dropping it releases the
/// global sender before discarding work that has not started.
pub(crate) struct UiQueue(mpsc::Receiver<Work>);
impl UiQueue {
    pub(crate) fn install(proxy: EventLoopProxy<HostEvent>) -> Result<Self, String> {
        let mut current = DISPATCHER.lock().unwrap();
        if current.is_some() {
            return Err("A native application UI queue is already running".into());
        }
        let (sender, receiver) = mpsc::channel();
        *current = Some(Dispatcher { sender, proxy });
        Ok(Self(receiver))
    }
    pub(crate) fn drain(&self, event_loop: &ActiveEventLoop) {
        while let Ok(work) = self.0.try_recv() {
            work(event_loop);
        }
    }
}
impl Drop for UiQueue {
    fn drop(&mut self) {
        *DISPATCHER.lock().unwrap() = None;
    }
}

pub(crate) fn with_event_loop(
    work: impl FnOnce(&ActiveEventLoop) + Send + 'static,
) -> Result<(), String> {
    let current = DISPATCHER.lock().unwrap();
    let dispatcher = current
        .as_ref()
        .ok_or("The native application UI queue is not running")?;
    dispatcher
        .sender
        .send(Box::new(work))
        .map_err(|_| "The native application UI queue has closed")?;
    dispatcher
        .proxy
        .send_event(HostEvent::Wake)
        .map_err(|error| error.to_string())
}

pub(crate) fn event_proxy() -> Result<EventLoopProxy<HostEvent>, String> {
    DISPATCHER
        .lock()
        .unwrap()
        .as_ref()
        .map(|dispatcher| dispatcher.proxy.clone())
        .ok_or_else(|| "The native application UI queue is not running".into())
}

pub(crate) fn dispatch(work: impl FnOnce() + Send + 'static) {
    if let Err(error) = with_event_loop(move |_| work()) {
        log::warn!("Failed to queue native UI work: {error}");
    }
}

pub(crate) async fn run<T: Send + 'static>(
    operation_name: &'static str,
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> crate::Result<T> {
    run_with_event_loop(operation_name, move |_| work()).await
}

pub(crate) async fn run_with_event_loop<T: Send + 'static>(
    operation_name: &'static str,
    work: impl FnOnce(&ActiveEventLoop) -> Result<T, String> + Send + 'static,
) -> crate::Result<T> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    with_event_loop(move |event_loop| {
        if !sender.is_closed() {
            let _ = sender.send(work(event_loop));
        }
    })?;
    receiver
        .await
        .map_err(|error| format!("Failed to receive {operation_name} result: {error}"))?
        .map_err(Into::into)
}

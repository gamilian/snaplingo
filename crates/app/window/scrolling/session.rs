use super::platform::{set_working, update};
use crate::domain::capture::{
    CaptureSessionId, ScrollingCaptureOutputAction, ScrollingCaptureStatus,
};
use crate::infrastructure::ui_thread;
use crate::{CaptureOutput, CaptureSessionRuntime, SettingsConfiguration};
use snaplingo_ui::scrolling::ScrollingAction as Action;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, Weak,
};
#[derive(Clone)]
pub(super) struct Services {
    pub runtime: Weak<CaptureSessionRuntime>,
    pub settings: Arc<SettingsConfiguration>,
    pub output: Arc<CaptureOutput>,
    pub ended: Arc<dyn Fn(&CaptureSessionId, bool) + Send + Sync>,
}
#[derive(Default)]
pub(super) struct Session {
    started: AtomicBool,
    closed: AtomicBool,
    working: AtomicBool,
    paused: AtomicBool,
    task: Mutex<Option<tokio::task::JoinHandle<()>>>,
}
impl Session {
    pub(super) fn stop(&self) {
        let mut task = self.task.lock().unwrap();
        self.closed.store(true, Ordering::SeqCst);
        if let Some(task) = task.take() {
            task.abort();
        }
    }
    pub(super) fn start(self: &Arc<Self>, services: Services, id: String) {
        let mut owner = self.task.lock().unwrap();
        if self.closed.load(Ordering::SeqCst) || self.started.swap(true, Ordering::SeqCst) {
            return;
        }
        let session = self.clone();
        let task = crate::infrastructure::runtime::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                if session.closed.load(Ordering::SeqCst) {
                    break;
                }
                if session.working.load(Ordering::SeqCst) || session.paused.load(Ordering::SeqCst) {
                    continue;
                }
                let Some(runtime) = services.runtime.upgrade() else {
                    break;
                };
                match runtime.capture_scrolling_frame(&id).await {
                    Ok(next) => {
                        session.paused.store(
                            next.status == ScrollingCaptureStatus::Error,
                            Ordering::SeqCst,
                        );
                        if next.status == ScrollingCaptureStatus::Completed {
                            break;
                        }
                    }
                    Err(error) => {
                        if session.closed.load(Ordering::SeqCst) {
                            break;
                        }
                        session.paused.store(true, Ordering::SeqCst);
                        let id = id.clone();
                        let current = session.clone();
                        ui_thread::dispatch(move || {
                            if !current.closed.load(Ordering::SeqCst) {
                                set_working(&id, false, Some(error.to_string()));
                            }
                        });
                    }
                }
            }
        });
        *owner = Some(task);
    }
}
pub(super) fn dispatch(services: Services, id: String, session: Arc<Session>, action: Action) {
    if session.closed.load(Ordering::SeqCst) || session.working.swap(true, Ordering::SeqCst) {
        return;
    }
    let working_id = id.clone();
    let current = session.clone();
    ui_thread::dispatch(move || {
        if !current.closed.load(Ordering::SeqCst) {
            set_working(&working_id, true, None);
        }
    });
    crate::infrastructure::runtime::spawn(async move {
        let result = run_action(&services, &id, action).await;
        let next = if !session.closed.load(Ordering::SeqCst) {
            if let Some(runtime) = services.runtime.upgrade() {
                runtime.scrolling_capture_view(&id).await.ok()
            } else {
                None
            }
        } else {
            None
        };
        session.working.store(false, Ordering::SeqCst);
        session.paused.store(
            result.is_err()
                || next
                    .as_ref()
                    .is_some_and(|view| view.status == ScrollingCaptureStatus::Error),
            Ordering::SeqCst,
        );
        let error = result.err().map(|error| error.to_string());
        if session.closed.load(Ordering::SeqCst) {
            return;
        }
        ui_thread::dispatch(move || {
            if session.closed.load(Ordering::SeqCst) {
                return;
            }
            if let Some(next) = next {
                update(next);
            }
            set_working(&id, false, error);
        });
    });
}
async fn run_action(services: &Services, id: &str, action: Action) -> crate::Result<()> {
    let runtime = services
        .runtime
        .upgrade()
        .ok_or("Capture runtime has stopped")?;
    match action {
        Action::Undo => runtime.undo_scrolling_capture(id).await.map(|_| ()),
        Action::Resume => runtime.capture_scrolling_frame(id).await.map(|_| ()),
        Action::Preview => {
            let view = runtime.scrolling_capture_view(id).await?;
            runtime
                .set_scrolling_preview_visible(id, !view.preview_expanded)
                .await
                .map(|_| ())
        }
        Action::Cancel => runtime.cancel_scrolling_capture(id).await,
        Action::Copy => {
            runtime
                .finish_scrolling_capture(id, ScrollingCaptureOutputAction::Copy)
                .await
        }
        Action::Pin => {
            runtime
                .finish_scrolling_capture(id, ScrollingCaptureOutputAction::Pin)
                .await
        }
        Action::Complete => {
            runtime
                .finish_scrolling_capture(id, ScrollingCaptureOutputAction::Complete)
                .await
        }
        Action::Save => {
            let preferences = services.settings.snapshot()?.screenshot;
            let Some(path) =
                super::platform::select_save_path(&services.settings, &services.output).await?
            else {
                return Ok(());
            };
            runtime
                .finish_scrolling_capture(
                    id,
                    ScrollingCaptureOutputAction::Save {
                        path: path.to_string_lossy().into_owned(),
                        format: preferences.format,
                        quality: preferences.quality,
                        copy_after_save: preferences.auto_copy,
                        remember_choice: true,
                    },
                )
                .await
        }
    }
}

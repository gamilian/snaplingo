use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::{AppHandle, Manager};

use super::scrolling_controls_macos::{self as controls, Action};
use crate::domain::capture::{ScrollingCaptureOutputAction, ScrollingCaptureStatus};

#[derive(Default)]
pub(super) struct Session {
    started: AtomicBool,
    closed: AtomicBool,
    working: AtomicBool,
    paused: AtomicBool,
}

impl Session {
    pub(super) fn stop(&self) {
        self.closed.store(true, Ordering::SeqCst);
    }

    // Start only after the runtime adopts the capture and publishes its first view.
    pub(super) fn start(self: &Arc<Self>, app: AppHandle, id: String) {
        if self.started.swap(true, Ordering::SeqCst) {
            return;
        }
        let session = self.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                if session.closed.load(Ordering::SeqCst) {
                    break;
                }
                if session.working.load(Ordering::SeqCst) || session.paused.load(Ordering::SeqCst) {
                    continue;
                }
                let Some(runtime) = app
                    .try_state::<crate::AppState>()
                    .map(|state| state.capture.runtime.clone())
                else {
                    break;
                };
                match runtime.capture_scrolling_frame(&id).await {
                    Ok(view) => {
                        session.paused.store(
                            view.status == ScrollingCaptureStatus::Error,
                            Ordering::SeqCst,
                        );
                        if view.status == ScrollingCaptureStatus::Completed {
                            break;
                        }
                    }
                    Err(error) => {
                        if session.closed.load(Ordering::SeqCst) {
                            break;
                        }
                        session.paused.store(true, Ordering::SeqCst);
                        let id = id.clone();
                        let _ = app.run_on_main_thread(move || {
                            controls::set_working(&id, false, Some(error.to_string()))
                        });
                    }
                }
            }
        });
    }
}

pub(super) fn dispatch(app: AppHandle, id: String, session: Arc<Session>, action: Action) {
    if session.closed.load(Ordering::SeqCst) || session.working.swap(true, Ordering::SeqCst) {
        return;
    }
    let working_id = id.clone();
    let _ = app.run_on_main_thread(move || controls::set_working(&working_id, true, None));
    tauri::async_runtime::spawn(async move {
        let result = run_action(&app, &id, action).await;
        let runtime = app
            .try_state::<crate::AppState>()
            .map(|state| state.capture.runtime.clone());
        let view = if !session.closed.load(Ordering::SeqCst) {
            if let Some(runtime) = runtime {
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
                || view
                    .as_ref()
                    .is_some_and(|view| view.status == ScrollingCaptureStatus::Error),
            Ordering::SeqCst,
        );
        let error = result.err().map(|error| error.to_string());
        let _ = app.run_on_main_thread(move || {
            if let Some(view) = view {
                controls::update(view);
            }
            controls::set_working(&id, false, error);
        });
    });
}

async fn run_action(app: &AppHandle, id: &str, action: Action) -> crate::Result<()> {
    let state = app.state::<crate::AppState>();
    let runtime = &state.capture.runtime;
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
            let preferences = state.settings.configuration.snapshot()?.screenshot;
            let Some(path) = super::image_save_dialog::select_capture_save_path(app).await? else {
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

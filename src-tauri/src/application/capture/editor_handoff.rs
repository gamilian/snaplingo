use std::time::Duration;

use serde::Serialize;

use super::CaptureSessionRuntime;
use crate::domain::capture::{CaptureSessionId, LogicalRect};
use crate::Result;

const EDITOR_START_TIMEOUT: Duration = Duration::from_secs(8);

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureEditorSelection {
    pub session_id: String,
    pub rect: LogicalRect,
    pub include_cursor: bool,
}

pub(super) struct EditorTransfer {
    selection: CaptureEditorSelection,
    generation: u64,
    completion: Option<tokio::sync::oneshot::Sender<std::result::Result<(), String>>>,
    canceled: bool,
    transferred: bool,
}

impl CaptureSessionRuntime {
    /// Transfer a frozen selection without opening another desktop session.
    pub(crate) async fn open_capture_editor(
        &self,
        session_id: &CaptureSessionId,
        rect: &LogicalRect,
        include_cursor: bool,
    ) -> Result<()> {
        let _startup = self.startup.lock().await;
        let generation = *self.ocr_generation.borrow();
        if self.presentations.lock().await.get(session_id) != Some(&generation) {
            return Err("Capture session is no longer current".into());
        }
        self.ensure_selection_snapshots_ready(session_id, rect)?;
        let geometry = self.sessions.window_geometry(session_id)?;
        let selection = CaptureEditorSelection {
            session_id: session_id.0.clone(),
            rect: rect.clone(),
            include_cursor,
        };
        let (sender, mut completed) = tokio::sync::oneshot::channel();
        *self.editor_transfer.lock().unwrap() = Some(EditorTransfer {
            selection: selection.clone(),
            generation,
            completion: Some(sender),
            canceled: false,
            transferred: false,
        });
        let attempt = async {
            tokio::select! {
                opened = self.host.open_capture_editor_window(&selection, &geometry) => {
                    opened.map_err(|error| error.to_string())?;
                    (&mut completed).await.map_err(|_| "Capture editor handoff was interrupted".to_owned())?
                }
                revealed = (&mut completed) => revealed.map_err(|_| "Capture editor handoff was interrupted".to_owned())?,
            }
        };
        let mut outcome = tokio::time::timeout(EDITOR_START_TIMEOUT, attempt)
            .await
            .unwrap_or_else(|_| Err("Timed out waiting for the capture editor".into()));
        if outcome.is_ok() {
            let mut state = self.editor_transfer.lock().unwrap();
            let current = state.as_mut().filter(|current| {
                current.selection.session_id == session_id.0 && !current.canceled
            });
            match current {
                Some(current) => current.transferred = true,
                None => outcome = Err("Capture editor handoff was canceled".into()),
            }
        }
        if outcome.is_ok() {
            outcome = self
                .host
                .finish_capture_editor_transfer(session_id)
                .await
                .map_err(|error| error.to_string());
        }
        if let Err(error) = outcome {
            self.abort_capture_editor(session_id);
            if let Some(current) = self.editor_transfer.lock().unwrap().as_mut() {
                if current.selection.session_id == session_id.0 {
                    current.transferred = false;
                }
            }
            if let Err(discard) = self.host.discard_capture_editor().await {
                return Err(
                    format!("{error}; failed to discard the capture editor: {discard}").into(),
                );
            }
            return Err(error.into());
        }
        Ok(())
    }

    pub fn capture_editor_selection(
        &self,
        session_id: &CaptureSessionId,
    ) -> Result<Option<CaptureEditorSelection>> {
        let state = self.editor_transfer.lock().unwrap();
        let Some(current) = state
            .as_ref()
            .filter(|current| current.selection.session_id == session_id.0)
        else {
            return Ok(None);
        };
        if current.canceled || current.generation != *self.ocr_generation.borrow() {
            return Err("Capture editor selection is no longer current".into());
        }
        Ok((!current.transferred).then(|| current.selection.clone()))
    }

    pub(crate) fn abort_capture_editor(&self, session_id: &CaptureSessionId) {
        let mut state = self.editor_transfer.lock().unwrap();
        if let Some(current) = state
            .as_mut()
            .filter(|current| current.selection.session_id == session_id.0)
        {
            current.canceled = true;
            if let Some(completion) = current.completion.take() {
                let _ = completion.send(Err("Capture editor handoff was canceled".into()));
            }
        }
    }

    pub(super) fn check_capture_editor_reveal(
        &self,
        session_id: Option<&CaptureSessionId>,
    ) -> Result<()> {
        let state = self.editor_transfer.lock().unwrap();
        if let Some(current) = state.as_ref().filter(|current| !current.transferred) {
            if current.canceled || session_id.is_none_or(|id| id.0 != current.selection.session_id)
            {
                return Err("Capture editor handoff is no longer current".into());
            }
        }
        Ok(())
    }

    pub(super) fn capture_editor_revealed(&self, session_id: Option<&CaptureSessionId>) {
        let mut state = self.editor_transfer.lock().unwrap();
        if let Some(current) = state.as_mut().filter(|current| {
            !current.canceled && session_id.is_some_and(|id| id.0 == current.selection.session_id)
        }) {
            if let Some(completion) = current.completion.take() {
                let _ = completion.send(Ok(()));
            }
        }
    }

    pub(super) fn clear_capture_editor(&self, session_id: &CaptureSessionId) {
        self.abort_capture_editor(session_id);
        let mut state = self.editor_transfer.lock().unwrap();
        if state
            .as_ref()
            .is_some_and(|current| current.selection.session_id == session_id.0)
        {
            *state = None;
        }
    }

    /// A failed/unrevealed editor never owns the native selection's session.
    pub async fn cancel_capture_session_from_window(
        &self,
        session_id: &CaptureSessionId,
    ) -> Result<()> {
        let retained = self
            .editor_transfer
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|current| {
                current.selection.session_id == session_id.0 && !current.transferred
            });
        if retained {
            self.abort_capture_editor(session_id);
            return Ok(());
        }
        self.cancel_capture_session(session_id).await
    }
}

#[cfg(test)]
#[path = "editor_handoff_test.rs"]
mod tests;

use std::sync::{Arc, Mutex};

use super::{
    ResultWindowClipboardPort, ResultWindowMode, ResultWindowNotifierPort, ResultWindowOpenRequest,
    ResultWindowPayload, ResultWindowRequestId, ResultWindowWindowPort,
};

pub(crate) struct ResultWindowRuntime {
    window: Arc<dyn ResultWindowWindowPort>,
    clipboard: Arc<dyn ResultWindowClipboardPort>,
    notifier: Arc<dyn ResultWindowNotifierPort>,
    state: Mutex<ResultWindowState>,
}

struct ResultWindowState {
    latest_request_id: u64,
    pending: Option<PendingResultWindowPayload>,
}

struct PendingResultWindowPayload {
    request_id: ResultWindowRequestId,
    payload: ResultWindowPayload,
}

impl ResultWindowRuntime {
    pub(crate) fn new(
        window: Arc<dyn ResultWindowWindowPort>,
        clipboard: Arc<dyn ResultWindowClipboardPort>,
        notifier: Arc<dyn ResultWindowNotifierPort>,
    ) -> Self {
        Self {
            window,
            clipboard,
            notifier,
            state: Mutex::new(ResultWindowState {
                latest_request_id: 0,
                pending: None,
            }),
        }
    }

    pub(crate) async fn open(&self, request: ResultWindowOpenRequest) -> crate::Result<()> {
        let request_id = self.next_request_id()?;
        let payload = self.payload_for(request).await;

        if !self.store_if_current(request_id, payload)? {
            return Ok(());
        }

        if let Err(error) = self.window.show_or_create().await {
            self.remove_if_current(request_id)?;
            return Err(error);
        }

        if !self.is_current(request_id)? {
            return Ok(());
        }

        self.notifier.notify_payload_ready(request_id).await
    }

    pub(crate) fn take_if_current(
        &self,
        request_id: ResultWindowRequestId,
    ) -> crate::Result<Option<ResultWindowPayload>> {
        let mut state = self.lock_state()?;
        if state.latest_request_id != request_id.0 {
            return Ok(None);
        }

        if !state
            .pending
            .as_ref()
            .is_some_and(|pending| pending.request_id == request_id)
        {
            return Ok(None);
        }

        Ok(state.pending.take().map(|pending| pending.payload))
    }

    /// Returns the pending payload's identity for a late subscriber to claim.
    pub(crate) fn current_request_id(&self) -> crate::Result<Option<ResultWindowRequestId>> {
        let state = self.lock_state()?;
        Ok(state.pending.as_ref().and_then(|pending| {
            (pending.request_id.0 == state.latest_request_id).then_some(pending.request_id)
        }))
    }

    fn next_request_id(&self) -> crate::Result<ResultWindowRequestId> {
        let mut state = self.lock_state()?;
        state.latest_request_id = state
            .latest_request_id
            .checked_add(1)
            .ok_or("Result window request ID exhausted")?;
        state.pending = None;
        Ok(ResultWindowRequestId(state.latest_request_id))
    }

    async fn payload_for(&self, request: ResultWindowOpenRequest) -> ResultWindowPayload {
        match request {
            ResultWindowOpenRequest::Translation {
                text,
                auto_translate,
            } => translation_payload(text, auto_translate),
            ResultWindowOpenRequest::InputTranslation => {
                let text = self.clipboard.read_text().await.unwrap_or_default();
                translation_payload(text, true)
            }
            ResultWindowOpenRequest::Ocr {
                text,
                intent,
                image_base64,
            } => ResultWindowPayload {
                mode: ResultWindowMode::Ocr,
                text,
                auto_translate: false,
                ocr_intent: Some(intent),
                image_base64,
            },
        }
    }

    fn store_if_current(
        &self,
        request_id: ResultWindowRequestId,
        payload: ResultWindowPayload,
    ) -> crate::Result<bool> {
        let mut state = self.lock_state()?;
        if state.latest_request_id != request_id.0 {
            return Ok(false);
        }

        state.pending = Some(PendingResultWindowPayload {
            request_id,
            payload,
        });
        Ok(true)
    }

    fn remove_if_current(&self, request_id: ResultWindowRequestId) -> crate::Result<()> {
        let mut state = self.lock_state()?;
        if state
            .pending
            .as_ref()
            .is_some_and(|pending| pending.request_id == request_id)
        {
            state.pending = None;
        }
        Ok(())
    }

    fn is_current(&self, request_id: ResultWindowRequestId) -> crate::Result<bool> {
        let state = self.lock_state()?;
        Ok(state.latest_request_id == request_id.0)
    }

    fn lock_state(&self) -> crate::Result<std::sync::MutexGuard<'_, ResultWindowState>> {
        self.state
            .lock()
            .map_err(|_| "Result window runtime lock poisoned".into())
    }
}

fn translation_payload(text: String, auto_translate: bool) -> ResultWindowPayload {
    ResultWindowPayload {
        mode: ResultWindowMode::Translation,
        text,
        auto_translate,
        ocr_intent: None,
        image_base64: None,
    }
}

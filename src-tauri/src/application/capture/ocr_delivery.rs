use std::sync::Arc;

use super::CaptureOcrTarget;
use crate::application::result_window::{
    ResultWindowMode, ResultWindowOcrStatus, ResultWindowOpenRequest, ResultWindowRequestId,
    ResultWindowRuntime,
};
use crate::domain::ocr::OcrResult;
use crate::Result;

pub(crate) trait CaptureOcrEffects: Send + Sync {
    fn copy_text(&self, text: String) -> Result<()>;
    fn show_error(&self, error: &str);
}

pub(crate) struct CaptureOcrDelivery {
    result_window: Arc<ResultWindowRuntime>,
    effects: Arc<dyn CaptureOcrEffects>,
}

impl CaptureOcrDelivery {
    pub(crate) fn new(
        result_window: Arc<ResultWindowRuntime>,
        effects: Arc<dyn CaptureOcrEffects>,
    ) -> Self {
        Self {
            result_window,
            effects,
        }
    }

    pub(crate) fn reserve_result(&self, target: CaptureOcrTarget) -> Result<u64> {
        if target == CaptureOcrTarget::Clipboard {
            return Ok(0);
        }
        self.result_window.reserve_request().map(|id| id.0)
    }

    pub(crate) async fn begin(
        &self,
        target: CaptureOcrTarget,
        result_id: u64,
        preview: Option<String>,
    ) -> Result<()> {
        if target == CaptureOcrTarget::Clipboard {
            return Ok(());
        }
        self.result_window
            .open_reserved(
                ResultWindowRequestId(result_id),
                capture_request(
                    target,
                    ResultWindowOcrStatus::Recognizing,
                    None,
                    preview,
                    None,
                ),
            )
            .await
    }

    pub(crate) async fn publish(
        &self,
        target: CaptureOcrTarget,
        result_id: u64,
        result: OcrResult,
        preview: Option<String>,
    ) -> Result<()> {
        if target == CaptureOcrTarget::Clipboard {
            return self.effects.copy_text(result.text);
        }
        self.result_window
            .update_reserved(
                ResultWindowRequestId(result_id),
                capture_request(
                    target,
                    ResultWindowOcrStatus::Complete,
                    Some(result),
                    preview,
                    None,
                ),
            )
            .await
    }

    pub(crate) async fn cancel(&self, target: CaptureOcrTarget, result_id: u64) {
        if target != CaptureOcrTarget::Clipboard {
            if let Err(error) = self
                .result_window
                .update_reserved(
                    ResultWindowRequestId(result_id),
                    capture_request(target, ResultWindowOcrStatus::Cancelled, None, None, None),
                )
                .await
            {
                log::error!("Could not finish cancelled capture OCR: {error}");
            }
        }
    }

    /// Startup can fail before a result window exists, so it needs a native error.
    pub(crate) async fn report_start_error(
        &self,
        target: CaptureOcrTarget,
        result_id: u64,
        error: &str,
    ) {
        self.report_error(target, result_id, error).await;
        if target != CaptureOcrTarget::Clipboard
            && self
                .result_window
                .is_current(ResultWindowRequestId(result_id))
                .unwrap_or(false)
        {
            self.effects.show_error(error);
        }
    }

    pub(crate) async fn report_error(&self, target: CaptureOcrTarget, result_id: u64, error: &str) {
        log::error!("Capture OCR failed: {error}");
        if target == CaptureOcrTarget::Clipboard {
            self.effects.show_error(error);
            return;
        }
        if let Err(delivery_error) = self
            .result_window
            .update_reserved(
                ResultWindowRequestId(result_id),
                capture_request(
                    target,
                    ResultWindowOcrStatus::Failed,
                    None,
                    None,
                    Some(error.into()),
                ),
            )
            .await
        {
            log::error!("Could not show capture OCR failure: {delivery_error}");
        }
    }
}

fn capture_request(
    target: CaptureOcrTarget,
    status: ResultWindowOcrStatus,
    result: Option<OcrResult>,
    preview: Option<String>,
    error: Option<String>,
) -> ResultWindowOpenRequest {
    let translation = target == CaptureOcrTarget::TranslationWindow;
    let (text, confidence, detected_language) = result
        .map(|result| (result.text, result.confidence, result.detected_language))
        .unwrap_or_default();
    ResultWindowOpenRequest::CaptureOcr {
        mode: if translation {
            ResultWindowMode::Translation
        } else {
            ResultWindowMode::Ocr
        },
        status,
        text,
        image_base64: if translation { None } else { preview },
        confidence,
        detected_language,
        error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::result_window::{
        ResultWindowMode, ResultWindowNotifierPort, ResultWindowOrigin, ResultWindowWindowPort,
    };
    use async_trait::async_trait;
    use std::sync::Mutex;

    #[derive(Default)]
    struct Effects {
        copies: Mutex<Vec<String>>,
        errors: Mutex<Vec<String>>,
        opens: Mutex<usize>,
    }
    impl CaptureOcrEffects for Effects {
        fn copy_text(&self, text: String) -> Result<()> {
            self.copies.lock().unwrap().push(text);
            Ok(())
        }
        fn show_error(&self, error: &str) {
            self.errors.lock().unwrap().push(error.into());
        }
    }
    #[async_trait]
    impl ResultWindowWindowPort for Effects {
        async fn show_or_create(&self) -> Result<()> {
            *self.opens.lock().unwrap() += 1;
            Ok(())
        }
    }
    #[async_trait]
    impl ResultWindowNotifierPort for Effects {
        async fn notify_payload_ready(&self, _: ResultWindowRequestId) -> Result<()> {
            Ok(())
        }
    }
    fn runtime() -> (CaptureOcrDelivery, Arc<ResultWindowRuntime>, Arc<Effects>) {
        let effects = Arc::new(Effects::default());
        let window = Arc::new(ResultWindowRuntime::new(effects.clone(), effects.clone()));
        (
            CaptureOcrDelivery::new(window.clone(), effects.clone()),
            window,
            effects,
        )
    }
    fn result() -> OcrResult {
        OcrResult {
            text: "recognized".into(),
            confidence: Some(0.9),
            detected_language: Some("en".into()),
            lines: vec![],
            provider_id: Some("system".into()),
        }
    }
    #[tokio::test]
    async fn maps_both_window_targets_and_preserves_preview_metadata() {
        let (delivery, window, effects) = runtime();
        for (index, target) in [
            CaptureOcrTarget::OcrWindow,
            CaptureOcrTarget::TranslationWindow,
        ]
        .into_iter()
        .enumerate()
        {
            let id = delivery.reserve_result(target).unwrap();
            delivery
                .begin(target, id, Some("frozen-preview".into()))
                .await
                .unwrap();
            let loading = window
                .take_if_current(ResultWindowRequestId(id))
                .unwrap()
                .unwrap();
            assert_eq!(loading.ocr_status, Some(ResultWindowOcrStatus::Recognizing));
            assert_eq!(*effects.opens.lock().unwrap(), index + 1);
            delivery
                .publish(target, id, result(), Some("frozen-preview".into()))
                .await
                .unwrap();
            let payload = window
                .take_if_current(ResultWindowRequestId(id))
                .unwrap()
                .unwrap();
            assert_eq!(payload.text, "recognized");
            assert_eq!(payload.ocr_status, Some(ResultWindowOcrStatus::Complete));
            assert_eq!(*effects.opens.lock().unwrap(), index + 1);
            assert_eq!(payload.detected_language.as_deref(), Some("en"));
            if target == CaptureOcrTarget::OcrWindow {
                assert_eq!(payload.mode, ResultWindowMode::Ocr);
                assert_eq!(payload.image_base64.as_deref(), Some("frozen-preview"));
                assert_eq!(payload.confidence, Some(0.9));
            } else {
                assert_eq!(payload.origin, ResultWindowOrigin::Screenshot);
                assert!(payload.auto_translate);
                assert_eq!(payload.image_base64, None);
            }
        }
    }
    #[tokio::test]
    async fn stale_results_and_errors_do_not_replace_a_newer_request() {
        let (delivery, window, effects) = runtime();
        let stale = delivery
            .reserve_result(CaptureOcrTarget::OcrWindow)
            .unwrap();
        delivery
            .begin(CaptureOcrTarget::OcrWindow, stale, None)
            .await
            .unwrap();
        window
            .open(ResultWindowOpenRequest::manual_translation("newer".into()))
            .await
            .unwrap();
        let current = window.current_request_id().unwrap().unwrap();
        delivery
            .publish(CaptureOcrTarget::OcrWindow, stale, result(), None)
            .await
            .unwrap();
        delivery
            .report_error(CaptureOcrTarget::OcrWindow, stale, "old failure")
            .await;
        delivery.cancel(CaptureOcrTarget::OcrWindow, stale).await;
        delivery
            .report_start_error(CaptureOcrTarget::OcrWindow, stale, "stale startup failure")
            .await;
        assert_eq!(
            window.take_if_current(current).unwrap().unwrap().text,
            "newer"
        );
        assert_eq!(*effects.opens.lock().unwrap(), 2);
        assert!(effects.errors.lock().unwrap().is_empty());
        let current = delivery
            .reserve_result(CaptureOcrTarget::OcrWindow)
            .unwrap();
        delivery
            .report_error(CaptureOcrTarget::OcrWindow, current, "current failure")
            .await;
        let failed = window
            .take_if_current(ResultWindowRequestId(current))
            .unwrap()
            .unwrap();
        assert_eq!(failed.ocr_status, Some(ResultWindowOcrStatus::Failed));
        assert_eq!(failed.ocr_error.as_deref(), Some("current failure"));
        assert!(effects.errors.lock().unwrap().is_empty());
    }
    #[tokio::test]
    async fn clipboard_delivery_preserves_the_current_window_request() {
        let (delivery, window, effects) = runtime();
        window
            .open(ResultWindowOpenRequest::manual_translation(
                "existing".into(),
            ))
            .await
            .unwrap();
        let current = window.current_request_id().unwrap().unwrap();
        let clipboard = delivery
            .reserve_result(CaptureOcrTarget::Clipboard)
            .unwrap();
        delivery
            .begin(CaptureOcrTarget::Clipboard, clipboard, None)
            .await
            .unwrap();
        delivery
            .cancel(CaptureOcrTarget::Clipboard, clipboard)
            .await;
        delivery
            .publish(CaptureOcrTarget::Clipboard, clipboard, result(), None)
            .await
            .unwrap();
        delivery
            .report_error(CaptureOcrTarget::Clipboard, clipboard, "clipboard failure")
            .await;
        assert_eq!(*effects.copies.lock().unwrap(), vec!["recognized"]);
        assert_eq!(*effects.errors.lock().unwrap(), vec!["clipboard failure"]);
        assert_eq!(
            window.take_if_current(current).unwrap().unwrap().text,
            "existing"
        );
        assert_eq!(*effects.opens.lock().unwrap(), 1);
    }
}

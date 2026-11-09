use std::sync::Arc;

use super::CaptureOcrTarget;
use crate::application::result_window::{
    ResultWindowOpenRequest, ResultWindowRequestId, ResultWindowRuntime,
};
use crate::domain::ocr::OcrResult;
use crate::Result;

pub trait CaptureOcrEffects: Send + Sync {
    fn copy_text(&self, text: String) -> Result<()>;
    fn show_error(&self, error: &str);
}

pub struct CaptureOcrDelivery {
    result_window: Arc<ResultWindowRuntime>,
    effects: Arc<dyn CaptureOcrEffects>,
}

impl CaptureOcrDelivery {
    pub fn new(
        result_window: Arc<ResultWindowRuntime>,
        effects: Arc<dyn CaptureOcrEffects>,
    ) -> Self {
        Self {
            result_window,
            effects,
        }
    }

    pub fn reserve_result(&self, target: CaptureOcrTarget) -> Result<u64> {
        if target == CaptureOcrTarget::Clipboard {
            return Ok(0);
        }
        self.result_window.reserve_request().map(|id| id.0)
    }

    pub async fn publish(
        &self,
        target: CaptureOcrTarget,
        result_id: u64,
        result: OcrResult,
        preview: Option<String>,
    ) -> Result<()> {
        if target == CaptureOcrTarget::Clipboard {
            return self.effects.copy_text(result.text);
        }
        let request = if target == CaptureOcrTarget::TranslationWindow {
            ResultWindowOpenRequest::screenshot_translation(result.text, result.detected_language)
        } else {
            ResultWindowOpenRequest::capture_ocr(
                result.text,
                preview,
                result.confidence,
                result.detected_language,
            )
        };
        self.result_window
            .open_reserved(ResultWindowRequestId(result_id), request)
            .await
    }

    pub fn report_error(&self, target: CaptureOcrTarget, result_id: u64, error: &str) {
        log::error!("Capture OCR failed: {error}");
        if target != CaptureOcrTarget::Clipboard
            && !self
                .result_window
                .is_current(ResultWindowRequestId(result_id))
                .unwrap_or(false)
        {
            return;
        }
        self.effects.show_error(error);
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
        let (delivery, window, _) = runtime();
        for target in [
            CaptureOcrTarget::OcrWindow,
            CaptureOcrTarget::TranslationWindow,
        ] {
            let id = delivery.reserve_result(target).unwrap();
            delivery
                .publish(target, id, result(), Some("frozen-preview".into()))
                .await
                .unwrap();
            let payload = window
                .take_if_current(ResultWindowRequestId(id))
                .unwrap()
                .unwrap();
            assert_eq!(payload.text, "recognized");
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
        window
            .open(ResultWindowOpenRequest::manual_translation("newer".into()))
            .await
            .unwrap();
        let current = window.current_request_id().unwrap().unwrap();
        delivery
            .publish(CaptureOcrTarget::OcrWindow, stale, result(), None)
            .await
            .unwrap();
        delivery.report_error(CaptureOcrTarget::OcrWindow, stale, "old failure");
        assert_eq!(
            window.take_if_current(current).unwrap().unwrap().text,
            "newer"
        );
        assert_eq!(*effects.opens.lock().unwrap(), 1);
        assert!(effects.errors.lock().unwrap().is_empty());
        let current = delivery
            .reserve_result(CaptureOcrTarget::OcrWindow)
            .unwrap();
        delivery.report_error(CaptureOcrTarget::OcrWindow, current, "current failure");
        assert_eq!(*effects.errors.lock().unwrap(), vec!["current failure"]);
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
            .publish(CaptureOcrTarget::Clipboard, clipboard, result(), None)
            .await
            .unwrap();
        delivery.report_error(CaptureOcrTarget::Clipboard, clipboard, "clipboard failure");
        assert_eq!(*effects.copies.lock().unwrap(), vec!["recognized"]);
        assert_eq!(*effects.errors.lock().unwrap(), vec!["clipboard failure"]);
        assert_eq!(
            window.take_if_current(current).unwrap().unwrap().text,
            "existing"
        );
        assert_eq!(*effects.opens.lock().unwrap(), 1);
    }
}

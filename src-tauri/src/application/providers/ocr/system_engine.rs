use crate::domain::ocr::{OcrRequest, OcrResult};
use crate::Result;

pub(crate) trait SystemOcrEngine: Send + Sync {
    fn is_available(&self) -> bool;

    fn recognize(&self, request: &OcrRequest) -> Result<OcrResult>;

    fn recognize_with_recovery(&self, request: &OcrRequest) -> Result<OcrResult> {
        self.recognize(request)
    }
}

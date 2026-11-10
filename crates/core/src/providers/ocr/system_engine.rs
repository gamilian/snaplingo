use crate::providers::ocr::model::OcrRequest;
use crate::providers::ocr::model::OcrResult;
use crate::Result;

pub trait SystemOcrEngine: Send + Sync {
    fn is_available(&self) -> bool;

    fn recognize(&self, request: &OcrRequest) -> Result<OcrResult>;

    fn recognize_with_recovery(&self, request: &OcrRequest) -> Result<OcrResult> {
        self.recognize(request)
    }
}

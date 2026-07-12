use crate::domain::ocr::{OcrRequest, OcrResult};
use crate::Result;

pub(crate) trait SystemOcrEngine: Send + Sync {
    fn recognize(&self, request: &OcrRequest) -> Result<OcrResult>;
}

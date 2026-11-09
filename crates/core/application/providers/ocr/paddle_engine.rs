use crate::domain::ocr::{OcrRequest, OcrResult};
use crate::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OcrExecutionDevice {
    Cpu,
    Gpu,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OcrAccelerationStatus {
    pub gpu_enabled: bool,
    pub device: Option<OcrExecutionDevice>,
    pub fallback_reason: Option<String>,
}

pub trait PaddleOcrEngine: Send + Sync {
    fn is_available(&self) -> bool;
    fn recognize(&self, request: &OcrRequest, gpu_enabled: bool) -> Result<OcrResult>;
    fn status(&self) -> OcrAccelerationStatus;
}

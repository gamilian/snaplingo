mod mnn_engine;
pub(crate) mod small_worker;
#[cfg(test)]
mod tests;

use std::path::PathBuf;

use parking_lot::RwLock;
use tokio::sync::{Mutex, Semaphore};

use crate::application::providers::ocr::{
    OcrAccelerationStatus, OcrExecutionDevice, PaddleOcrEngine,
};
use crate::domain::ocr::{OcrBoundingBox, OcrLine, OcrRequest, OcrResult};
use crate::{AppError, Result};

const MODEL_FILES: [&str; 3] = ["small_det.mnn", "small_rec.mnn", "small_keys.txt"];

pub(crate) struct LocalPaddleOcrEngine {
    root: Option<PathBuf>,
    request_lock: Mutex<()>,
    request_slots: Semaphore,
    status: RwLock<OcrAccelerationStatus>,
}

impl LocalPaddleOcrEngine {
    pub(crate) fn new(root: Option<PathBuf>) -> Self {
        Self {
            root,
            request_lock: Mutex::new(()),
            request_slots: Semaphore::new(2),
            status: RwLock::new(OcrAccelerationStatus {
                gpu_enabled: false,
                device: Some(OcrExecutionDevice::Cpu),
                fallback_reason: Some("small · CPU · 识别后释放模型".into()),
            }),
        }
    }
}

#[async_trait::async_trait]
impl PaddleOcrEngine for LocalPaddleOcrEngine {
    fn supports_gpu(&self) -> bool {
        false
    }

    fn is_available(&self) -> bool {
        self.root
            .as_ref()
            .is_some_and(|root| MODEL_FILES.iter().all(|file| root.join(file).is_file()))
    }

    fn status(&self) -> OcrAccelerationStatus {
        self.status.read().clone()
    }

    async fn recognize(&self, request: &OcrRequest, _: bool) -> Result<OcrResult> {
        let root = self
            .root
            .as_deref()
            .ok_or_else(|| AppError::Other("Packaged OCR resources are unavailable".into()))?;
        // Exiting the worker releases both model sessions and native allocator caches.
        let _slot = self.request_slots.try_acquire().map_err(|_| {
            AppError::System("OCR is busy; try again after the current request".into())
        })?;
        let _request = self.request_lock.lock().await;
        let result = small_worker::recognize(root, request).await;
        self.status.write().device = result.as_ref().ok().map(|_| OcrExecutionDevice::Cpu);
        result
    }
}

fn normalized_box(points: &[(f32, f32)], width: u32, height: u32) -> Option<OcrBoundingBox> {
    if points.is_empty() || points.iter().any(|(x, y)| !x.is_finite() || !y.is_finite()) {
        return None;
    }
    let x0 = points
        .iter()
        .map(|p| p.0)
        .fold(f32::INFINITY, f32::min)
        .clamp(0.0, width as f32);
    let x1 = points
        .iter()
        .map(|p| p.0)
        .fold(f32::NEG_INFINITY, f32::max)
        .clamp(0.0, width as f32);
    let y0 = points
        .iter()
        .map(|p| p.1)
        .fold(f32::INFINITY, f32::min)
        .clamp(0.0, height as f32);
    let y1 = points
        .iter()
        .map(|p| p.1)
        .fold(f32::NEG_INFINITY, f32::max)
        .clamp(0.0, height as f32);
    Some(OcrBoundingBox {
        x: x0 / width as f32,
        y: 1.0 - y1 / height as f32,
        width: (x1 - x0) / width as f32,
        height: (y1 - y0) / height as f32,
    })
}

fn from_lines(lines: Vec<OcrLine>) -> OcrResult {
    let scores: Vec<_> = lines.iter().filter_map(|line| line.confidence).collect();
    OcrResult {
        text: lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        confidence: (!scores.is_empty()).then(|| scores.iter().sum::<f32>() / scores.len() as f32),
        lines,
        detected_language: None,
        provider_id: None,
    }
}

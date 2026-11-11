use super::{from_lines, normalized_box, MODEL_FILES};
use crate::domain::ocr::OcrLine;
use crate::{AppError, Result};
use image::RgbImage;
use ocr_rs::{Backend, OcrEngine, OcrEngineConfig, RecOptions};
use std::path::Path;

pub(super) struct MnnSession {
    engine: OcrEngine,
}

impl MnnSession {
    pub(super) fn new(root: &Path) -> Result<Self> {
        let config = OcrEngineConfig::new()
            .with_backend(Backend::CPU)
            .with_threads(4)
            .with_parallel(true)
            .with_rec_options(RecOptions::default().with_batch_size(8));
        let engine = OcrEngine::new(
            root.join(MODEL_FILES[0]),
            root.join(MODEL_FILES[1]),
            root.join(MODEL_FILES[2]),
            Some(config),
        )
        .map_err(|error| AppError::Other(format!("Could not initialize MNN OCR: {error}")))?;
        Ok(Self { engine })
    }
}

impl MnnSession {
    pub(super) fn recognize(&mut self, image: &RgbImage) -> Result<crate::domain::ocr::OcrResult> {
        let results = self
            .engine
            .recognize(&image::DynamicImage::ImageRgb8(image.clone()))
            .map_err(|error| AppError::Other(format!("MNN OCR inference failed: {error}")))?;
        Ok(from_lines(
            results
                .into_iter()
                .filter(|result| !result.text.is_empty())
                .map(|result| {
                    let rect = result.bbox.rect;
                    let points = result
                        .bbox
                        .points
                        .map(|points| points.iter().map(|p| (p.x, p.y)).collect::<Vec<_>>())
                        .unwrap_or_else(|| {
                            vec![
                                (rect.left() as f32, rect.top() as f32),
                                (
                                    (rect.left() + rect.width() as i32) as f32,
                                    (rect.top() + rect.height() as i32) as f32,
                                ),
                            ]
                        });
                    OcrLine {
                        text: result.text,
                        confidence: Some(result.confidence),
                        bounding_box: normalized_box(&points, image.width(), image.height()),
                    }
                })
                .collect(),
        ))
    }
}

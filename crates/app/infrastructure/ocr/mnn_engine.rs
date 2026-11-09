use super::{from_lines, normalized_box, MediumSession};
use crate::{AppError, Result};
use image::RgbImage;
use ocr_rs::{Backend, OcrEngine, OcrEngineConfig, RecOptions};
use snaplingo_core::domain::ocr::OcrLine;
use std::path::Path;

pub(super) struct MnnMediumSession {
    engine: OcrEngine,
}

impl MnnMediumSession {
    pub(super) fn new(root: &Path) -> Result<Self> {
        let config = OcrEngineConfig::new()
            .with_backend(Backend::CPU)
            .with_threads(4)
            .with_rec_options(RecOptions::default().with_batch_size(8));
        let engine = OcrEngine::new(
            root.join("medium_det.mnn"),
            root.join("medium_rec.mnn"),
            root.join("keys.txt"),
            Some(config),
        )
        .map_err(|error| AppError::Other(format!("Could not initialize MNN medium: {error}")))?;
        Ok(Self { engine })
    }
}

impl MediumSession for MnnMediumSession {
    fn recognize(&mut self, image: &RgbImage) -> Result<snaplingo_core::domain::ocr::OcrResult> {
        let results = self
            .engine
            .recognize(&image::DynamicImage::ImageRgb8(image.clone()))
            .map_err(|error| AppError::Other(format!("MNN medium inference failed: {error}")))?;
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

use std::path::Path;
use std::sync::OnceLock;

use image::{DynamicImage, RgbImage};
use ndarray::{s, Array3, Array4, Ix3, Ix4};
use oar_ocr_core::processors::{
    sort_quad_boxes, ColorOrder, DBPostProcess, DetResizeForTest, LimitType, NormalizeImage,
};
use oar_ocr_core::utils::get_rotate_crop_image;
use ort::session::{builder::GraphOptimizationLevel, Session};
use ort::value::Tensor;

use super::{from_lines, normalized_box, MediumSession, OcrBackend};
use crate::domain::ocr::{OcrLine, OcrResult};
use crate::{AppError, Result};

#[cfg(target_os = "macos")]
pub(super) const LIBRARY_NAME: &str = "libonnxruntime.dylib";
#[cfg(target_os = "windows")]
pub(super) const LIBRARY_NAME: &str = "onnxruntime.dll";
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub(super) const LIBRARY_NAME: &str = "libonnxruntime.so";

static RUNTIME: OnceLock<std::result::Result<(), String>> = OnceLock::new();

fn initialize_runtime(root: &Path) -> Result<()> {
    RUNTIME
        .get_or_init(|| {
            #[cfg(target_os = "windows")]
            {
                static SIDE_LIBRARIES: OnceLock<Vec<libloading::Library>> = OnceLock::new();
                let parent = root
                    .parent()
                    .ok_or("OCR resource directory has no parent")?;
                let development_crt = parent.join("windows-crt");
                let crt = if development_crt.is_dir() {
                    development_crt.as_path()
                } else {
                    parent
                };
                let mut libraries = Vec::new();
                // Load the mandatory application-local CRT dependencies before ORT.
                for name in [
                    "vcruntime140.dll",
                    "vcruntime140_1.dll",
                    "msvcp140.dll",
                    "msvcp140_1.dll",
                ] {
                    libraries.push(
                        unsafe { libloading::Library::new(crt.join(name)) }
                            .map_err(|error| error.to_string())?,
                    );
                }
                let _ = SIDE_LIBRARIES.set(libraries);
            }
            ort::init_from(root.join("runtime").join(LIBRARY_NAME))
                .map_err(|error| error.to_string())?
                .with_name("SnapLingo OCR")
                .with_telemetry(false)
                .commit();
            Ok(())
        })
        .as_ref()
        .map(|_| ())
        .map_err(|error| AppError::Other(format!("Could not load OCR runtime: {error}")))
}

pub(super) struct OrtMediumSession {
    detection: Session,
    recognition: Session,
    detection_input: String,
    recognition_input: String,
    dictionary: Vec<String>,
}

fn session(model: &Path, cache: &Path, backend: OcrBackend) -> Result<Session> {
    #[cfg(not(target_os = "macos"))]
    let _ = cache;
    let mut builder = Session::builder()
        .map_err(ocr_error)?
        .with_intra_threads(4)
        .map_err(ocr_error)?
        .with_inter_threads(1)
        .map_err(ocr_error)?
        .with_parallel_execution(false)
        .map_err(ocr_error)?
        .with_memory_pattern(false)
        .map_err(ocr_error)?
        .with_optimization_level(GraphOptimizationLevel::Level3)
        .map_err(ocr_error)?;
    match backend {
        OcrBackend::CoreMl => {
            #[cfg(target_os = "macos")]
            {
                use ort::ep::coreml::{ComputeUnits, ModelFormat};
                std::fs::create_dir_all(cache)?;
                builder = builder
                    .with_execution_providers([ort::ep::CoreML::default()
                        .with_model_format(ModelFormat::MLProgram)
                        .with_compute_units(ComputeUnits::CPUAndGPU)
                        .with_model_cache_dir(cache.to_string_lossy())
                        .build()
                        .error_on_failure()])
                    .map_err(ocr_error)?;
            }
            #[cfg(not(target_os = "macos"))]
            {
                return Err(AppError::Other("Core ML requires macOS".into()));
            }
        }
        OcrBackend::DirectMl => {
            #[cfg(target_os = "windows")]
            {
                static DIRECTML: OnceLock<libloading::Library> = OnceLock::new();
                if DIRECTML.get().is_none() {
                    let root = model.parent().ok_or_else(|| {
                        AppError::Other("OCR model directory has no parent".into())
                    })?;
                    // GPU-only dependencies must not prevent CPU initialization or fallback.
                    let library =
                        unsafe { libloading::Library::new(root.join("runtime/DirectML.dll")) }
                            .map_err(ocr_error)?;
                    let _ = DIRECTML.set(library);
                }
                builder = builder
                    .with_execution_providers([ort::ep::DirectML::default()
                        .build()
                        .error_on_failure()])
                    .map_err(ocr_error)?;
            }
            #[cfg(not(target_os = "windows"))]
            {
                return Err(AppError::Other("DirectML requires Windows".into()));
            }
        }
        OcrBackend::OrtCpu => {
            builder = builder
                .with_execution_providers([ort::ep::CPU::default().build()])
                .map_err(ocr_error)?;
        }
        OcrBackend::MnnCpu => {
            return Err(AppError::Other("MNN requires its own OCR session".into()))
        }
    }
    builder.commit_from_file(model).map_err(ocr_error)
}

impl OrtMediumSession {
    pub(super) fn new(root: &Path, cache: &Path, backend: OcrBackend) -> Result<Self> {
        initialize_runtime(root)?;
        let detection = session(
            &root.join("medium_det.onnx"),
            &cache.join("medium-det-ort31"),
            backend,
        )?;
        let recognition = session(
            &root.join("medium_rec.onnx"),
            &cache.join("medium-rec-ort31"),
            backend,
        )?;
        let dictionary = std::iter::once(String::new())
            .chain(
                std::fs::read_to_string(root.join("keys.txt"))?
                    .lines()
                    .map(str::to_owned),
            )
            .chain(std::iter::once(" ".to_string()))
            .collect();
        let detection_input = detection.inputs()[0].name().to_string();
        let recognition_input = recognition.inputs()[0].name().to_string();
        Ok(Self {
            detection,
            recognition,
            detection_input,
            recognition_input,
            dictionary,
        })
    }
}

impl MediumSession for OrtMediumSession {
    fn recognize(&mut self, image: &RgbImage) -> Result<OcrResult> {
        let (resized, scales) = DetResizeForTest::new(
            None,
            None,
            None,
            Some(960),
            Some(LimitType::Max),
            None,
            Some(4000),
        )
        .apply(
            vec![DynamicImage::ImageRgb8(image.clone())],
            None,
            None,
            None,
        );
        let detector_normalizer =
            NormalizeImage::new(None, None, None, None, Some(ColorOrder::BGR))
                .map_err(ocr_error)?;
        let input = detector_normalizer
            .normalize_to(resized.into_iter().next().unwrap())
            .map_err(ocr_error)?;
        let outputs = self.detection.run(ort::inputs![self.detection_input.as_str() => Tensor::from_array(input).map_err(ocr_error)?]).map_err(ocr_error)?;
        let prediction = outputs[0]
            .try_extract_array::<f32>()
            .map_err(ocr_error)?
            .into_dimensionality::<Ix4>()
            .map_err(ocr_error)?
            .to_owned();
        if prediction.shape()[0] != 1
            || prediction.shape()[1] != 1
            || prediction.shape()[2..].contains(&0)
        {
            return Err(AppError::InvalidResponse(
                "Invalid OCR detection output shape".into(),
            ));
        }
        if prediction.iter().any(|value| !value.is_finite()) {
            return Err(AppError::InvalidResponse(
                "OCR detection returned non-finite scores".into(),
            ));
        }
        let (boxes, _) = DBPostProcess::new(
            Some(0.3),
            Some(0.5),
            None,
            Some(1.5),
            Some(false),
            None,
            None,
        )
        .apply(&prediction, scales, None);
        let boxes = sort_quad_boxes(&boxes[0]);
        drop(outputs);
        let recognition_normalizer = NormalizeImage::for_ocr_recognition().map_err(ocr_error)?;
        let mut lines = Vec::with_capacity(boxes.len());
        for batch in boxes.chunks(8) {
            let crops = batch
                .iter()
                .map(|bbox| get_rotate_crop_image(image, &bbox.points).map_err(ocr_error))
                .collect::<Result<Vec<_>>>()?;
            let input = recognition_tensor(&crops, &recognition_normalizer)?;
            let outputs = self.recognition.run(ort::inputs![self.recognition_input.as_str() => Tensor::from_array(input).map_err(ocr_error)?]).map_err(ocr_error)?;
            let prediction = outputs[0]
                .try_extract_array::<f32>()
                .map_err(ocr_error)?
                .into_dimensionality::<Ix3>()
                .map_err(ocr_error)?
                .to_owned();
            if prediction.shape()[0] != batch.len() {
                return Err(AppError::InvalidResponse(
                    "OCR recognition returned an incomplete batch".into(),
                ));
            }
            for (bbox, (text, confidence)) in
                batch.iter().zip(decode_ctc(&prediction, &self.dictionary)?)
            {
                if text.is_empty() || confidence < 0.5 {
                    continue;
                }
                let points = bbox.points.iter().map(|p| (p.x, p.y)).collect::<Vec<_>>();
                lines.push(OcrLine {
                    text,
                    confidence: Some(confidence),
                    bounding_box: normalized_box(&points, image.width(), image.height()),
                });
            }
        }
        Ok(from_lines(lines))
    }
}

fn recognition_tensor(crops: &[RgbImage], normalizer: &NormalizeImage) -> Result<Array4<f32>> {
    let widths = crops
        .iter()
        .map(|image| {
            ((image.width() as f32 / image.height() as f32 * 48.0).ceil() as usize).clamp(1, 3200)
        })
        .collect::<Vec<_>>();
    let width = widths.iter().copied().max().unwrap_or(320).max(320);
    // PaddleOCR pads after normalization, so the padded tensor contains zero, not black (-1).
    let mut tensor = Array4::zeros((crops.len(), 3, 48, width));
    for (index, (image, &crop_width)) in crops.iter().zip(&widths).enumerate() {
        let resized = image::imageops::resize(
            image,
            crop_width as u32,
            48,
            image::imageops::FilterType::Triangle,
        );
        let normalized = normalizer
            .normalize_to(DynamicImage::ImageRgb8(resized))
            .map_err(ocr_error)?;
        tensor
            .slice_mut(s![index, .., .., ..crop_width])
            .assign(&normalized.slice(s![0, .., .., ..]));
    }
    Ok(tensor)
}

fn decode_ctc(prediction: &Array3<f32>, dictionary: &[String]) -> Result<Vec<(String, f32)>> {
    if prediction.shape()[2] != dictionary.len() {
        return Err(AppError::InvalidResponse(format!(
            "OCR vocabulary mismatch: model {}, dictionary {}",
            prediction.shape()[2],
            dictionary.len()
        )));
    }
    prediction
        .outer_iter()
        .map(|sample| {
            let mut text = String::new();
            let mut previous = 0;
            let mut score = 0.0;
            let mut count = 0;
            for timestep in sample.outer_iter() {
                let mut best = 0;
                for (index, &value) in timestep.iter().enumerate() {
                    if !value.is_finite() {
                        return Err(AppError::InvalidResponse(
                            "OCR returned non-finite scores".into(),
                        ));
                    }
                    if value > timestep[best] {
                        best = index;
                    }
                }
                if best != 0 && best != previous {
                    text.push_str(&dictionary[best]);
                    score += timestep[best];
                    count += 1;
                }
                previous = best;
            }
            Ok((
                text,
                if count == 0 {
                    0.0
                } else {
                    score / count as f32
                },
            ))
        })
        .collect()
}

fn ocr_error(error: impl std::fmt::Display) -> AppError {
    AppError::Other(format!("PaddleOCR: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ctc_preserves_repeated_characters_separated_by_blank_and_whole_tokens() {
        let prediction = Array3::from_shape_vec(
            (1, 5, 3),
            vec![
                0.0, 0.9, 0.1, 0.0, 0.8, 0.2, 0.9, 0.0, 0.1, 0.0, 0.7, 0.3, 0.0, 0.1, 0.9,
            ],
        )
        .unwrap();
        let decoded = decode_ctc(&prediction, &["".into(), "中".into(), "ab".into()]).unwrap();
        assert_eq!(decoded[0].0, "中中ab");
        assert!((decoded[0].1 - 2.5 / 3.0).abs() < 0.0001);
    }

    #[test]
    fn recognition_padding_is_zero_after_normalization() {
        let tensor = recognition_tensor(
            &[RgbImage::from_pixel(10, 48, image::Rgb([255, 255, 255]))],
            &NormalizeImage::for_ocr_recognition().unwrap(),
        )
        .unwrap();
        assert_eq!(tensor.shape(), &[1, 3, 48, 320]);
        assert_eq!(tensor[[0, 0, 0, 0]], 1.0);
        assert_eq!(tensor[[0, 0, 0, 10]], 0.0);
    }

    #[test]
    fn invalid_scores_and_wrong_dictionary_are_rejected() {
        assert!(decode_ctc(&Array3::zeros((1, 2, 3)), &["".into()]).is_err());
        assert!(decode_ctc(&Array3::from_elem((1, 2, 1), f32::NAN), &["".into()]).is_err());
    }
}

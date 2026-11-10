use crate::application::providers::ocr::SystemOcrEngine;
use crate::domain::ocr::{OcrBoundingBox, OcrLine, OcrRequest, OcrResult};
use crate::{AppError, Result};
use objc2::runtime::AnyObject;
use objc2::AnyThread;
use objc2_foundation::{NSArray, NSData, NSDictionary, NSString};
use objc2_vision::{
    VNDetectedObjectObservation, VNImageOption, VNImageRequestHandler, VNRecognizeTextRequest,
    VNRequest, VNRequestTextRecognitionLevel,
};
use std::io::Cursor;

pub struct MacOSVisionOcrEngine;

impl MacOSVisionOcrEngine {
    pub fn new() -> Self {
        Self
    }
}

impl SystemOcrEngine for MacOSVisionOcrEngine {
    fn is_available(&self) -> bool {
        true
    }

    fn recognize(&self, request: &OcrRequest) -> Result<OcrResult> {
        recognize_with_vision(request)
    }

    fn recognize_with_recovery(&self, request: &OcrRequest) -> Result<OcrResult> {
        let initial = recognize_with_vision(request)?;
        if !should_attempt_recovery(&initial) {
            return Ok(initial);
        }

        let enhanced_image = match enhance_image_for_recovery(&request.image_data) {
            Ok(Some(image)) => image,
            Ok(None) | Err(_) => return Ok(initial),
        };
        let recovered =
            match recognize_with_vision_data(&enhanced_image, request.language.as_deref()) {
                Ok(result) => result,
                Err(_) => return Ok(initial),
            };

        Ok(if is_better_result(&recovered, &initial) {
            recovered
        } else {
            initial
        })
    }
}

fn recognize_with_vision(request: &OcrRequest) -> Result<OcrResult> {
    recognize_with_vision_data(&request.image_data, request.language.as_deref())
}

fn recognize_with_vision_data(
    image_data: &[u8],
    requested_language: Option<&str>,
) -> Result<OcrResult> {
    let image_data = NSData::with_bytes(image_data);
    let options = NSDictionary::<VNImageOption, AnyObject>::new();
    let handler = VNImageRequestHandler::initWithData_options(
        VNImageRequestHandler::alloc(),
        &image_data,
        &options,
    );

    let vision_request = VNRecognizeTextRequest::new();
    vision_request.setRecognitionLevel(VNRequestTextRecognitionLevel::Accurate);
    vision_request.setUsesLanguageCorrection(true);
    if objc2::available!(macos = 13.0) {
        vision_request.setAutomaticallyDetectsLanguage(true);
    }

    let language_values: Vec<_> = vision_languages_for_request(requested_language)
        .iter()
        .map(|language| NSString::from_str(language))
        .collect();
    let languages = NSArray::from_retained_slice(&language_values);
    vision_request.setRecognitionLanguages(&languages);

    let request_array = NSArray::<VNRequest>::from_retained_slice(&[vision_request
        .clone()
        .into_super()
        .into_super()]);
    handler
        .performRequests_error(&request_array)
        .map_err(|error| {
            AppError::System(format!(
                "System OCR failed: {}",
                error.localizedDescription()
            ))
        })?;

    let Some(observations) = vision_request.results() else {
        return Ok(OcrResult {
            text: String::new(),
            confidence: None,
            lines: Vec::new(),
            detected_language: None,
            provider_id: None,
        });
    };

    Ok(ocr_result_from_observations(&observations))
}

const OCR_RECOVERY_CONFIDENCE_THRESHOLD: f32 = 0.62;

fn should_attempt_recovery(result: &OcrResult) -> bool {
    result.text.trim().is_empty()
        || result
            .confidence
            .is_some_and(|confidence| confidence < OCR_RECOVERY_CONFIDENCE_THRESHOLD)
}

fn is_better_result(candidate: &OcrResult, baseline: &OcrResult) -> bool {
    let candidate_chars = candidate
        .text
        .chars()
        .filter(|character| !character.is_whitespace())
        .count();
    let baseline_chars = baseline
        .text
        .chars()
        .filter(|character| !character.is_whitespace())
        .count();
    if candidate_chars == 0 {
        return false;
    }
    if baseline_chars == 0 {
        return true;
    }

    candidate.confidence.unwrap_or_default() >= baseline.confidence.unwrap_or_default() + 0.05
        || candidate_chars >= baseline_chars + 4
}

fn enhance_image_for_recovery(image_data: &[u8]) -> Result<Option<Vec<u8>>> {
    let image = match image::load_from_memory(image_data) {
        Ok(image) => image,
        Err(_) => return Ok(None),
    };
    if image.width() == 0 || image.height() == 0 {
        return Ok(None);
    }

    let mut enhanced = image.grayscale().adjust_contrast(35.0);
    let max_dimension = enhanced.width().max(enhanced.height());
    if max_dimension < 1600 {
        let scale = 1600.0 / max_dimension as f32;
        enhanced = enhanced.resize(
            (enhanced.width() as f32 * scale).round() as u32,
            (enhanced.height() as f32 * scale).round() as u32,
            image::imageops::FilterType::Lanczos3,
        );
    }

    let mut encoded = Cursor::new(Vec::new());
    enhanced
        .write_to(&mut encoded, image::ImageFormat::Png)
        .map_err(|error| {
            AppError::Other(format!("Failed to prepare OCR recovery image: {error}"))
        })?;
    Ok(Some(encoded.into_inner()))
}

fn ocr_result_from_observations(
    observations: &NSArray<objc2_vision::VNRecognizedTextObservation>,
) -> OcrResult {
    let mut lines = Vec::new();
    let mut confidence_sum = 0.0f32;
    let mut confidence_count = 0usize;

    for observation in observations.to_vec() {
        let candidates = observation.topCandidates(1);
        let Some(candidate) = candidates.to_vec().into_iter().next() else {
            continue;
        };

        let text = candidate.string().to_string();
        if text.trim().is_empty() {
            continue;
        }

        confidence_sum += candidate.confidence();
        confidence_count += 1;
        let bounding_box = unsafe {
            <_ as AsRef<VNDetectedObjectObservation>>::as_ref(&observation).boundingBox()
        };
        lines.push(OcrLine {
            text,
            confidence: Some(candidate.confidence()),
            bounding_box: Some(OcrBoundingBox {
                x: bounding_box.origin.x as f32,
                y: bounding_box.origin.y as f32,
                width: bounding_box.size.width as f32,
                height: bounding_box.size.height as f32,
            }),
        });
    }

    lines.sort_by(|left, right| {
        right
            .bounding_box
            .map(|box_| box_.y)
            .partial_cmp(&left.bounding_box.map(|box_| box_.y))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                left.bounding_box
                    .map(|box_| box_.x)
                    .partial_cmp(&right.bounding_box.map(|box_| box_.x))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    let text = lines
        .iter()
        .map(|line| line.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    OcrResult {
        text,
        confidence: if confidence_count == 0 {
            None
        } else {
            Some(confidence_sum / confidence_count as f32)
        },
        lines,
        detected_language: None,
        provider_id: None,
    }
}

pub(crate) fn vision_languages_for_request(requested_language: Option<&str>) -> Vec<String> {
    match requested_language.map(normalize_language_code).as_deref() {
        Some("zh") | Some("zh-cn") | Some("zh-hans") | Some("cn") | None => {
            vec!["zh-Hans".to_string(), "en-US".to_string()]
        }
        Some("zh-tw") | Some("zh-hk") | Some("zh-hant") => {
            vec!["zh-Hant".to_string(), "en-US".to_string()]
        }
        Some("en") | Some("en-us") | Some("en-gb") => vec!["en-US".to_string()],
        Some("ja") | Some("ja-jp") => vec!["ja-JP".to_string(), "en-US".to_string()],
        Some("ko") | Some("ko-kr") => vec!["ko-KR".to_string(), "en-US".to_string()],
        Some(language) => vec![language.to_string()],
    }
}

fn normalize_language_code(language: &str) -> String {
    language.trim().replace('_', "-").to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vision_languages_default_to_chinese_and_english() {
        assert_eq!(
            vision_languages_for_request(None),
            vec!["zh-Hans".to_string(), "en-US".to_string()]
        );
    }

    #[test]
    fn vision_languages_map_explicit_chinese_hint() {
        assert_eq!(
            vision_languages_for_request(Some("zh-CN")),
            vec!["zh-Hans".to_string(), "en-US".to_string()]
        );
    }

    #[test]
    fn recovery_only_runs_for_empty_or_low_confidence_results() {
        assert!(should_attempt_recovery(&OcrResult::from_text("", None)));
        assert!(should_attempt_recovery(&OcrResult::from_text(
            "text",
            Some(0.4),
        )));
        assert!(!should_attempt_recovery(&OcrResult::from_text(
            "text",
            Some(0.9),
        )));
    }

    #[test]
    fn recovery_prefers_more_complete_or_confident_results() {
        let baseline = OcrResult::from_text("short", Some(0.5));
        let confident = OcrResult::from_text("short", Some(0.6));
        let expanded = OcrResult::from_text("a much longer result", Some(0.5));
        let empty = OcrResult::from_text("", Some(0.99));

        assert!(is_better_result(&confident, &baseline));
        assert!(is_better_result(&expanded, &baseline));
        assert!(!is_better_result(&empty, &baseline));
    }
}

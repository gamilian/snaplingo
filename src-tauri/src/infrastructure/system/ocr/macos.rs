use crate::application::providers::ocr::SystemOcrEngine;
use crate::domain::ocr::{OcrRequest, OcrResult};
use crate::{AppError, Result};
use objc2::runtime::AnyObject;
use objc2::AnyThread;
use objc2_foundation::{NSArray, NSData, NSDictionary, NSString};
use objc2_vision::{
    VNImageOption, VNImageRequestHandler, VNRecognizeTextRequest, VNRequest,
    VNRequestTextRecognitionLevel,
};

pub struct MacOSVisionOcrEngine;

impl MacOSVisionOcrEngine {
    pub fn new() -> Self {
        Self
    }
}

impl SystemOcrEngine for MacOSVisionOcrEngine {
    fn recognize(&self, request: &OcrRequest) -> Result<OcrResult> {
        recognize_with_vision(request)
    }
}

fn recognize_with_vision(request: &OcrRequest) -> Result<OcrResult> {
    let image_data = NSData::with_bytes(&request.image_data);
    let options = NSDictionary::<VNImageOption, AnyObject>::new();
    let handler = VNImageRequestHandler::initWithData_options(
        VNImageRequestHandler::alloc(),
        &image_data,
        &options,
    );

    let vision_request = VNRecognizeTextRequest::new();
    vision_request.setRecognitionLevel(VNRequestTextRecognitionLevel::Accurate);
    vision_request.setUsesLanguageCorrection(true);
    vision_request.setAutomaticallyDetectsLanguage(true);

    let language_values: Vec<_> = vision_languages_for_request(request.language.as_deref())
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
        });
    };

    Ok(ocr_result_from_observations(&observations))
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
        lines.push(text);
    }

    OcrResult {
        text: lines.join("\n"),
        confidence: if confidence_count == 0 {
            None
        } else {
            Some(confidence_sum / confidence_count as f32)
        },
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
}

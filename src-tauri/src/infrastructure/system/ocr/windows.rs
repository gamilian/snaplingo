use std::thread;

use crate::application::providers::ocr::SystemOcrEngine;
use crate::domain::ocr::{OcrRequest, OcrResult};
use crate::{AppError, Result};
use windows::core::HSTRING;
use windows::Globalization::Language;
use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
use windows::Media::Ocr::OcrEngine;
use windows::Storage::Streams::DataWriter;
use windows::Win32::System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED};

/// Windows Runtime OCR backed by the language packs installed for the current user.
pub struct WindowsOcrEngine;

impl WindowsOcrEngine {
    pub fn new() -> Self {
        Self
    }
}

impl SystemOcrEngine for WindowsOcrEngine {
    fn is_available(&self) -> bool {
        run_on_windows_ocr_thread(|| create_ocr_engine(None).map(|_| ())).is_ok()
    }

    fn recognize(&self, request: &OcrRequest) -> Result<OcrResult> {
        let image_data = request.image_data.clone();
        let language = request.language.clone();
        run_on_windows_ocr_thread(move || {
            recognize_with_windows_runtime(&image_data, language.as_deref())
        })
    }
}

fn run_on_windows_ocr_thread<T, F>(operation: F) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T> + Send + 'static,
{
    thread::Builder::new()
        .name("snaplingo-windows-ocr".to_string())
        .spawn(move || {
            unsafe {
                RoInitialize(RO_INIT_MULTITHREADED).map_err(|error| {
                    AppError::System(format!("Failed to initialize Windows OCR: {error}"))
                })?;
            }
            let _runtime = WindowsRuntimeGuard;
            operation()
        })
        .map_err(|error| AppError::System(format!("Failed to start Windows OCR thread: {error}")))?
        .join()
        .map_err(|_| AppError::System("Windows OCR thread panicked".to_string()))?
}

fn recognize_with_windows_runtime(
    image_data: &[u8],
    requested_language: Option<&str>,
) -> Result<OcrResult> {
    let (pixels, width, height) = bgra_pixels_from_image_data(image_data)?;
    let writer = DataWriter::new().map_err(|error| {
        AppError::System(format!("Failed to create Windows OCR buffer: {error}"))
    })?;
    writer.WriteBytes(&pixels).map_err(|error| {
        AppError::System(format!("Failed to write Windows OCR buffer: {error}"))
    })?;
    let buffer = writer.DetachBuffer().map_err(|error| {
        AppError::System(format!("Failed to finalize Windows OCR buffer: {error}"))
    })?;
    let bitmap =
        SoftwareBitmap::CreateCopyFromBuffer(&buffer, BitmapPixelFormat::Bgra8, width, height)
            .map_err(|error| {
                AppError::System(format!("Failed to create Windows OCR bitmap: {error}"))
            })?;

    let engine = create_ocr_engine(requested_language)?;
    let result = engine
        .RecognizeAsync(&bitmap)
        .and_then(|operation| operation.get())
        .map_err(|error| AppError::System(format!("Windows OCR failed: {error}")))?;
    let text = result
        .Text()
        .map_err(|error| AppError::System(format!("Failed to read Windows OCR result: {error}")))?
        .to_string();

    Ok(OcrResult {
        text,
        confidence: None,
    })
}

fn create_ocr_engine(requested_language: Option<&str>) -> Result<OcrEngine> {
    if let Some(language_tag) = requested_language.and_then(normalize_language_tag) {
        let language = Language::CreateLanguage(&HSTRING::from(language_tag))
            .map_err(|error| AppError::System(format!("Invalid Windows OCR language: {error}")))?;
        if let Ok(engine) = OcrEngine::TryCreateFromLanguage(&language) {
            return Ok(engine);
        }
    }

    OcrEngine::TryCreateFromUserProfileLanguages().map_err(|error| {
        AppError::System(format!(
            "Windows OCR is unavailable; install a supported language pack: {error}"
        ))
    })
}

fn normalize_language_tag(language: &str) -> Option<String> {
    let normalized = language.trim().replace('_', "-").to_lowercase();
    if normalized.is_empty() {
        return None;
    }

    Some(
        match normalized.as_str() {
            "zh" | "zh-cn" | "zh-hans" | "cn" => "zh-Hans",
            "zh-tw" | "zh-hk" | "zh-hant" => "zh-Hant",
            "en" | "en-us" => "en-US",
            "en-gb" => "en-GB",
            "ja" | "ja-jp" => "ja-JP",
            "ko" | "ko-kr" => "ko-KR",
            other => other,
        }
        .to_string(),
    )
}

fn bgra_pixels_from_image_data(image_data: &[u8]) -> Result<(Vec<u8>, i32, i32)> {
    let image = image::load_from_memory(image_data)
        .map_err(|error| AppError::Other(format!("Invalid Windows OCR image data: {error}")))?;
    let max_dimension = OcrEngine::MaxImageDimension().unwrap_or(u32::MAX).max(1);
    let image = if image.width().max(image.height()) > max_dimension {
        image.thumbnail(max_dimension, max_dimension)
    } else {
        image
    };
    let width = i32::try_from(image.width())
        .map_err(|_| AppError::Other("Windows OCR image width is too large".to_string()))?;
    let height = i32::try_from(image.height())
        .map_err(|_| AppError::Other("Windows OCR image height is too large".to_string()))?;
    if width == 0 || height == 0 {
        return Err(AppError::Other(
            "Invalid Windows OCR image data: image dimensions must be non-zero".to_string(),
        ));
    }

    let rgba = image.to_rgba8();
    let mut bgra = rgba.into_raw();
    for pixel in bgra.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }

    Ok((bgra, width, height))
}

struct WindowsRuntimeGuard;

impl Drop for WindowsRuntimeGuard {
    fn drop(&mut self) {
        unsafe {
            RoUninitialize();
        }
    }
}

#[cfg(test)]
mod tests {
    use image::{ImageBuffer, ImageFormat, Rgba};

    use super::{bgra_pixels_from_image_data, normalize_language_tag};

    #[test]
    fn normalizes_common_windows_ocr_language_tags() {
        assert_eq!(normalize_language_tag("zh_CN"), Some("zh-Hans".to_string()));
        assert_eq!(normalize_language_tag("en"), Some("en-US".to_string()));
        assert_eq!(normalize_language_tag("  "), None);
    }

    #[test]
    fn converts_rgba_pixels_to_bgra_for_windows_runtime() {
        let image = ImageBuffer::from_pixel(1, 1, Rgba::<u8>([1, 2, 3, 4]));
        let mut encoded = Vec::new();
        image
            .write_to(&mut std::io::Cursor::new(&mut encoded), ImageFormat::Png)
            .unwrap();

        let (pixels, width, height) = bgra_pixels_from_image_data(&encoded).unwrap();

        assert_eq!((width, height), (1, 1));
        assert_eq!(pixels, vec![3, 2, 1, 4]);
    }
}

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;
use std::time::Duration;

use crate::application::providers::ocr::SystemOcrEngine;
use crate::domain::ocr::{OcrRequest, OcrResult};
use crate::{AppError, Result};
use windows::core::HSTRING;
use windows::Foundation::{AsyncOperationCompletedHandler, AsyncStatus};
use windows::Globalization::Language;
use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
use windows::Media::Ocr::OcrEngine;
use windows::Storage::Streams::DataWriter;
use windows::Win32::System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED};

use super::windows_image::bgra_pixels_from_image_data_with_limit;

const OCR_COMPLETION_TIMEOUT: Duration = Duration::from_secs(30);
const OCR_REQUEST_TIMEOUT: Duration = Duration::from_secs(40);
const OCR_AVAILABILITY_TIMEOUT: Duration = Duration::from_secs(10);

/// Windows Runtime OCR backed by the language packs installed for the current user.
pub struct WindowsOcrEngine {
    worker: WindowsOcrWorker,
}

impl WindowsOcrEngine {
    pub fn new() -> Self {
        Self {
            worker: WindowsOcrWorker::spawn(),
        }
    }
}

impl SystemOcrEngine for WindowsOcrEngine {
    fn is_available(&self) -> bool {
        self.worker.is_available()
    }

    fn recognize(&self, request: &OcrRequest) -> Result<OcrResult> {
        self.worker.recognize(request)
    }
}

enum WindowsOcrJob {
    CheckAvailability(SyncSender<Result<()>>),
    Recognize {
        image_data: Vec<u8>,
        language: Option<String>,
        reply: SyncSender<Result<OcrResult>>,
    },
}

enum WindowsOcrWorker {
    Ready(SyncSender<WindowsOcrJob>),
    Failed(String),
}

impl WindowsOcrWorker {
    fn spawn() -> Self {
        let (sender, receiver) = sync_channel(1);
        match thread::Builder::new()
            .name("snaplingo-windows-ocr".to_string())
            .spawn(move || run_windows_ocr_worker(receiver))
        {
            Ok(_) => Self::Ready(sender),
            Err(error) => Self::Failed(format!("Failed to start Windows OCR thread: {error}")),
        }
    }

    fn is_available(&self) -> bool {
        let Self::Ready(sender) = self else {
            return false;
        };
        let (reply, receiver) = sync_channel(1);
        if sender
            .try_send(WindowsOcrJob::CheckAvailability(reply))
            .is_err()
        {
            return false;
        }
        receiver
            .recv_timeout(OCR_AVAILABILITY_TIMEOUT)
            .map(|result| result.is_ok())
            .unwrap_or(false)
    }

    fn recognize(&self, request: &OcrRequest) -> Result<OcrResult> {
        let sender = match self {
            Self::Ready(sender) => sender,
            Self::Failed(error) => return Err(AppError::System(error.clone())),
        };
        let (reply, receiver) = sync_channel(1);
        sender
            .try_send(WindowsOcrJob::Recognize {
                image_data: request.image_data.clone(),
                language: request.language.clone(),
                reply,
            })
            .map_err(|error| match error {
                std::sync::mpsc::TrySendError::Full(_) => {
                    AppError::System("Windows OCR worker is busy".to_string())
                }
                std::sync::mpsc::TrySendError::Disconnected(_) => {
                    AppError::System("Windows OCR worker stopped".to_string())
                }
            })?;
        receiver
            .recv_timeout(OCR_REQUEST_TIMEOUT)
            .map_err(|error| match error {
                std::sync::mpsc::RecvTimeoutError::Timeout => {
                    AppError::System("Windows OCR request timed out".to_string())
                }
                std::sync::mpsc::RecvTimeoutError::Disconnected => {
                    AppError::System("Windows OCR worker stopped".to_string())
                }
            })?
    }
}

fn run_windows_ocr_worker(receiver: Receiver<WindowsOcrJob>) {
    let initialization = unsafe { RoInitialize(RO_INIT_MULTITHREADED) };
    let initialization_error = initialization
        .as_ref()
        .err()
        .map(|error| format!("Failed to initialize Windows OCR: {error}"));
    let _runtime = initialization.ok().map(|_| WindowsRuntimeGuard);

    for job in receiver {
        match job {
            WindowsOcrJob::CheckAvailability(reply) => {
                let result = initialization_error.as_ref().map_or_else(
                    || catch_ocr_panic(|| create_ocr_engine(None).map(|_| ())),
                    |error| Err(AppError::System(error.clone())),
                );
                let _ = reply.send(result);
            }
            WindowsOcrJob::Recognize {
                image_data,
                language,
                reply,
            } => {
                let result = initialization_error.as_ref().map_or_else(
                    || {
                        catch_ocr_panic(|| {
                            recognize_with_windows_runtime(&image_data, language.as_deref())
                        })
                    },
                    |error| Err(AppError::System(error.clone())),
                );
                let _ = reply.send(result);
            }
        }
    }
}

fn catch_ocr_panic<T>(operation: impl FnOnce() -> Result<T>) -> Result<T> {
    catch_unwind(AssertUnwindSafe(operation))
        .unwrap_or_else(|_| Err(AppError::System("Windows OCR worker panicked".to_string())))
}

fn recognize_with_windows_runtime(
    image_data: &[u8],
    requested_language: Option<&str>,
) -> Result<OcrResult> {
    let (pixels, width, height) = bgra_pixels_from_image_data(image_data)?;
    let buffer = {
        let writer = DataWriter::new().map_err(|error| {
            AppError::System(format!("Failed to create Windows OCR buffer: {error}"))
        })?;
        writer.WriteBytes(&pixels).map_err(|error| {
            AppError::System(format!("Failed to write Windows OCR buffer: {error}"))
        })?;
        writer.DetachBuffer().map_err(|error| {
            AppError::System(format!("Failed to finalize Windows OCR buffer: {error}"))
        })?
    };
    drop(pixels);
    let bitmap =
        SoftwareBitmap::CreateCopyFromBuffer(&buffer, BitmapPixelFormat::Bgra8, width, height)
            .map_err(|error| {
                AppError::System(format!("Failed to create Windows OCR bitmap: {error}"))
            })?;
    drop(buffer);

    let engine = create_ocr_engine(requested_language)?;
    let operation = engine
        .RecognizeAsync(&bitmap)
        .map_err(|error| AppError::System(format!("Failed to start Windows OCR: {error}")))?;
    let result = wait_for_ocr_operation(operation)?;
    let text = result
        .Text()
        .map_err(|error| AppError::System(format!("Failed to read Windows OCR result: {error}")))?
        .to_string();

    let detected_language = engine
        .RecognizerLanguage()
        .ok()
        .and_then(|language| language.LanguageTag().ok())
        .map(|language| language.to_string());
    let mut result = OcrResult::from_text(text, None);
    result.detected_language = detected_language;
    Ok(result)
}

fn wait_for_ocr_operation(
    operation: windows::Foundation::IAsyncOperation<windows::Media::Ocr::OcrResult>,
) -> Result<windows::Media::Ocr::OcrResult> {
    let cancel_operation = operation.clone();
    let (completed_sender, completed_receiver) = sync_channel(1);
    operation
        .SetCompleted(&AsyncOperationCompletedHandler::new(
            move |_operation, status| {
                let _ = completed_sender.send(status);
                Ok(())
            },
        ))
        .map_err(|error| {
            AppError::System(format!(
                "Failed to subscribe to Windows OCR completion: {error}"
            ))
        })?;

    let status = match completed_receiver.recv_timeout(OCR_COMPLETION_TIMEOUT) {
        Ok(status) => status,
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
            let _ = cancel_operation.Cancel();
            return Err(AppError::System(
                "Windows OCR recognition timed out after 30 seconds".to_string(),
            ));
        }
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
            return Err(AppError::System(
                "Windows OCR completion callback disconnected".to_string(),
            ));
        }
    };

    match status {
        AsyncStatus::Completed => operation
            .GetResults()
            .map_err(|error| AppError::System(format!("Windows OCR failed: {error}"))),
        AsyncStatus::Canceled => Err(AppError::System("Windows OCR was canceled".to_string())),
        AsyncStatus::Error => match operation.ErrorCode() {
            Ok(error) => Err(AppError::System(format!("Windows OCR failed: {error}"))),
            Err(error) => Err(AppError::System(format!(
                "Failed to read Windows OCR error: {error}"
            ))),
        },
        _ => Err(AppError::System(
            "Windows OCR returned an invalid completion status".to_string(),
        )),
    }
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
    let max_dimension = OcrEngine::MaxImageDimension().unwrap_or(4096).max(1);
    bgra_pixels_from_image_data_with_limit(image_data, max_dimension)
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
    use super::{catch_ocr_panic, normalize_language_tag};

    #[test]
    fn normalizes_common_windows_ocr_language_tags() {
        assert_eq!(normalize_language_tag("zh_CN"), Some("zh-Hans".to_string()));
        assert_eq!(normalize_language_tag("en"), Some("en-US".to_string()));
        assert_eq!(normalize_language_tag("  "), None);
    }

    #[test]
    fn converts_worker_panics_to_ocr_errors() {
        let result = catch_ocr_panic::<()>(|| panic!("native projection failed"));

        assert!(
            matches!(result, Err(crate::AppError::System(message)) if message == "Windows OCR worker panicked")
        );
        assert!(catch_ocr_panic(|| Ok(())).is_ok());
    }
}

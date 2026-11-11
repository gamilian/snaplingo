//! One OCR request per process, so native model allocations cannot outlive it.
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

use super::mnn_engine::MnnSession;
use crate::domain::ocr::{OcrRequest, OcrResult};
use crate::{AppError, Result};

const WORKER_ARGUMENT: &str = "--ocr-small-worker";
type WorkerResult = std::result::Result<OcrResult, String>;

pub(super) fn recognize(root: &Path, request: &OcrRequest) -> Result<OcrResult> {
    let temporary = tempfile::Builder::new()
        .prefix("snaplingo-ocr-")
        .tempdir()?;
    let input = temporary.path().join("input");
    let response = temporary.path().join("result.json");
    std::fs::write(&input, &request.image_data)?;
    let status = Command::new(std::env::current_exe()?)
        .arg(WORKER_ARGUMENT)
        .arg(root)
        .arg(&input)
        .arg(&response)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|error| AppError::System(format!("Could not start OCR worker: {error}")))?;
    let data = std::fs::read(&response).map_err(|error| {
        AppError::System(format!(
            "OCR worker exited with {status} without a result: {error}"
        ))
    })?;
    decode_response(status, &data)
}

fn decode_response(status: ExitStatus, data: &[u8]) -> Result<OcrResult> {
    let result: WorkerResult = serde_json::from_slice(data).map_err(|error| {
        AppError::InvalidResponse(format!("Invalid OCR worker result: {error}"))
    })?;
    match result {
        Ok(result) if status.success() => Ok(result),
        Ok(_) => Err(AppError::System(format!("OCR worker exited with {status}"))),
        Err(error) => Err(AppError::Other(error)),
    }
}

struct WorkerPaths {
    root: PathBuf,
    input: PathBuf,
    response: PathBuf,
}

fn worker_paths(arguments: &[OsString]) -> Option<std::result::Result<WorkerPaths, &'static str>> {
    if arguments
        .first()
        .is_none_or(|argument| argument != WORKER_ARGUMENT)
    {
        return None;
    }
    Some(match arguments {
        [_, root, input, response] => Ok(WorkerPaths {
            root: root.into(),
            input: input.into(),
            response: response.into(),
        }),
        _ => Err("OCR worker requires resource, input and result paths"),
    })
}

/// Called before Tauri, the single-instance plugin, database and UI startup.
pub(crate) fn run_if_requested() -> Option<i32> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    worker_paths(&arguments).map(|paths| match paths {
        Ok(paths) => run_worker(&paths),
        Err(error) => {
            eprintln!("{error}");
            2
        }
    })
}

fn run_worker(paths: &WorkerPaths) -> i32 {
    let result: WorkerResult = (|| {
        let data = std::fs::read(&paths.input)?;
        let image = image::load_from_memory(&data)
            .map_err(|error| AppError::Other(format!("Invalid OCR image: {error}")))?
            .to_rgb8();
        let mut session = MnnSession::new(&paths.root)?;
        session.recognize(&image)
    })()
    .map_err(|error: AppError| error.to_string());
    let code = if result.is_ok() { 0 } else { 1 };
    let written = serde_json::to_vec(&result)
        .map_err(AppError::from)
        .and_then(|data| std::fs::write(&paths.response, data).map_err(AppError::from));
    if let Err(error) = written {
        eprintln!("Could not write OCR worker result: {error}");
        return 1;
    }
    code
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use std::os::unix::process::ExitStatusExt;
    #[cfg(windows)]
    use std::os::windows::process::ExitStatusExt;

    fn exit_status(code: u32) -> ExitStatus {
        #[cfg(unix)]
        return ExitStatus::from_raw((code as i32) << 8);
        #[cfg(windows)]
        return ExitStatus::from_raw(code);
    }

    #[test]
    fn ordinary_launch_arguments_do_not_start_an_ocr_worker() {
        assert!(worker_paths(&[]).is_none());
        assert!(worker_paths(&["--other".into(), WORKER_ARGUMENT.into()]).is_none());
        assert!(worker_paths(&[WORKER_ARGUMENT.into()]).unwrap().is_err());
        assert!(worker_paths(&[
            WORKER_ARGUMENT.into(),
            "root".into(),
            "input".into(),
            "output".into(),
            "extra".into(),
        ])
        .unwrap()
        .is_err());
    }

    #[test]
    fn invalid_image_reports_an_error_without_initializing_models() {
        let temporary = tempfile::tempdir().unwrap();
        let paths = WorkerPaths {
            root: temporary.path().join("missing-models"),
            input: temporary.path().join("input"),
            response: temporary.path().join("result"),
        };
        std::fs::write(&paths.input, b"invalid image").unwrap();
        assert_eq!(run_worker(&paths), 1);
        let result = decode_response(exit_status(1), &std::fs::read(paths.response).unwrap());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid OCR image"));
    }

    #[test]
    fn unsuccessful_exit_cannot_return_a_successful_result() {
        let result: WorkerResult = Ok(OcrResult::from_text("text", Some(0.9)));
        let data = serde_json::to_vec(&result).unwrap();
        assert!(decode_response(exit_status(1), &data).is_err());
        assert!(decode_response(exit_status(0), b"incomplete json").is_err());
        assert_eq!(decode_response(exit_status(0), &data).unwrap().text, "text");
    }
}

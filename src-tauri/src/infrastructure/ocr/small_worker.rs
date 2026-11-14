//! One OCR request per process, so native model allocations cannot outlive it.
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Stdio};
use std::time::Duration;
use tokio::process::Command;

use super::mnn_engine::MnnSession;
use crate::domain::ocr::{OcrRequest, OcrResult};
use crate::{AppError, Result};

const WORKER_ARGUMENT: &str = "--ocr-small-worker";
type WorkerResult = std::result::Result<OcrResult, String>;

pub(super) async fn recognize(root: &Path, request: &OcrRequest) -> Result<OcrResult> {
    let temporary = tempfile::Builder::new()
        .prefix("snaplingo-ocr-")
        .tempdir()?;
    let input = temporary.path().join("input");
    let response = temporary.path().join("result.json");
    tokio::fs::write(&input, &request.image_data).await?;
    let mut command = Command::new(std::env::current_exe()?);
    command
        .arg(WORKER_ARGUMENT)
        .arg(root)
        .arg(&input)
        .arg(&response)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    wait_for_worker(command, &response, Duration::from_secs(60)).await
}

async fn wait_for_worker(
    mut command: Command,
    response: &Path,
    timeout: Duration,
) -> Result<OcrResult> {
    let mut child = command
        .kill_on_drop(true)
        .spawn()
        .map_err(|error| AppError::System(format!("Could not start OCR worker: {error}")))?;
    let status = match tokio::time::timeout(timeout, child.wait()).await {
        Ok(result) => result?,
        Err(_) => {
            child.kill().await?;
            return Err(AppError::System("OCR worker timed out".into()));
        }
    };
    let data = tokio::fs::read(response).await.map_err(|error| {
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

    #[test]
    fn worker_process_fixture() {
        let Some(root) = std::env::var_os("SNAPLINGO_WORKER_TEST_ROOT") else {
            return;
        };
        let root = PathBuf::from(root);
        if std::env::var("SNAPLINGO_WORKER_TEST_MODE").unwrap() == "success" {
            let result: WorkerResult = Ok(OcrResult::from_text("worker text", Some(0.9)));
            std::fs::write(root.join("result"), serde_json::to_vec(&result).unwrap()).unwrap();
            return;
        }
        // Keeping a port bound lets the parent observe process death on every platform.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        std::fs::write(
            root.join("ready"),
            listener.local_addr().unwrap().to_string(),
        )
        .unwrap();
        std::thread::sleep(Duration::from_secs(30));
        std::fs::write(root.join("finished"), b"finished").unwrap();
        drop(listener);
    }

    fn fixture_command(root: &Path, mode: &str) -> Command {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .arg("--exact")
            .arg("infrastructure::ocr::small_worker::tests::worker_process_fixture")
            .env("SNAPLINGO_WORKER_TEST_ROOT", root)
            .env("SNAPLINGO_WORKER_TEST_MODE", mode)
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        command
    }

    async fn ready_address(root: &Path) -> String {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let Ok(address) = tokio::fs::read_to_string(root.join("ready")).await {
                    if address.parse::<std::net::SocketAddr>().is_ok() {
                        return address;
                    }
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("worker must start")
    }

    async fn assert_worker_stopped(address: &str, root: &Path) {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if std::net::TcpListener::bind(address).is_ok() {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("worker process must release its port");
        assert!(!root.join("finished").exists());
    }

    #[tokio::test]
    async fn cancelling_a_request_terminates_its_worker_process() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().to_path_buf();
        let command = fixture_command(&root, "pending");
        let request = tokio::spawn(async move {
            wait_for_worker(command, &root.join("result"), Duration::from_secs(30)).await
        });
        let address = ready_address(temporary.path()).await;
        request.abort();
        assert!(request.await.unwrap_err().is_cancelled());
        assert_worker_stopped(&address, temporary.path()).await;
    }

    #[tokio::test]
    async fn timeout_terminates_and_reaps_the_worker_process() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().to_path_buf();
        let command = fixture_command(&root, "pending");
        let request = tokio::spawn(async move {
            wait_for_worker(command, &root.join("result"), Duration::from_secs(2)).await
        });
        let address = ready_address(temporary.path()).await;
        assert!(request
            .await
            .unwrap()
            .unwrap_err()
            .to_string()
            .contains("timed out"));
        assert_worker_stopped(&address, temporary.path()).await;
    }

    #[tokio::test]
    async fn successful_worker_process_returns_the_protocol_result() {
        let temporary = tempfile::tempdir().unwrap();
        let result = wait_for_worker(
            fixture_command(temporary.path(), "success"),
            &temporary.path().join("result"),
            Duration::from_secs(5),
        )
        .await
        .unwrap();
        assert_eq!(result.text, "worker text");
        assert_eq!(result.confidence, Some(0.9));
    }
}

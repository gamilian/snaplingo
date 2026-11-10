use super::*;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

struct FakeFactory {
    creates: Mutex<Vec<OcrBackend>>,
    fail_gpu_init: bool,
    fail_gpu_run: AtomicBool,
    runs: Arc<AtomicUsize>,
}

struct FakeSession {
    backend: OcrBackend,
    fail: bool,
    runs: Arc<AtomicUsize>,
}

impl MediumSession for FakeSession {
    fn recognize(&mut self, _: &RgbImage) -> Result<OcrResult> {
        self.runs.fetch_add(1, Ordering::SeqCst);
        if self.fail {
            return Err(AppError::Other("device lost".into()));
        }
        Ok(OcrResult::from_text(
            format!("{:?}", self.backend),
            Some(0.9),
        ))
    }
}

impl SessionFactory for FakeFactory {
    fn is_available(&self) -> bool {
        true
    }
    fn cpu_backend(&self) -> OcrBackend {
        OcrBackend::OrtCpu
    }
    fn gpu_backend(&self) -> Option<OcrBackend> {
        Some(OcrBackend::CoreMl)
    }
    fn create(&self, backend: OcrBackend) -> Result<Box<dyn MediumSession>> {
        self.creates.lock().push(backend);
        if backend.is_gpu() && self.fail_gpu_init {
            return Err(AppError::Other("no GPU".into()));
        }
        Ok(Box::new(FakeSession {
            backend,
            fail: backend.is_gpu() && self.fail_gpu_run.load(Ordering::SeqCst),
            runs: self.runs.clone(),
        }))
    }
}

fn factory(init: bool, run: bool) -> Arc<FakeFactory> {
    Arc::new(FakeFactory {
        creates: Mutex::new(vec![]),
        fail_gpu_init: init,
        fail_gpu_run: AtomicBool::new(run),
        runs: Arc::new(AtomicUsize::new(0)),
    })
}

fn request() -> OcrRequest {
    let mut bytes = std::io::Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(RgbImage::new(32, 32))
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    OcrRequest {
        image_data: bytes.into_inner(),
        language: None,
    }
}

use image::DynamicImage;

#[test]
fn gpu_initialization_failure_falls_back_once_and_reuses_cpu_session() {
    let factory = factory(true, false);
    let engine = LocalPaddleOcrEngine::with_factory(factory.clone());
    for _ in 0..2 {
        assert_eq!(engine.recognize(&request(), true).unwrap().text, "OrtCpu");
    }
    assert_eq!(
        *factory.creates.lock(),
        vec![OcrBackend::CoreMl, OcrBackend::OrtCpu]
    );
    assert_eq!(engine.status().device, Some(OcrExecutionDevice::Cpu));
    assert!(engine.status().fallback_reason.is_some());
}

#[test]
fn gpu_inference_failure_retries_the_same_image_on_cpu() {
    let factory = factory(false, true);
    let engine = LocalPaddleOcrEngine::with_factory(factory.clone());
    assert_eq!(engine.recognize(&request(), true).unwrap().text, "OrtCpu");
    assert_eq!(factory.runs.load(Ordering::SeqCst), 2);
    engine.recognize(&request(), true).unwrap();
    assert_eq!(factory.creates.lock().len(), 2);
}

#[test]
fn gpu_toggle_replaces_the_session_and_can_retry_a_failed_gpu() {
    let factory = factory(false, true);
    let engine = LocalPaddleOcrEngine::with_factory(factory.clone());
    engine.recognize(&request(), true).unwrap();
    engine.recognize(&request(), false).unwrap();
    assert!(!engine.status().gpu_enabled);
    factory.fail_gpu_run.store(false, Ordering::SeqCst);
    assert_eq!(engine.recognize(&request(), true).unwrap().text, "CoreMl");
    assert!(engine.status().fallback_reason.is_none());
}

#[test]
fn invalid_image_never_initializes_an_engine() {
    let factory = factory(false, false);
    let engine = LocalPaddleOcrEngine::with_factory(factory.clone());
    assert!(engine
        .recognize(
            &OcrRequest {
                image_data: vec![1, 2, 3],
                language: None
            },
            true
        )
        .is_err());
    assert!(factory.creates.lock().is_empty());
}

#[test]
fn missing_bundle_resources_do_not_fall_back_to_the_working_directory() {
    let engine = LocalPaddleOcrEngine::new(None, PathBuf::new());
    assert!(!engine.is_available());
    assert!(engine
        .recognize(&request(), false)
        .unwrap_err()
        .to_string()
        .contains("resources are unavailable"));
}

#[cfg(target_os = "macos")]
#[test]
fn macos_tiny_is_available_without_ort_or_resident_model_sessions() {
    let root = tempfile::tempdir().unwrap();
    for name in ["tiny_det.mnn", "tiny_rec.mnn", "tiny_keys.txt"] {
        std::fs::write(root.path().join(name), name).unwrap();
    }
    let engine = LocalPaddleOcrEngine::new(Some(root.path().to_path_buf()), PathBuf::new());
    assert!(engine.is_available());
    assert!(!engine.supports_gpu());
    assert!(engine.state.lock().session.is_none());
    assert_eq!(engine.status().device, Some(OcrExecutionDevice::Cpu));
    std::fs::remove_file(root.path().join("tiny_keys.txt")).unwrap();
    assert!(!engine.is_available());
}

#[test]
fn platform_selection_keeps_old_macos_away_from_ort() {
    assert_eq!(Platform::MacArm(12).cpu(), OcrBackend::MnnCpu);
    assert_eq!(Platform::MacArm(13).gpu(), None);
    assert_eq!(Platform::MacArm(14).gpu(), Some(OcrBackend::CoreMl));
    assert_eq!(Platform::MacIntel.cpu(), OcrBackend::MnnCpu);
    assert_eq!(Platform::Windows.gpu(), Some(OcrBackend::DirectMl));
    assert_eq!(Platform::Other.cpu(), OcrBackend::OrtCpu);
}

#[test]
fn boxes_use_normalized_bottom_left_coordinates_and_clip_to_the_image() {
    let bbox = normalized_box(&[(-10.0, 10.0), (50.0, 30.0)], 100, 50).unwrap();
    assert_eq!(bbox.x, 0.0);
    assert!((bbox.y - 0.4).abs() < 0.00001);
    assert_eq!(bbox.width, 0.5);
    assert!((bbox.height - 0.4).abs() < 0.00001);
}

#[test]
#[ignore = "requires npm run ocr:prepare; exercises the bundled platform OCR model"]
fn bundled_medium_smoke() {
    let root = std::env::var_os("SNAPLINGO_OCR_SMOKE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/ocr"));
    let cache = tempfile::tempdir().unwrap();
    let engine = LocalPaddleOcrEngine::new(Some(root.clone()), cache.path().to_path_buf());
    assert!(engine.is_available());
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ocr-medium-small.png");
    let request = OcrRequest {
        image_data: std::fs::read(fixture).unwrap(),
        language: None,
    };
    #[cfg(not(target_os = "macos"))]
    for gpu in [false, true] {
        let result = engine.recognize(&request, gpu).unwrap();
        assert!(!result.text.is_empty(), "medium returned no text");
        assert!(result.lines.iter().all(|line| line.bounding_box.is_some()));
        println!("{:?}: {}", engine.status(), result.text);
    }
    #[cfg(target_os = "macos")]
    {
        // The installation smoke test exercises the signed worker process.
        // Here the test executable validates the exact CPU model directly.
        let mut session = mnn_engine::MnnMediumSession::new_tiny(&root).unwrap();
        let result = session
            .recognize(
                &image::load_from_memory(&request.image_data)
                    .unwrap()
                    .to_rgb8(),
            )
            .unwrap();
        assert!(!result.text.is_empty());
        assert!(result.lines.iter().all(|line| line.bounding_box.is_some()));
        println!("tiny MNN CPU: {}", result.text);
    }
}

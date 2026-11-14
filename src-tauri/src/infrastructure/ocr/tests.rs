use super::*;

#[tokio::test]
async fn request_queue_is_bounded_and_cancelled_waiters_release_admission() {
    let root = tempfile::tempdir().unwrap();
    let engine = std::sync::Arc::new(LocalPaddleOcrEngine::new(Some(root.path().to_path_buf())));
    let _blocked = engine.request_lock.lock().await;
    let spawn_request = || {
        let engine = engine.clone();
        tokio::spawn(async move {
            engine
                .recognize(
                    &OcrRequest {
                        image_data: vec![],
                        language: None,
                    },
                    false,
                )
                .await
        })
    };
    let first = spawn_request();
    let second = spawn_request();
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while engine.request_slots.available_permits() != 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let rejected = spawn_request().await.unwrap().unwrap_err();
    assert!(rejected.to_string().contains("OCR is busy"));
    second.abort();
    assert!(second.await.unwrap_err().is_cancelled());
    assert_eq!(engine.request_slots.available_permits(), 1);
    first.abort();
    assert!(first.await.unwrap_err().is_cancelled());
    assert_eq!(engine.request_slots.available_permits(), 2);
}

#[tokio::test]
async fn missing_bundle_resources_do_not_fall_back_to_the_working_directory() {
    let engine = LocalPaddleOcrEngine::new(None);
    assert!(!engine.is_available());
    assert!(!engine.supports_gpu());
    assert!(engine
        .recognize(
            &OcrRequest {
                image_data: vec![],
                language: None
            },
            true
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("resources are unavailable"));
}

#[test]
fn small_cpu_requires_both_models_and_the_matching_dictionary() {
    let root = tempfile::tempdir().unwrap();
    for name in ["tiny_det.mnn", "tiny_rec.mnn", "tiny_keys.txt"] {
        std::fs::write(root.path().join(name), name).unwrap();
    }
    let engine = LocalPaddleOcrEngine::new(Some(root.path().to_path_buf()));
    assert!(!engine.is_available());
    for name in ["small_det.mnn", "small_rec.mnn"] {
        std::fs::write(root.path().join(name), name).unwrap();
    }
    assert!(!engine.is_available());
    std::fs::write(root.path().join("small_keys.txt"), "dictionary").unwrap();
    assert!(engine.is_available());
    assert!(!engine.supports_gpu());
    assert!(!engine.status().gpu_enabled);
    assert_eq!(engine.status().device, Some(OcrExecutionDevice::Cpu));
    std::fs::remove_file(root.path().join("small_rec.mnn")).unwrap();
    assert!(!engine.is_available());
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
#[ignore = "requires npm run ocr:prepare; exercises the bundled small MNN CPU model"]
fn bundled_small_smoke() {
    let root = std::env::var_os("SNAPLINGO_OCR_SMOKE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/ocr"));
    let engine = LocalPaddleOcrEngine::new(Some(root.clone()));
    assert!(engine.is_available());
    // Installation smoke tests exercise the application's worker entrypoint.
    // The test executable validates the same CPU model directly on every platform.
    let mut session = mnn_engine::MnnSession::new(&root).unwrap();
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ocr-medium-small.png");
    let result = session
        .recognize(&image::open(fixture).unwrap().to_rgb8())
        .unwrap();
    assert!(!result.text.is_empty());
    assert!(result.lines.iter().all(|line| line.bounding_box.is_some()));
    println!("small MNN CPU: {}", result.text);
}

use super::*;
use crate::Result;
use async_trait::async_trait;
use base64::Engine;
use snaplingo_core::providers::common::CredentialField;
use snaplingo_core::providers::ocr::model::OcrRequest;
use snaplingo_core::providers::ocr::model::OcrResult;
use snaplingo_core::providers::ocr::OcrCoordinator;
use snaplingo_core::providers::ocr::*;
use snaplingo_core::providers::*;
use snaplingo_core::settings::*;
use std::sync::Arc;
mod cases {

    use super::FilesystemImageReader;
    use super::*;
    use snaplingo_core::providers::ocr::OcrProvider;
    use snaplingo_core::providers::Provider;
    use snaplingo_storage::SqliteConfigStore;
    use std::io::Write;
    use std::sync::Mutex;

    struct MemoryImageReader(Vec<u8>);
    #[async_trait]
    impl ImageFileReader for MemoryImageReader {
        async fn read(&self, _: &str) -> Result<Vec<u8>> {
            Ok(self.0.clone())
        }
    }

    struct Recognizer {
        requests: Arc<Mutex<Vec<OcrRequest>>>,
        fail: bool,
    }
    impl Provider for Recognizer {
        fn id(&self) -> &str {
            "test"
        }
        fn name(&self) -> &str {
            "Test OCR"
        }
        fn is_configured(&self) -> bool {
            true
        }
        fn requires_api_key(&self) -> bool {
            false
        }
    }
    #[async_trait]
    impl OcrProvider for Recognizer {
        async fn recognize(&self, request: &OcrRequest) -> Result<OcrResult> {
            self.requests.lock().unwrap().push(request.clone());
            if self.fail {
                return Err("recognition failed".into());
            }
            Ok(OcrResult {
                text: "recognized".into(),
                confidence: Some(0.9),
                detected_language: Some("en".into()),
                provider_id: Some("test".into()),
                lines: vec![],
            })
        }
    }
    fn workflow(
        fail: bool,
        reader: Arc<dyn ImageFileReader>,
    ) -> (FileOcr, Arc<Mutex<Vec<OcrRequest>>>) {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let ocr = Arc::new(OcrCoordinator::new(Arc::new(
            SqliteConfigStore::new_in_memory(),
        )));
        ocr.register(Recognizer {
            requests: requests.clone(),
            fail,
        })
        .unwrap();
        ocr.activate("test").unwrap();
        (FileOcr::new(reader, ocr), requests)
    }
    #[tokio::test]
    async fn preview_and_ocr_share_the_original_file_bytes() {
        let (workflow, requests) = workflow(false, Arc::new(FilesystemImageReader));
        for format in [
            image::ImageFormat::Png,
            image::ImageFormat::Jpeg,
            image::ImageFormat::WebP,
        ] {
            let mut bytes = std::io::Cursor::new(Vec::new());
            image::DynamicImage::new_rgb8(2, 2)
                .write_to(&mut bytes, format)
                .unwrap();
            let mut file = tempfile::NamedTempFile::new().unwrap();
            file.write_all(bytes.get_ref()).unwrap();
            let output = workflow
                .recognize(file.path().to_str().unwrap(), Some("fr".into()))
                .await
                .unwrap();
            let (header, data) = output.image_data_url.split_once(',').unwrap();
            assert_eq!(header, format!("data:{};base64", format.to_mime_type()));
            assert_eq!(
                base64::engine::general_purpose::STANDARD
                    .decode(data)
                    .unwrap(),
                *bytes.get_ref()
            );
            let request = requests.lock().unwrap().last().unwrap().clone();
            assert_eq!(request.image_data, *bytes.get_ref());
            assert_eq!(request.language.as_deref(), Some("fr"));
            assert_eq!(output.result.text, "recognized");
            assert_eq!(output.result.confidence, Some(0.9));
            assert_eq!(output.result.detected_language.as_deref(), Some("en"));
            assert_eq!(output.result.provider_id.as_deref(), Some("test"));
        }
    }
    #[tokio::test]
    async fn missing_files_do_not_start_recognition() {
        let (workflow, requests) = workflow(false, Arc::new(FilesystemImageReader));
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("missing.png");
        assert!(workflow
            .recognize(path.to_str().unwrap(), None)
            .await
            .is_err());
        assert!(requests.lock().unwrap().is_empty());
    }
    #[tokio::test]
    async fn recognition_errors_are_returned_to_the_caller() {
        let (workflow, _) = workflow(true, Arc::new(MemoryImageReader(vec![1, 2, 3])));
        let error = workflow
            .recognize("in-memory-image", None)
            .await
            .err()
            .unwrap();
        assert!(error.to_string().contains("recognition failed"));
    }
}

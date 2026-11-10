use std::sync::Arc;

use async_trait::async_trait;
use base64::Engine;

use super::OcrCoordinator;
use crate::providers::ocr::model::OcrRequest;
use crate::providers::ocr::model::OcrResult;
use crate::Result;

#[async_trait]
pub trait ImageFileReader: Send + Sync {
    async fn read(&self, path: &str) -> Result<Vec<u8>>;
}

pub struct RecognizedImageFile {
    pub result: OcrResult,
    pub image_data_url: String,
}

pub struct FileOcr {
    reader: Arc<dyn ImageFileReader>,
    ocr: Arc<OcrCoordinator>,
}

impl FileOcr {
    pub fn new(reader: Arc<dyn ImageFileReader>, ocr: Arc<OcrCoordinator>) -> Self {
        Self { reader, ocr }
    }

    pub async fn recognize(
        &self,
        path: &str,
        language: Option<String>,
    ) -> Result<RecognizedImageFile> {
        let image_data = self.reader.read(path).await?;
        let mime = match image::guess_format(&image_data) {
            Ok(image::ImageFormat::Jpeg) => "image/jpeg",
            Ok(image::ImageFormat::WebP) => "image/webp",
            Ok(image::ImageFormat::Bmp) => "image/bmp",
            Ok(image::ImageFormat::Tiff) => "image/tiff",
            _ => "image/png",
        };
        let image_data_url = format!(
            "data:{mime};base64,{}",
            base64::engine::general_purpose::STANDARD.encode(&image_data)
        );
        let result = self
            .ocr
            .recognize(&OcrRequest {
                image_data,
                language,
            })
            .await?;
        Ok(RecognizedImageFile {
            result,
            image_data_url,
        })
    }
}

use async_trait::async_trait;
use snaplingo_core::providers::ocr::ImageFileReader;

pub struct FilesystemImageReader;

#[async_trait]
impl ImageFileReader for FilesystemImageReader {
    async fn read(&self, path: &str) -> crate::Result<Vec<u8>> {
        tokio::fs::read(path).await.map_err(Into::into)
    }
}

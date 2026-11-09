use async_trait::async_trait;
use snaplingo_core::application::providers::ocr::ImageFileReader;

pub(crate) struct FilesystemImageReader;

#[async_trait]
impl ImageFileReader for FilesystemImageReader {
    async fn read(&self, path: &str) -> crate::Result<Vec<u8>> {
        tokio::fs::read(path).await.map_err(Into::into)
    }
}

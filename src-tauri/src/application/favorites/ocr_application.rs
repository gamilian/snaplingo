use std::sync::Arc;

use async_trait::async_trait;

use crate::application::favorites::Favorites;
use crate::application::history::History;
use crate::application::providers::ocr::OcrCoordinator;
use crate::domain::ocr::{OcrRequest, OcrResult};
use crate::Result;

#[async_trait]
pub trait OcrFavoriteHistory: Send + Sync {
    async fn read_ocr_source(&self, id: i64) -> Result<Vec<u8>>;
}

#[async_trait]
impl OcrFavoriteHistory for History {
    async fn read_ocr_source(&self, id: i64) -> Result<Vec<u8>> {
        History::read_ocr_source(self, id).await
    }
}

#[async_trait]
pub trait OcrFavoriteStore: Send + Sync {
    async fn add_ocr(
        &self,
        source_history_id: Option<i64>,
        image_data: Vec<u8>,
        language: Option<String>,
        provider_used: String,
        result: OcrResult,
    ) -> Result<i64>;

    async fn read_ocr_source(&self, id: i64) -> Result<Vec<u8>>;
}

#[async_trait]
impl OcrFavoriteStore for Favorites {
    async fn add_ocr(
        &self,
        source_history_id: Option<i64>,
        image_data: Vec<u8>,
        language: Option<String>,
        provider_used: String,
        result: OcrResult,
    ) -> Result<i64> {
        Favorites::add_ocr(
            self,
            source_history_id,
            image_data,
            language,
            provider_used,
            result,
        )
        .await
        .map(|record| record.id)
    }

    async fn read_ocr_source(&self, id: i64) -> Result<Vec<u8>> {
        Favorites::read_ocr_source(self, id).await
    }
}

#[async_trait]
pub trait OcrFavoriteRecognizer: Send + Sync {
    fn active_provider_id(&self) -> Option<String>;
    async fn recognize(&self, image: Vec<u8>) -> Result<OcrResult>;
}

#[async_trait]
impl OcrFavoriteRecognizer for OcrCoordinator {
    fn active_provider_id(&self) -> Option<String> {
        self.get_active()
            .map(|provider| provider.read().id().to_string())
    }

    async fn recognize(&self, image: Vec<u8>) -> Result<OcrResult> {
        self.recognize_image(image).await
    }
}

/// Owns the OCR favorite workflow across provider, history, and favorite modules.
pub struct OcrFavoriteApplication {
    history: Arc<dyn OcrFavoriteHistory>,
    favorites: Arc<dyn OcrFavoriteStore>,
    recognizer: Arc<dyn OcrFavoriteRecognizer>,
}

impl OcrFavoriteApplication {
    pub fn new(
        history: Arc<dyn OcrFavoriteHistory>,
        favorites: Arc<dyn OcrFavoriteStore>,
        recognizer: Arc<dyn OcrFavoriteRecognizer>,
    ) -> Self {
        Self {
            history,
            favorites,
            recognizer,
        }
    }

    pub async fn favorite(
        &self,
        source_history_id: Option<i64>,
        request: OcrRequest,
        result: OcrResult,
        provider_used: Option<String>,
    ) -> Result<i64> {
        let provider_used = provider_used
            .or_else(|| self.recognizer.active_provider_id())
            .unwrap_or_else(|| "manual".to_string());
        let image_data = if request.image_data.is_empty() {
            match source_history_id {
                Some(id) => self.history.read_ocr_source(id).await?,
                None => Vec::new(),
            }
        } else {
            request.image_data
        };

        self.favorites
            .add_ocr(
                source_history_id,
                image_data,
                request.language,
                provider_used,
                result,
            )
            .await
    }

    pub async fn rerun(&self, id: i64) -> Result<OcrResult> {
        let image = self.favorites.read_ocr_source(id).await?;
        self.recognizer.recognize(image).await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    struct AddedFavorite {
        source_history_id: Option<i64>,
        image_data: Vec<u8>,
        language: Option<String>,
        provider_used: String,
        result: OcrResult,
    }

    struct StubHistory {
        image: Result<Vec<u8>>,
    }

    #[async_trait]
    impl OcrFavoriteHistory for StubHistory {
        async fn read_ocr_source(&self, _id: i64) -> Result<Vec<u8>> {
            match &self.image {
                Ok(image) => Ok(image.clone()),
                Err(error) => Err(error.to_string().into()),
            }
        }
    }

    struct RecordingFavorites {
        added: Mutex<Vec<AddedFavorite>>,
        image: Vec<u8>,
    }

    #[async_trait]
    impl OcrFavoriteStore for RecordingFavorites {
        async fn add_ocr(
            &self,
            source_history_id: Option<i64>,
            image_data: Vec<u8>,
            language: Option<String>,
            provider_used: String,
            result: OcrResult,
        ) -> Result<i64> {
            self.added.lock().unwrap().push(AddedFavorite {
                source_history_id,
                image_data,
                language,
                provider_used,
                result,
            });
            Ok(42)
        }

        async fn read_ocr_source(&self, _id: i64) -> Result<Vec<u8>> {
            Ok(self.image.clone())
        }
    }

    struct RecordingRecognizer {
        active_provider_id: Option<String>,
        recognized_images: Mutex<Vec<Vec<u8>>>,
    }

    #[async_trait]
    impl OcrFavoriteRecognizer for RecordingRecognizer {
        fn active_provider_id(&self) -> Option<String> {
            self.active_provider_id.clone()
        }

        async fn recognize(&self, image: Vec<u8>) -> Result<OcrResult> {
            self.recognized_images.lock().unwrap().push(image);
            Ok(ocr_result("rerun"))
        }
    }

    fn ocr_result(text: &str) -> OcrResult {
        OcrResult {
            text: text.to_string(),
            confidence: Some(0.9),
        }
    }

    fn application(
        history_image: Result<Vec<u8>>,
        favorite_image: Vec<u8>,
        active_provider_id: Option<&str>,
    ) -> (
        OcrFavoriteApplication,
        Arc<RecordingFavorites>,
        Arc<RecordingRecognizer>,
    ) {
        let favorites = Arc::new(RecordingFavorites {
            added: Mutex::new(Vec::new()),
            image: favorite_image,
        });
        let recognizer = Arc::new(RecordingRecognizer {
            active_provider_id: active_provider_id.map(str::to_string),
            recognized_images: Mutex::new(Vec::new()),
        });
        let application = OcrFavoriteApplication::new(
            Arc::new(StubHistory {
                image: history_image,
            }),
            favorites.clone(),
            recognizer.clone(),
        );
        (application, favorites, recognizer)
    }

    #[tokio::test]
    async fn favorite_preserves_explicit_image_and_provider() {
        let (application, favorites, _) = application(Ok(vec![9]), Vec::new(), Some("active"));

        let id = application
            .favorite(
                Some(7),
                OcrRequest {
                    image_data: vec![1, 2],
                    language: Some("eng".to_string()),
                },
                ocr_result("text"),
                Some("explicit".to_string()),
            )
            .await
            .unwrap();

        assert_eq!(id, 42);
        let added = favorites.added.lock().unwrap();
        assert_eq!(added[0].image_data, vec![1, 2]);
        assert_eq!(added[0].provider_used, "explicit");
    }

    #[tokio::test]
    async fn favorite_restores_history_image_and_uses_active_provider() {
        let (application, favorites, _) = application(Ok(vec![3, 4]), Vec::new(), Some("active"));

        application
            .favorite(
                Some(7),
                OcrRequest {
                    image_data: Vec::new(),
                    language: None,
                },
                ocr_result("text"),
                None,
            )
            .await
            .unwrap();

        let added = favorites.added.lock().unwrap();
        assert_eq!(added[0].image_data, vec![3, 4]);
        assert_eq!(added[0].provider_used, "active");
    }

    #[tokio::test]
    async fn favorite_uses_manual_without_explicit_or_active_provider() {
        let (application, favorites, _) = application(Ok(Vec::new()), Vec::new(), None);

        application
            .favorite(
                None,
                OcrRequest {
                    image_data: Vec::new(),
                    language: None,
                },
                ocr_result("text"),
                None,
            )
            .await
            .unwrap();

        assert_eq!(favorites.added.lock().unwrap()[0].provider_used, "manual");
    }

    #[tokio::test]
    async fn history_read_failure_prevents_favorite_write() {
        let (application, favorites, _) =
            application(Err("missing image".into()), Vec::new(), None);

        let result = application
            .favorite(
                Some(7),
                OcrRequest {
                    image_data: Vec::new(),
                    language: None,
                },
                ocr_result("text"),
                None,
            )
            .await;

        assert_eq!(result.unwrap_err().to_string(), "missing image");
        assert!(favorites.added.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn rerun_reads_favorite_source_before_recognition() {
        let (application, _, recognizer) = application(Ok(Vec::new()), vec![5, 6], None);

        let result = application.rerun(12).await.unwrap();

        assert_eq!(result.text, "rerun");
        assert_eq!(recognizer.recognized_images.lock().unwrap()[0], vec![5, 6]);
    }
}

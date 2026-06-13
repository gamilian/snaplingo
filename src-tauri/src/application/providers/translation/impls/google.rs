use crate::application::providers::common::Provider;
use crate::application::providers::translation::TranslationProvider;
use crate::domain::translation::{TranslationRequest, TranslationResult};
use crate::infrastructure::http::HttpClient;
use crate::{AppError, Result};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// Google Translate provider implementation.
///
/// Uses the free Google Translate API endpoint that doesn't require API keys.
/// This is suitable for personal use but may have rate limits.
pub struct GoogleTranslateProvider {
    http_client: Arc<dyn HttpClient>,
}

impl GoogleTranslateProvider {
    /// Creates a new Google Translate provider with the given HTTP client.
    pub fn new(http_client: Arc<dyn HttpClient>) -> Self {
        Self { http_client }
    }
}

impl Provider for GoogleTranslateProvider {
    fn id(&self) -> &str {
        "google-translate"
    }

    fn name(&self) -> &str {
        "Google Translate"
    }

    fn is_configured(&self) -> bool {
        true // Google Translate is always ready (no API key needed)
    }

    fn requires_api_key(&self) -> bool {
        false
    }
}

#[async_trait]
impl TranslationProvider for GoogleTranslateProvider {
    async fn translate(&self, request: &TranslationRequest) -> Result<TranslationResult> {
        let url = format!(
            "https://translate.googleapis.com/translate_a/single?client=gtx&sl={}&tl={}&dt=t&q={}",
            request.source_lang,
            request.target_lang,
            urlencoding::encode(&request.text)
        );

        let response = self.http_client.get(&url, HashMap::new()).await
            .map_err(|e| AppError::Other(format!("HTTP request failed: {}", e)))?;

        if response.status != 200 {
            return Err(AppError::Other(format!(
                "Google Translate API returned status {}: {}",
                response.status,
                response.body
            )));
        }

        let json: Value = serde_json::from_str(&response.body)?;

        let translated_text = json[0][0][0]
            .as_str()
            .ok_or_else(|| AppError::Other("Invalid response format from Google Translate".to_string()))?
            .to_string();

        let detected_language = json[2].as_str().map(String::from);

        Ok(TranslationResult {
            translated_text,
            detected_language,
            confidence: None, // Google's free API doesn't provide confidence scores
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::http::HttpResponse;
    use std::collections::HashMap;

    struct MockHttpClient {
        response: HttpResponse,
    }

    #[async_trait]
    impl HttpClient for MockHttpClient {
        async fn post(
            &self,
            _url: &str,
            _headers: HashMap<String, String>,
            _body: String,
        ) -> anyhow::Result<HttpResponse> {
            Ok(self.response.clone())
        }

        async fn get(
            &self,
            _url: &str,
            _headers: HashMap<String, String>,
        ) -> anyhow::Result<HttpResponse> {
            Ok(self.response.clone())
        }
    }

    #[tokio::test]
    async fn test_provider_metadata() {
        let mock_client = Arc::new(MockHttpClient {
            response: HttpResponse {
                status: 200,
                body: String::new(),
                headers: HashMap::new(),
            },
        });

        let provider = GoogleTranslateProvider::new(mock_client);

        assert_eq!(provider.id(), "google-translate");
        assert_eq!(provider.name(), "Google Translate");
        assert!(provider.is_configured());
        assert!(!provider.requires_api_key());
    }

    #[tokio::test]
    async fn test_translate_success() {
        let mock_response = r#"[[["Hello","Bonjour",null,null,10]],null,"fr",null,null,null,null,[]]"#;

        let mock_client = Arc::new(MockHttpClient {
            response: HttpResponse {
                status: 200,
                body: mock_response.to_string(),
                headers: HashMap::new(),
            },
        });

        let provider = GoogleTranslateProvider::new(mock_client);

        let request = TranslationRequest {
            text: "Bonjour".to_string(),
            source_lang: "fr".to_string(),
            target_lang: "en".to_string(),
        };

        let result = provider.translate(&request).await.unwrap();

        assert_eq!(result.translated_text, "Hello");
        assert_eq!(result.detected_language, Some("fr".to_string()));
        assert_eq!(result.confidence, None);
    }

    #[tokio::test]
    async fn test_translate_http_error() {
        let mock_client = Arc::new(MockHttpClient {
            response: HttpResponse {
                status: 500,
                body: "Internal Server Error".to_string(),
                headers: HashMap::new(),
            },
        });

        let provider = GoogleTranslateProvider::new(mock_client);

        let request = TranslationRequest {
            text: "Hello".to_string(),
            source_lang: "en".to_string(),
            target_lang: "fr".to_string(),
        };

        let result = provider.translate(&request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_translate_invalid_json() {
        let mock_client = Arc::new(MockHttpClient {
            response: HttpResponse {
                status: 200,
                body: "invalid json".to_string(),
                headers: HashMap::new(),
            },
        });

        let provider = GoogleTranslateProvider::new(mock_client);

        let request = TranslationRequest {
            text: "Hello".to_string(),
            source_lang: "en".to_string(),
            target_lang: "fr".to_string(),
        };

        let result = provider.translate(&request).await;
        assert!(result.is_err());
    }
}

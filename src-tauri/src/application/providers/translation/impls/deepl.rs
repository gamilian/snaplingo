use crate::application::providers::common::Provider;
use crate::application::providers::translation::TranslationProvider;
use crate::domain::translation::{TranslationRequest, TranslationResult};
use crate::infrastructure::http::HttpClient;
use crate::{AppError, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// DeepL translation provider implementation.
///
/// Uses the DeepL API which requires an API key (paid service).
/// Supports both free and paid API tiers.
pub struct DeepLProvider {
    http_client: Arc<dyn HttpClient>,
    api_key: Option<String>,
}

impl DeepLProvider {
    /// Creates a new DeepL provider with the given HTTP client.
    pub fn new(http_client: Arc<dyn HttpClient>) -> Self {
        Self {
            http_client,
            api_key: None,
        }
    }

    /// Sets the API key for authentication.
    pub fn set_api_key(&mut self, api_key: String) {
        self.api_key = Some(api_key);
    }
}

impl Provider for DeepLProvider {
    fn id(&self) -> &str {
        "deepl"
    }

    fn name(&self) -> &str {
        "DeepL"
    }

    fn is_configured(&self) -> bool {
        self.api_key.is_some()
    }

    fn requires_api_key(&self) -> bool {
        true
    }
}

#[derive(Serialize)]
struct DeepLRequest {
    text: Vec<String>,
    source_lang: String,
    target_lang: String,
}

#[derive(Deserialize)]
struct DeepLResponse {
    translations: Vec<DeepLTranslation>,
}

#[derive(Deserialize)]
struct DeepLTranslation {
    detected_source_language: Option<String>,
    text: String,
}

#[async_trait]
impl TranslationProvider for DeepLProvider {
    async fn translate(&self, request: &TranslationRequest) -> Result<TranslationResult> {
        let api_key = self.api_key.as_ref()
            .ok_or_else(|| AppError::Other("DeepL API key not configured".to_string()))?;

        let url = "https://api-free.deepl.com/v2/translate";

        let deepl_request = DeepLRequest {
            text: vec![request.text.clone()],
            source_lang: request.source_lang.to_uppercase(),
            target_lang: request.target_lang.to_uppercase(),
        };

        let body = serde_json::to_string(&deepl_request)?;

        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), format!("DeepL-Auth-Key {}", api_key));
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        let response = self.http_client.post(&url, headers, body).await
            .map_err(|e| AppError::Other(format!("HTTP request failed: {}", e)))?;

        if response.status != 200 {
            return Err(AppError::Other(format!(
                "DeepL API returned status {}: {}",
                response.status,
                response.body
            )));
        }

        let deepl_response: DeepLResponse = serde_json::from_str(&response.body)?;

        let translation = deepl_response.translations.first()
            .ok_or_else(|| AppError::Other("Empty response from DeepL".to_string()))?;

        Ok(TranslationResult {
            translated_text: translation.text.clone(),
            detected_language: translation.detected_source_language.clone(),
            confidence: None, // DeepL doesn't provide confidence scores
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::http::HttpResponse;

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

        let provider = DeepLProvider::new(mock_client);

        assert_eq!(provider.id(), "deepl");
        assert_eq!(provider.name(), "DeepL");
        assert!(!provider.is_configured()); // Not configured without API key
        assert!(provider.requires_api_key());
    }

    #[tokio::test]
    async fn test_provider_configured_with_api_key() {
        let mock_client = Arc::new(MockHttpClient {
            response: HttpResponse {
                status: 200,
                body: String::new(),
                headers: HashMap::new(),
            },
        });

        let mut provider = DeepLProvider::new(mock_client);
        provider.set_api_key("test-api-key".to_string());

        assert!(provider.is_configured());
    }

    #[tokio::test]
    async fn test_translate_success() {
        let mock_response = r#"{"translations":[{"detected_source_language":"FR","text":"Hello"}]}"#;

        let mock_client = Arc::new(MockHttpClient {
            response: HttpResponse {
                status: 200,
                body: mock_response.to_string(),
                headers: HashMap::new(),
            },
        });

        let mut provider = DeepLProvider::new(mock_client);
        provider.set_api_key("test-api-key".to_string());

        let request = TranslationRequest {
            text: "Bonjour".to_string(),
            source_lang: "fr".to_string(),
            target_lang: "en".to_string(),
        };

        let result = provider.translate(&request).await.unwrap();

        assert_eq!(result.translated_text, "Hello");
        assert_eq!(result.detected_language, Some("FR".to_string()));
        assert_eq!(result.confidence, None);
    }

    #[tokio::test]
    async fn test_translate_without_api_key() {
        let mock_client = Arc::new(MockHttpClient {
            response: HttpResponse {
                status: 200,
                body: String::new(),
                headers: HashMap::new(),
            },
        });

        let provider = DeepLProvider::new(mock_client);

        let request = TranslationRequest {
            text: "Hello".to_string(),
            source_lang: "en".to_string(),
            target_lang: "fr".to_string(),
        };

        let result = provider.translate(&request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_translate_http_error() {
        let mock_client = Arc::new(MockHttpClient {
            response: HttpResponse {
                status: 403,
                body: "Forbidden".to_string(),
                headers: HashMap::new(),
            },
        });

        let mut provider = DeepLProvider::new(mock_client);
        provider.set_api_key("invalid-key".to_string());

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

        let mut provider = DeepLProvider::new(mock_client);
        provider.set_api_key("test-api-key".to_string());

        let request = TranslationRequest {
            text: "Hello".to_string(),
            source_lang: "en".to_string(),
            target_lang: "fr".to_string(),
        };

        let result = provider.translate(&request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_translate_empty_response() {
        let mock_response = r#"{"translations":[]}"#;

        let mock_client = Arc::new(MockHttpClient {
            response: HttpResponse {
                status: 200,
                body: mock_response.to_string(),
                headers: HashMap::new(),
            },
        });

        let mut provider = DeepLProvider::new(mock_client);
        provider.set_api_key("test-api-key".to_string());

        let request = TranslationRequest {
            text: "Hello".to_string(),
            source_lang: "en".to_string(),
            target_lang: "fr".to_string(),
        };

        let result = provider.translate(&request).await;
        assert!(result.is_err());
    }
}

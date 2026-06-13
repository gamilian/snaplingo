use crate::application::providers::common::Provider;
use crate::application::providers::ocr::OcrProvider;
use crate::domain::ocr::{OcrRequest, OcrResult};
use crate::infrastructure::http::HttpClient;
use crate::{AppError, Result};
use async_trait::async_trait;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;

/// Baidu OCR provider implementation.
///
/// Uses the Baidu OCR API which requires API Key and Secret Key.
/// Authentication is performed via OAuth 2.0 access token.
pub struct BaiduOcrProvider {
    http_client: Arc<dyn HttpClient>,
    api_key: Option<String>,
    secret_key: Option<String>,
}

impl BaiduOcrProvider {
    /// Creates a new Baidu OCR provider with the given HTTP client.
    pub fn new(http_client: Arc<dyn HttpClient>) -> Self {
        Self {
            http_client,
            api_key: None,
            secret_key: None,
        }
    }

    /// Configures the provider with API Key and Secret Key.
    pub fn configure(&mut self, api_key: String, secret_key: String) {
        self.api_key = Some(api_key);
        self.secret_key = Some(secret_key);
    }

    /// Gets an access token from Baidu OAuth API.
    async fn get_access_token(&self) -> Result<String> {
        let api_key = self.api_key.as_ref()
            .ok_or_else(|| AppError::Other("Baidu API Key not configured".to_string()))?;
        let secret_key = self.secret_key.as_ref()
            .ok_or_else(|| AppError::Other("Baidu Secret Key not configured".to_string()))?;

        let url = format!(
            "https://aip.baidubce.com/oauth/2.0/token?grant_type=client_credentials&client_id={}&client_secret={}",
            api_key,
            secret_key
        );

        let response = self.http_client.get(&url, HashMap::new()).await
            .map_err(|e| AppError::Other(format!("Failed to get access token: {}", e)))?;

        if response.status != 200 {
            return Err(AppError::Other(format!(
                "Baidu OAuth API returned status {}: {}",
                response.status,
                response.body
            )));
        }

        let token_response: BaiduTokenResponse = serde_json::from_str(&response.body)?;

        if let Some(error) = token_response.error {
            return Err(AppError::Other(format!(
                "Baidu OAuth error: {}",
                token_response.error_description.unwrap_or(error)
            )));
        }

        token_response.access_token
            .ok_or_else(|| AppError::Other("No access token in response".to_string()))
    }
}

impl Provider for BaiduOcrProvider {
    fn id(&self) -> &str {
        "baidu-ocr"
    }

    fn name(&self) -> &str {
        "Baidu OCR"
    }

    fn is_configured(&self) -> bool {
        self.api_key.is_some() && self.secret_key.is_some()
    }

    fn requires_api_key(&self) -> bool {
        true
    }
}

#[derive(Deserialize)]
struct BaiduTokenResponse {
    access_token: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

#[derive(Deserialize)]
struct BaiduOcrResponse {
    words_result: Option<Vec<BaiduOcrWord>>,
    error_code: Option<i32>,
    error_msg: Option<String>,
}

#[derive(Deserialize)]
struct BaiduOcrWord {
    words: String,
}

#[async_trait]
impl OcrProvider for BaiduOcrProvider {
    async fn recognize(&self, request: &OcrRequest) -> Result<OcrResult> {
        if !self.is_configured() {
            return Err(AppError::Other("Baidu OCR not configured with API Key and Secret Key".to_string()));
        }

        // Get access token
        let access_token = self.get_access_token().await?;

        // Encode image to base64
        let image_base64 = STANDARD.encode(&request.image_data);

        // Prepare form data for OCR API
        let url = format!(
            "https://aip.baidubce.com/rest/2.0/ocr/v1/general_basic?access_token={}",
            access_token
        );

        let mut form_data = HashMap::new();
        form_data.insert("image".to_string(), image_base64);

        // Convert HashMap to URL-encoded form string
        let body = form_data
            .iter()
            .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");

        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/x-www-form-urlencoded".to_string());

        let response = self.http_client.post(&url, headers, body).await
            .map_err(|e| AppError::Other(format!("HTTP request failed: {}", e)))?;

        if response.status != 200 {
            return Err(AppError::Other(format!(
                "Baidu OCR API returned status {}: {}",
                response.status,
                response.body
            )));
        }

        let ocr_response: BaiduOcrResponse = serde_json::from_str(&response.body)?;

        // Check for API errors
        if let Some(error_code) = ocr_response.error_code {
            let error_msg = ocr_response.error_msg.unwrap_or_else(|| "Unknown error".to_string());
            return Err(AppError::Other(format!(
                "Baidu OCR API error {}: {}",
                error_code,
                error_msg
            )));
        }

        let words_result = ocr_response.words_result
            .ok_or_else(|| AppError::Other("No OCR result from Baidu".to_string()))?;

        // Concatenate all recognized text lines
        let text = words_result
            .iter()
            .map(|word| word.words.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        Ok(OcrResult {
            text,
            confidence: None, // Baidu basic OCR doesn't provide confidence scores
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::http::HttpResponse;

    struct MockHttpClient {
        responses: Vec<HttpResponse>,
        call_count: std::sync::Mutex<usize>,
    }

    impl MockHttpClient {
        fn new(responses: Vec<HttpResponse>) -> Self {
            Self {
                responses,
                call_count: std::sync::Mutex::new(0),
            }
        }

        fn get_next_response(&self) -> HttpResponse {
            let mut count = self.call_count.lock().unwrap();
            let response = self.responses.get(*count).unwrap().clone();
            *count += 1;
            response
        }
    }

    #[async_trait]
    impl HttpClient for MockHttpClient {
        async fn post(
            &self,
            _url: &str,
            _headers: HashMap<String, String>,
            _body: String,
        ) -> anyhow::Result<HttpResponse> {
            Ok(self.get_next_response())
        }

        async fn get(
            &self,
            _url: &str,
            _headers: HashMap<String, String>,
        ) -> anyhow::Result<HttpResponse> {
            Ok(self.get_next_response())
        }
    }

    #[test]
    fn test_provider_metadata() {
        let mock_client = Arc::new(MockHttpClient::new(vec![]));
        let provider = BaiduOcrProvider::new(mock_client);

        assert_eq!(provider.id(), "baidu-ocr");
        assert_eq!(provider.name(), "Baidu OCR");
        assert!(!provider.is_configured());
        assert!(provider.requires_api_key());
    }

    #[test]
    fn test_provider_configured() {
        let mock_client = Arc::new(MockHttpClient::new(vec![]));
        let mut provider = BaiduOcrProvider::new(mock_client);

        provider.configure("test-api-key".to_string(), "test-secret".to_string());
        assert!(provider.is_configured());
    }

    #[tokio::test]
    async fn test_get_access_token_success() {
        let token_response = r#"{"access_token":"test-token-123","expires_in":2592000}"#;

        let mock_client = Arc::new(MockHttpClient::new(vec![
            HttpResponse {
                status: 200,
                body: token_response.to_string(),
                headers: HashMap::new(),
            },
        ]));

        let mut provider = BaiduOcrProvider::new(mock_client);
        provider.configure("test-api-key".to_string(), "test-secret".to_string());

        let token = provider.get_access_token().await.unwrap();
        assert_eq!(token, "test-token-123");
    }

    #[tokio::test]
    async fn test_get_access_token_error() {
        let error_response = r#"{"error":"invalid_client","error_description":"Client authentication failed"}"#;

        let mock_client = Arc::new(MockHttpClient::new(vec![
            HttpResponse {
                status: 200,
                body: error_response.to_string(),
                headers: HashMap::new(),
            },
        ]));

        let mut provider = BaiduOcrProvider::new(mock_client);
        provider.configure("bad-api-key".to_string(), "bad-secret".to_string());

        let result = provider.get_access_token().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_recognize_success() {
        let token_response = r#"{"access_token":"test-token-123","expires_in":2592000}"#;
        let ocr_response = r#"{"words_result":[{"words":"Hello"},{"words":"World"}],"words_result_num":2}"#;

        let mock_client = Arc::new(MockHttpClient::new(vec![
            HttpResponse {
                status: 200,
                body: token_response.to_string(),
                headers: HashMap::new(),
            },
            HttpResponse {
                status: 200,
                body: ocr_response.to_string(),
                headers: HashMap::new(),
            },
        ]));

        let mut provider = BaiduOcrProvider::new(mock_client);
        provider.configure("test-api-key".to_string(), "test-secret".to_string());

        let request = OcrRequest {
            image_data: vec![0xFF, 0xD8, 0xFF], // Fake JPEG header
            language: Some("en".to_string()),
        };

        let result = provider.recognize(&request).await.unwrap();
        assert_eq!(result.text, "Hello\nWorld");
        assert_eq!(result.confidence, None);
    }

    #[tokio::test]
    async fn test_recognize_without_credentials() {
        let mock_client = Arc::new(MockHttpClient::new(vec![]));
        let provider = BaiduOcrProvider::new(mock_client);

        let request = OcrRequest {
            image_data: vec![0xFF, 0xD8, 0xFF],
            language: Some("en".to_string()),
        };

        let result = provider.recognize(&request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_recognize_api_error() {
        let token_response = r#"{"access_token":"test-token-123","expires_in":2592000}"#;
        let error_response = r#"{"error_code":216015,"error_msg":"module closed"}"#;

        let mock_client = Arc::new(MockHttpClient::new(vec![
            HttpResponse {
                status: 200,
                body: token_response.to_string(),
                headers: HashMap::new(),
            },
            HttpResponse {
                status: 200,
                body: error_response.to_string(),
                headers: HashMap::new(),
            },
        ]));

        let mut provider = BaiduOcrProvider::new(mock_client);
        provider.configure("test-api-key".to_string(), "test-secret".to_string());

        let request = OcrRequest {
            image_data: vec![0xFF, 0xD8, 0xFF],
            language: Some("en".to_string()),
        };

        let result = provider.recognize(&request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_recognize_empty_result() {
        let token_response = r#"{"access_token":"test-token-123","expires_in":2592000}"#;
        let ocr_response = r#"{"words_result":[],"words_result_num":0}"#;

        let mock_client = Arc::new(MockHttpClient::new(vec![
            HttpResponse {
                status: 200,
                body: token_response.to_string(),
                headers: HashMap::new(),
            },
            HttpResponse {
                status: 200,
                body: ocr_response.to_string(),
                headers: HashMap::new(),
            },
        ]));

        let mut provider = BaiduOcrProvider::new(mock_client);
        provider.configure("test-api-key".to_string(), "test-secret".to_string());

        let request = OcrRequest {
            image_data: vec![0xFF, 0xD8, 0xFF],
            language: Some("en".to_string()),
        };

        let result = provider.recognize(&request).await.unwrap();
        assert_eq!(result.text, "");
    }

    #[tokio::test]
    async fn test_recognize_http_error() {
        let token_response = r#"{"access_token":"test-token-123","expires_in":2592000}"#;

        let mock_client = Arc::new(MockHttpClient::new(vec![
            HttpResponse {
                status: 200,
                body: token_response.to_string(),
                headers: HashMap::new(),
            },
            HttpResponse {
                status: 500,
                body: "Internal Server Error".to_string(),
                headers: HashMap::new(),
            },
        ]));

        let mut provider = BaiduOcrProvider::new(mock_client);
        provider.configure("test-api-key".to_string(), "test-secret".to_string());

        let request = OcrRequest {
            image_data: vec![0xFF, 0xD8, 0xFF],
            language: Some("en".to_string()),
        };

        let result = provider.recognize(&request).await;
        assert!(result.is_err());
    }
}

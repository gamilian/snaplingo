use crate::application::providers::common::{CredentialField, Provider};
use crate::application::providers::translation::TranslationProvider;
use crate::application::providers::HttpClient;
use crate::domain::translation::{TranslationRequest, TranslationResult};
use crate::{AppError, Result};
use async_trait::async_trait;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;

/// Baidu Translate provider implementation.
///
/// Uses the Baidu Translation API which requires APP ID and Secret Key.
/// Authentication is performed using MD5 signature generation.
pub struct BaiduTranslateProvider {
    http_client: Arc<dyn HttpClient>,
    app_id: Option<String>,
    secret_key: Option<String>,
}

impl BaiduTranslateProvider {
    /// Creates a new Baidu Translate provider with the given HTTP client.
    pub fn new(http_client: Arc<dyn HttpClient>) -> Self {
        Self {
            http_client,
            app_id: None,
            secret_key: None,
        }
    }

    /// Configures the provider with APP ID and Secret Key.
    pub fn configure(&mut self, app_id: String, secret_key: String) {
        self.app_id = Some(app_id);
        self.secret_key = Some(secret_key);
    }

    /// Configures the provider from a HashMap of credentials.
    /// Expected keys: "app_id", "secret_key"
    pub fn configure_from_map(
        &mut self,
        credentials: &HashMap<String, String>,
    ) -> crate::Result<()> {
        let app_id = credentials
            .get("app_id")
            .ok_or_else(|| crate::AppError::Other("Missing app_id".to_string()))?
            .clone();
        let secret_key = credentials
            .get("secret_key")
            .ok_or_else(|| crate::AppError::Other("Missing secret_key".to_string()))?
            .clone();

        self.configure(app_id, secret_key);
        Ok(())
    }

    /// Generates MD5 signature for Baidu API authentication.
    /// Signature formula: MD5(appid + query + salt + secret)
    fn generate_signature(&self, query: &str, salt: &str) -> Result<String> {
        let app_id = self
            .app_id
            .as_ref()
            .ok_or_else(|| AppError::Other("Baidu APP ID not configured".to_string()))?;
        let secret_key = self
            .secret_key
            .as_ref()
            .ok_or_else(|| AppError::Other("Baidu Secret Key not configured".to_string()))?;

        let sign_str = format!("{}{}{}{}", app_id, query, salt, secret_key);
        let digest = md5::compute(sign_str.as_bytes());
        Ok(format!("{:x}", digest))
    }
}

impl Provider for BaiduTranslateProvider {
    fn id(&self) -> &str {
        "baidu-translate"
    }

    fn name(&self) -> &str {
        "Baidu Translate"
    }

    fn is_configured(&self) -> bool {
        self.app_id.is_some() && self.secret_key.is_some()
    }

    fn requires_api_key(&self) -> bool {
        true
    }

    fn credential_fields(&self) -> Vec<CredentialField> {
        vec![
            CredentialField::new("app_id", "App ID", false),
            CredentialField::new("secret_key", "Secret Key", true),
        ]
    }

    fn reconfigure_credentials(
        &mut self,
        credentials: &HashMap<String, String>,
    ) -> crate::Result<()> {
        self.configure_from_map(credentials)
    }
}

#[derive(Deserialize)]
struct BaiduResponse {
    trans_result: Option<Vec<BaiduTranslation>>,
    error_code: Option<String>,
    error_msg: Option<String>,
}

#[derive(Deserialize)]
struct BaiduTranslation {
    dst: String,
}

#[async_trait]
impl TranslationProvider for BaiduTranslateProvider {
    async fn translate(&self, request: &TranslationRequest) -> Result<TranslationResult> {
        if !self.is_configured() {
            return Err(AppError::Other(
                "Baidu Translate not configured with APP ID and Secret Key".to_string(),
            ));
        }

        // Generate salt (random number)
        let salt = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
            .to_string();

        let signature = self.generate_signature(&request.text, &salt)?;

        let app_id = self.app_id.as_ref().unwrap();

        let url = format!(
            "https://fanyi-api.baidu.com/api/trans/vip/translate?q={}&from={}&to={}&appid={}&salt={}&sign={}",
            urlencoding::encode(&request.text),
            request.source_lang,
            request.target_lang,
            app_id,
            salt,
            signature
        );

        let response = self
            .http_client
            .get(&url, HashMap::new())
            .await
            .map_err(|e| AppError::Other(format!("HTTP request failed: {}", e)))?;

        if response.status != 200 {
            return Err(AppError::Other(format!(
                "Baidu Translate API returned status {}: {}",
                response.status, response.body
            )));
        }

        let baidu_response: BaiduResponse = serde_json::from_str(&response.body)?;

        // Check for API errors
        if let Some(error_code) = baidu_response.error_code {
            let error_msg = baidu_response
                .error_msg
                .unwrap_or_else(|| "Unknown error".to_string());
            return Err(AppError::Other(format!(
                "Baidu Translate API error {}: {}",
                error_code, error_msg
            )));
        }

        let translations = baidu_response
            .trans_result
            .ok_or_else(|| AppError::Other("No translation result from Baidu".to_string()))?;

        let translation = translations
            .first()
            .ok_or_else(|| AppError::Other("Empty translation result from Baidu".to_string()))?;

        Ok(TranslationResult {
            provider_id: self.id().to_string(),
            translated_text: translation.dst.clone(),
            detected_language: None, // Baidu doesn't provide detected language in this endpoint
            confidence: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::providers::HttpResponse;

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

        let provider = BaiduTranslateProvider::new(mock_client);

        assert_eq!(provider.id(), "baidu-translate");
        assert_eq!(provider.name(), "Baidu Translate");
        assert!(!provider.is_configured()); // Not configured without credentials
        assert!(provider.requires_api_key());
    }

    #[tokio::test]
    async fn test_provider_configured() {
        let mock_client = Arc::new(MockHttpClient {
            response: HttpResponse {
                status: 200,
                body: String::new(),
                headers: HashMap::new(),
            },
        });

        let mut provider = BaiduTranslateProvider::new(mock_client);
        provider.configure("test-app-id".to_string(), "test-secret".to_string());

        assert!(provider.is_configured());
    }

    #[tokio::test]
    async fn test_generate_signature() {
        let mock_client = Arc::new(MockHttpClient {
            response: HttpResponse {
                status: 200,
                body: String::new(),
                headers: HashMap::new(),
            },
        });

        let mut provider = BaiduTranslateProvider::new(mock_client);
        provider.configure("test-app-id".to_string(), "test-secret".to_string());

        let signature = provider.generate_signature("hello", "12345").unwrap();

        // Expected: MD5("test-app-idhello12345test-secret")
        assert!(!signature.is_empty());
        assert_eq!(signature.len(), 32); // MD5 hex string is 32 characters
    }

    #[tokio::test]
    async fn test_translate_success() {
        let mock_response = r#"{"trans_result":[{"src":"hello","dst":"你好"}]}"#;

        let mock_client = Arc::new(MockHttpClient {
            response: HttpResponse {
                status: 200,
                body: mock_response.to_string(),
                headers: HashMap::new(),
            },
        });

        let mut provider = BaiduTranslateProvider::new(mock_client);
        provider.configure("test-app-id".to_string(), "test-secret".to_string());

        let request = TranslationRequest {
            text: "hello".to_string(),
            source_lang: "en".to_string(),
            target_lang: "zh".to_string(),
        };

        let result = provider.translate(&request).await.unwrap();

        assert_eq!(result.translated_text, "你好");
        assert_eq!(result.detected_language, None);
        assert_eq!(result.confidence, None);
    }

    #[tokio::test]
    async fn test_translate_without_credentials() {
        let mock_client = Arc::new(MockHttpClient {
            response: HttpResponse {
                status: 200,
                body: String::new(),
                headers: HashMap::new(),
            },
        });

        let provider = BaiduTranslateProvider::new(mock_client);

        let request = TranslationRequest {
            text: "hello".to_string(),
            source_lang: "en".to_string(),
            target_lang: "zh".to_string(),
        };

        let result = provider.translate(&request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_translate_api_error() {
        let mock_response = r#"{"error_code":"52003","error_msg":"UNAUTHORIZED USER"}"#;

        let mock_client = Arc::new(MockHttpClient {
            response: HttpResponse {
                status: 200,
                body: mock_response.to_string(),
                headers: HashMap::new(),
            },
        });

        let mut provider = BaiduTranslateProvider::new(mock_client);
        provider.configure("test-app-id".to_string(), "test-secret".to_string());

        let request = TranslationRequest {
            text: "hello".to_string(),
            source_lang: "en".to_string(),
            target_lang: "zh".to_string(),
        };

        let result = provider.translate(&request).await;
        assert!(result.is_err());
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

        let mut provider = BaiduTranslateProvider::new(mock_client);
        provider.configure("test-app-id".to_string(), "test-secret".to_string());

        let request = TranslationRequest {
            text: "hello".to_string(),
            source_lang: "en".to_string(),
            target_lang: "zh".to_string(),
        };

        let result = provider.translate(&request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_translate_empty_result() {
        let mock_response = r#"{"trans_result":[]}"#;

        let mock_client = Arc::new(MockHttpClient {
            response: HttpResponse {
                status: 200,
                body: mock_response.to_string(),
                headers: HashMap::new(),
            },
        });

        let mut provider = BaiduTranslateProvider::new(mock_client);
        provider.configure("test-app-id".to_string(), "test-secret".to_string());

        let request = TranslationRequest {
            text: "hello".to_string(),
            source_lang: "en".to_string(),
            target_lang: "zh".to_string(),
        };

        let result = provider.translate(&request).await;
        assert!(result.is_err());
    }
}

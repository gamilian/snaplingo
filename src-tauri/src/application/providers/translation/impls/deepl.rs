use crate::application::providers::common::{CredentialField, Provider};
use crate::application::providers::translation::TranslationProvider;
use crate::domain::translation::{TranslationRequest, TranslationResult};
use crate::infrastructure::http::HttpClient;
use crate::{AppError, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// DeepLX translation provider implementation.
///
/// Uses a configured DeepLX-compatible endpoint.
pub struct DeepLProvider {
    http_client: Arc<dyn HttpClient>,
    mode: DeepLMode,
    endpoint: Option<String>,
    api_key: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DeepLMode {
    DeepLX,
    DeepL,
}

impl DeepLProvider {
    /// Creates a new DeepLX provider with the given HTTP client.
    pub fn new(http_client: Arc<dyn HttpClient>) -> Self {
        Self {
            http_client,
            mode: DeepLMode::DeepLX,
            endpoint: None,
            api_key: None,
        }
    }

    /// Sets the DeepLX endpoint.
    pub fn set_endpoint(&mut self, endpoint: String) {
        self.mode = DeepLMode::DeepLX;
        self.endpoint = Some(endpoint);
    }
}

impl Provider for DeepLProvider {
    fn id(&self) -> &str {
        "deeplx"
    }

    fn name(&self) -> &str {
        "DeepLX"
    }

    fn is_configured(&self) -> bool {
        match self.mode {
            DeepLMode::DeepLX => self.endpoint.is_some(),
            DeepLMode::DeepL => self.api_key.is_some(),
        }
    }

    fn requires_api_key(&self) -> bool {
        true
    }

    fn credential_fields(&self) -> Vec<CredentialField> {
        vec![
            CredentialField::new("mode", "模式", false),
            CredentialField::new("endpoint", "DeepLX API 地址", false),
            CredentialField::new("api_key", "DeepL API Key", true),
        ]
    }

    fn validate_credentials(&self, credentials: &HashMap<String, String>) -> crate::Result<()> {
        validate_deepl_credentials(credentials)
    }

    fn reconfigure_credentials(
        &mut self,
        credentials: &std::collections::HashMap<String, String>,
    ) -> crate::Result<()> {
        match parse_deepl_credentials(credentials)? {
            DeepLCredentials::DeepL { api_key } => {
                self.mode = DeepLMode::DeepL;
                self.api_key = Some(api_key);
            }
            DeepLCredentials::DeepLX { endpoint } => {
                self.mode = DeepLMode::DeepLX;
                self.endpoint = Some(endpoint);
            }
        }

        Ok(())
    }
}

enum DeepLCredentials {
    DeepL { api_key: String },
    DeepLX { endpoint: String },
}

fn validate_deepl_credentials(credentials: &HashMap<String, String>) -> crate::Result<()> {
    parse_deepl_credentials(credentials).map(|_| ())
}

fn parse_deepl_credentials(
    credentials: &HashMap<String, String>,
) -> crate::Result<DeepLCredentials> {
    let mode = credentials
        .get("mode")
        .map(String::as_str)
        .unwrap_or("deeplx");

    let credentials = match mode {
        "deepl" => {
            let Some(api_key) = credentials.get("api_key") else {
                return Err(crate::AppError::Other(
                    "DeepL mode requires api_key".to_string(),
                ));
            };
            if api_key.trim().is_empty() {
                return Err(crate::AppError::Other(
                    "DeepL api_key cannot be blank".to_string(),
                ));
            }
            DeepLCredentials::DeepL {
                api_key: api_key.trim().to_string(),
            }
        }
        "deeplx" => {
            let Some(endpoint) = credentials.get("endpoint") else {
                return Err(crate::AppError::Other(
                    "DeepLX mode requires endpoint".to_string(),
                ));
            };
            if endpoint.trim().is_empty() {
                return Err(crate::AppError::Other(
                    "DeepLX endpoint cannot be blank".to_string(),
                ));
            }
            DeepLCredentials::DeepLX {
                endpoint: endpoint.trim().to_string(),
            }
        }
        other => {
            return Err(crate::AppError::Other(format!(
                "Invalid DeepLX mode: {}",
                other
            )));
        }
    };

    Ok(credentials)
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

#[derive(Serialize)]
struct DeepLXRequest {
    text: String,
    source_lang: String,
    target_lang: String,
}

#[derive(Deserialize)]
struct DeepLXResponse {
    code: Option<i32>,
    data: String,
}

fn deeplx_translate_url(endpoint: &str) -> String {
    let endpoint = endpoint.trim().trim_end_matches('/');
    if endpoint.ends_with("/translate") {
        endpoint.to_string()
    } else {
        format!("{}/translate", endpoint)
    }
}

fn deepl_language_code(lang: &str) -> String {
    if lang.eq_ignore_ascii_case("zh") {
        return "ZH".to_string();
    }

    match lang.to_ascii_lowercase().as_str() {
        "zh-cn" | "zh-tw" => "ZH".to_string(),
        _ => lang.to_uppercase(),
    }
}

#[async_trait]
impl TranslationProvider for DeepLProvider {
    async fn translate(&self, request: &TranslationRequest) -> Result<TranslationResult> {
        if self.mode == DeepLMode::DeepL {
            return self.translate_with_deepl(request).await;
        }

        let endpoint = self
            .endpoint
            .as_ref()
            .ok_or_else(|| AppError::Other("DeepLX endpoint not configured".to_string()))?;

        let url = deeplx_translate_url(endpoint);

        let deeplx_request = DeepLXRequest {
            text: request.text.clone(),
            source_lang: deepl_language_code(&request.source_lang),
            target_lang: deepl_language_code(&request.target_lang),
        };

        let body = serde_json::to_string(&deeplx_request)?;

        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        let response = self
            .http_client
            .post(&url, headers, body)
            .await
            .map_err(|e| AppError::Other(format!("HTTP request failed: {}", e)))?;

        if response.status != 200 {
            return Err(AppError::Other(format!(
                "DeepLX API returned status {}: {}",
                response.status, response.body
            )));
        }

        let deeplx_response: DeepLXResponse = serde_json::from_str(&response.body)?;
        if let Some(code) = deeplx_response.code {
            if code != 200 {
                return Err(AppError::Other(format!(
                    "DeepLX API returned code {}",
                    code
                )));
            }
        }

        Ok(TranslationResult {
            provider_id: self.id().to_string(),
            translated_text: deeplx_response.data,
            detected_language: None,
            confidence: None,
        })
    }
}

impl DeepLProvider {
    async fn translate_with_deepl(
        &self,
        request: &TranslationRequest,
    ) -> Result<TranslationResult> {
        let api_key = self
            .api_key
            .as_ref()
            .ok_or_else(|| AppError::Other("DeepL API key not configured".to_string()))?;

        let deepl_request = DeepLRequest {
            text: vec![request.text.clone()],
            source_lang: deepl_language_code(&request.source_lang),
            target_lang: deepl_language_code(&request.target_lang),
        };

        let body = serde_json::to_string(&deepl_request)?;
        let mut headers = HashMap::new();
        headers.insert(
            "Authorization".to_string(),
            format!("DeepL-Auth-Key {}", api_key),
        );
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        let response = self
            .http_client
            .post("https://api-free.deepl.com/v2/translate", headers, body)
            .await
            .map_err(|e| AppError::Other(format!("HTTP request failed: {}", e)))?;

        if response.status != 200 {
            return Err(AppError::Other(format!(
                "DeepL API returned status {}: {}",
                response.status, response.body
            )));
        }

        let deepl_response: DeepLResponse = serde_json::from_str(&response.body)?;
        let translation = deepl_response
            .translations
            .first()
            .ok_or_else(|| AppError::Other("Empty response from DeepL".to_string()))?;

        Ok(TranslationResult {
            provider_id: self.id().to_string(),
            translated_text: translation.text.clone(),
            detected_language: translation.detected_source_language.clone(),
            confidence: None,
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

    struct RecordingHttpClient {
        response: HttpResponse,
        posts: Arc<std::sync::Mutex<Vec<(String, HashMap<String, String>, String)>>>,
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

    #[async_trait]
    impl HttpClient for RecordingHttpClient {
        async fn post(
            &self,
            url: &str,
            headers: HashMap<String, String>,
            body: String,
        ) -> anyhow::Result<HttpResponse> {
            self.posts
                .lock()
                .unwrap()
                .push((url.to_string(), headers, body));
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

        assert_eq!(provider.id(), "deeplx");
        assert_eq!(provider.name(), "DeepLX");
        assert!(!provider.is_configured()); // Not configured without endpoint
        assert!(provider.requires_api_key());
        let field_names: Vec<_> = provider
            .credential_fields()
            .into_iter()
            .map(|field| field.name)
            .collect();
        assert_eq!(
            field_names,
            vec![
                "mode".to_string(),
                "endpoint".to_string(),
                "api_key".to_string()
            ]
        );
    }

    #[tokio::test]
    async fn test_provider_configured_with_endpoint() {
        let mock_client = Arc::new(MockHttpClient {
            response: HttpResponse {
                status: 200,
                body: String::new(),
                headers: HashMap::new(),
            },
        });

        let mut provider = DeepLProvider::new(mock_client);
        provider.set_endpoint("https://deeplx.example.test".to_string());

        assert!(provider.is_configured());
    }

    #[test]
    fn validate_credentials_accepts_deepl_mode_api_key_without_endpoint() {
        let mock_client = Arc::new(MockHttpClient {
            response: HttpResponse {
                status: 200,
                body: String::new(),
                headers: HashMap::new(),
            },
        });
        let provider = DeepLProvider::new(mock_client);

        provider
            .validate_credentials(&HashMap::from([
                ("mode".to_string(), "deepl".to_string()),
                ("api_key".to_string(), "test-api-key".to_string()),
            ]))
            .unwrap();
    }

    #[test]
    fn validate_credentials_accepts_deeplx_mode_endpoint_without_api_key() {
        let mock_client = Arc::new(MockHttpClient {
            response: HttpResponse {
                status: 200,
                body: String::new(),
                headers: HashMap::new(),
            },
        });
        let provider = DeepLProvider::new(mock_client);

        provider
            .validate_credentials(&HashMap::from([
                ("mode".to_string(), "deeplx".to_string()),
                (
                    "endpoint".to_string(),
                    "https://deeplx.example.test".to_string(),
                ),
            ]))
            .unwrap();
    }

    #[test]
    fn validate_credentials_rejects_missing_required_deepl_api_key() {
        let mock_client = Arc::new(MockHttpClient {
            response: HttpResponse {
                status: 200,
                body: String::new(),
                headers: HashMap::new(),
            },
        });
        let provider = DeepLProvider::new(mock_client);

        let error = provider
            .validate_credentials(&HashMap::from([("mode".to_string(), "deepl".to_string())]))
            .unwrap_err();

        assert_eq!(error.to_string(), "DeepL mode requires api_key");
    }

    #[test]
    fn validate_credentials_rejects_blank_required_deeplx_endpoint() {
        let mock_client = Arc::new(MockHttpClient {
            response: HttpResponse {
                status: 200,
                body: String::new(),
                headers: HashMap::new(),
            },
        });
        let provider = DeepLProvider::new(mock_client);

        let error = provider
            .validate_credentials(&HashMap::from([
                ("mode".to_string(), "deeplx".to_string()),
                ("endpoint".to_string(), "  ".to_string()),
            ]))
            .unwrap_err();

        assert_eq!(error.to_string(), "DeepLX endpoint cannot be blank");
    }

    #[test]
    fn validate_credentials_rejects_invalid_mode() {
        let mock_client = Arc::new(MockHttpClient {
            response: HttpResponse {
                status: 200,
                body: String::new(),
                headers: HashMap::new(),
            },
        });
        let provider = DeepLProvider::new(mock_client);

        let error = provider
            .validate_credentials(&HashMap::from([("mode".to_string(), "hybrid".to_string())]))
            .unwrap_err();

        assert_eq!(error.to_string(), "Invalid DeepLX mode: hybrid");
    }

    #[tokio::test]
    async fn test_translate_success() {
        let mock_response = r#"{"code":200,"data":"Hello","alternatives":[]}"#;

        let mock_client = Arc::new(MockHttpClient {
            response: HttpResponse {
                status: 200,
                body: mock_response.to_string(),
                headers: HashMap::new(),
            },
        });

        let mut provider = DeepLProvider::new(mock_client);
        provider.set_endpoint("https://deeplx.example.test".to_string());

        let request = TranslationRequest {
            text: "Bonjour".to_string(),
            source_lang: "fr".to_string(),
            target_lang: "en".to_string(),
        };

        let result = provider.translate(&request).await.unwrap();

        assert_eq!(result.translated_text, "Hello");
        assert_eq!(result.detected_language, None);
        assert_eq!(result.confidence, None);
    }

    #[tokio::test]
    async fn test_deeplx_normalizes_chinese_locale_codes() {
        let posts = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mock_client = Arc::new(RecordingHttpClient {
            response: HttpResponse {
                status: 200,
                body: r#"{"code":200,"data":"你好","alternatives":[]}"#.to_string(),
                headers: HashMap::new(),
            },
            posts: posts.clone(),
        });

        let mut provider = DeepLProvider::new(mock_client);
        provider.set_endpoint("https://deeplx.example.test".to_string());

        provider
            .translate(&TranslationRequest {
                text: "你好".to_string(),
                source_lang: "zh-CN".to_string(),
                target_lang: "zh-CN".to_string(),
            })
            .await
            .unwrap();

        let posts = posts.lock().unwrap();
        let body: serde_json::Value = serde_json::from_str(&posts[0].2).unwrap();
        assert_eq!(body["source_lang"], "ZH");
        assert_eq!(body["target_lang"], "ZH");
    }

    #[tokio::test]
    async fn test_translate_with_standard_deepl_mode() {
        let posts = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mock_client = Arc::new(RecordingHttpClient {
            response: HttpResponse {
                status: 200,
                body: r#"{"translations":[{"detected_source_language":"FR","text":"Hello"}]}"#
                    .to_string(),
                headers: HashMap::new(),
            },
            posts: posts.clone(),
        });

        let mut provider = DeepLProvider::new(mock_client);
        provider
            .reconfigure_credentials(&HashMap::from([
                ("mode".to_string(), "deepl".to_string()),
                ("api_key".to_string(), "test-api-key".to_string()),
            ]))
            .unwrap();

        let result = provider
            .translate(&TranslationRequest {
                text: "Bonjour".to_string(),
                source_lang: "fr".to_string(),
                target_lang: "en".to_string(),
            })
            .await
            .unwrap();

        let posts = posts.lock().unwrap();
        assert_eq!(posts[0].0, "https://api-free.deepl.com/v2/translate");
        assert_eq!(
            posts[0].1.get("Authorization").map(String::as_str),
            Some("DeepL-Auth-Key test-api-key")
        );
        assert_eq!(result.translated_text, "Hello");
        assert_eq!(result.detected_language, Some("FR".to_string()));
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
        provider.set_endpoint("https://deeplx.example.test".to_string());

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
        provider.set_endpoint("https://deeplx.example.test".to_string());

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
        let mock_response = r#"{"code":500,"data":""}"#;

        let mock_client = Arc::new(MockHttpClient {
            response: HttpResponse {
                status: 200,
                body: mock_response.to_string(),
                headers: HashMap::new(),
            },
        });

        let mut provider = DeepLProvider::new(mock_client);
        provider.set_endpoint("https://deeplx.example.test".to_string());

        let request = TranslationRequest {
            text: "Hello".to_string(),
            source_lang: "en".to_string(),
            target_lang: "fr".to_string(),
        };

        let result = provider.translate(&request).await;
        assert!(result.is_err());
    }
}

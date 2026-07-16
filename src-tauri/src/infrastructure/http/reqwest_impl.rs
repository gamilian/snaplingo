use async_trait::async_trait;
use reqwest;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::application::providers::{HttpClient, HttpResponse};
use crate::application::settings::SettingsConfiguration;

#[derive(Clone, Debug, PartialEq, Eq)]
struct NetworkConfiguration {
    proxy_mode: String,
    proxy_url: String,
    timeout_ms: u32,
    retry_count: u8,
}

struct CachedClient {
    configuration: NetworkConfiguration,
    client: reqwest::Client,
}

/// HTTP client implementation using Reqwest
pub struct ReqwestHttpClient {
    default_client: reqwest::Client,
    settings: Option<Arc<SettingsConfiguration>>,
    cached_client: Mutex<Option<CachedClient>>,
}

impl ReqwestHttpClient {
    /// Create a new ReqwestHttpClient
    pub fn new() -> Self {
        Self {
            default_client: reqwest::Client::new(),
            settings: None,
            cached_client: Mutex::new(None),
        }
    }

    pub fn with_client(client: reqwest::Client) -> Self {
        Self {
            default_client: client,
            settings: None,
            cached_client: Mutex::new(None),
        }
    }

    pub fn with_settings(settings: Arc<SettingsConfiguration>) -> Self {
        Self {
            default_client: reqwest::Client::new(),
            settings: Some(settings),
            cached_client: Mutex::new(None),
        }
    }

    fn current_configuration(&self) -> anyhow::Result<Option<NetworkConfiguration>> {
        let Some(settings) = &self.settings else {
            return Ok(None);
        };
        let general = settings.snapshot()?.general;
        Ok(Some(NetworkConfiguration {
            proxy_mode: general.proxy_mode,
            proxy_url: general.proxy_url,
            timeout_ms: general.request_timeout_ms,
            retry_count: general.retry_count,
        }))
    }

    fn client_for_request(&self) -> anyhow::Result<(reqwest::Client, u8)> {
        let Some(configuration) = self.current_configuration()? else {
            return Ok((self.default_client.clone(), 0));
        };
        let mut cached = self.cached_client.lock().unwrap();
        if let Some(current) = cached.as_ref() {
            if current.configuration == configuration {
                return Ok((current.client.clone(), configuration.retry_count));
            }
        }

        let mut builder = reqwest::Client::builder()
            .timeout(Duration::from_millis(configuration.timeout_ms.into()));
        match configuration.proxy_mode.as_str() {
            "none" => builder = builder.no_proxy(),
            "manual" if !configuration.proxy_url.is_empty() => {
                builder = builder.proxy(reqwest::Proxy::all(&configuration.proxy_url)?);
            }
            "manual" => builder = builder.no_proxy(),
            _ => {}
        }
        let client = builder.build()?;
        *cached = Some(CachedClient {
            configuration: configuration.clone(),
            client: client.clone(),
        });
        Ok((client, configuration.retry_count))
    }

    async fn send_with_retries(
        &self,
        make_request: impl Fn(&reqwest::Client) -> reqwest::RequestBuilder,
    ) -> anyhow::Result<reqwest::Response> {
        let (client, retries) = self.client_for_request()?;
        for attempt in 0..=retries {
            match make_request(&client).send().await {
                Ok(response) if !response.status().is_server_error() || attempt == retries => {
                    return Ok(response);
                }
                Ok(_) | Err(_) if attempt < retries => {
                    tokio::time::sleep(Duration::from_millis(150 * u64::from(attempt + 1))).await;
                }
                Err(error) => return Err(error.into()),
                Ok(response) => return Ok(response),
            }
        }
        unreachable!("retry loop always returns")
    }
}

#[async_trait]
impl HttpClient for ReqwestHttpClient {
    async fn post(
        &self,
        url: &str,
        headers: HashMap<String, String>,
        body: String,
    ) -> anyhow::Result<HttpResponse> {
        let response = self
            .send_with_retries(|client| {
                let mut request = client.post(url).body(body.clone());
                for (key, value) in &headers {
                    request = request.header(key, value);
                }
                request
            })
            .await?;

        // Extract status
        let status = response.status().as_u16();

        // Extract headers
        let mut response_headers = HashMap::new();
        for (key, value) in response.headers() {
            if let Ok(value_str) = value.to_str() {
                response_headers.insert(key.to_string(), value_str.to_string());
            }
        }

        // Extract body
        let body = response.text().await?;

        Ok(HttpResponse {
            status,
            body,
            headers: response_headers,
        })
    }

    async fn get(
        &self,
        url: &str,
        headers: HashMap<String, String>,
    ) -> anyhow::Result<HttpResponse> {
        let response = self
            .send_with_retries(|client| {
                let mut request = client.get(url);
                for (key, value) in &headers {
                    request = request.header(key, value);
                }
                request
            })
            .await?;

        // Extract status
        let status = response.status().as_u16();

        // Extract headers
        let mut response_headers = HashMap::new();
        for (key, value) in response.headers() {
            if let Ok(value_str) = value.to_str() {
                response_headers.insert(key.to_string(), value_str.to_string());
            }
        }

        // Extract body
        let body = response.text().await?;

        Ok(HttpResponse {
            status,
            body,
            headers: response_headers,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::ReqwestHttpClient;
    use crate::application::settings::SettingsConfiguration;
    use crate::domain::GeneralSettings;
    use crate::infrastructure::storage::SqliteConfigStore;

    #[test]
    fn reads_proxy_timeout_and_retry_settings_for_each_request() {
        let configuration = Arc::new(SettingsConfiguration::new(Arc::new(
            SqliteConfigStore::new_in_memory(),
        )));
        configuration
            .update_general(GeneralSettings {
                proxy_mode: "manual".to_string(),
                proxy_url: "http://127.0.0.1:7890".to_string(),
                request_timeout_ms: 12_000,
                retry_count: 3,
                ..GeneralSettings::default()
            })
            .unwrap();
        let client = ReqwestHttpClient::with_settings(configuration);

        let settings = client.current_configuration().unwrap().unwrap();
        assert_eq!(settings.proxy_mode, "manual");
        assert_eq!(settings.proxy_url, "http://127.0.0.1:7890");
        assert_eq!(settings.timeout_ms, 12_000);
        assert_eq!(settings.retry_count, 3);
    }

    #[test]
    fn builds_a_client_for_a_manual_socks5_proxy() {
        let configuration = Arc::new(SettingsConfiguration::new(Arc::new(
            SqliteConfigStore::new_in_memory(),
        )));
        configuration
            .update_general(GeneralSettings {
                proxy_mode: "manual".to_string(),
                proxy_url: "socks5://127.0.0.1:1080".to_string(),
                ..GeneralSettings::default()
            })
            .unwrap();
        let client = ReqwestHttpClient::with_settings(configuration);

        assert!(client.client_for_request().is_ok());
    }
}

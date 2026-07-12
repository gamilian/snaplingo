use async_trait::async_trait;
use reqwest;
use std::collections::HashMap;

use crate::application::providers::{HttpClient, HttpResponse};

/// HTTP client implementation using Reqwest
pub struct ReqwestHttpClient {
    client: reqwest::Client,
}

impl ReqwestHttpClient {
    /// Create a new ReqwestHttpClient
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    pub fn with_client(client: reqwest::Client) -> Self {
        Self { client }
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
        let mut request = self.client.post(url);

        // Add headers
        for (key, value) in headers {
            request = request.header(key, value);
        }

        // Send request with body
        let response = request.body(body).send().await?;

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
        let mut request = self.client.get(url);

        // Add headers
        for (key, value) in headers {
            request = request.header(key, value);
        }

        // Send request
        let response = request.send().await?;

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

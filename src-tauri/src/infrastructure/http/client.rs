use async_trait::async_trait;
use std::collections::HashMap;

/// HTTP response structure
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
    pub headers: HashMap<String, String>,
}

/// HTTP client trait for making async requests
#[async_trait]
pub trait HttpClient: Send + Sync {
    /// Perform an async POST request
    async fn post(
        &self,
        url: &str,
        headers: HashMap<String, String>,
        body: String,
    ) -> anyhow::Result<HttpResponse>;

    /// Perform an async GET request
    async fn get(
        &self,
        url: &str,
        headers: HashMap<String, String>,
    ) -> anyhow::Result<HttpResponse>;
}

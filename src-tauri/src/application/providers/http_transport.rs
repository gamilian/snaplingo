use async_trait::async_trait;
use std::collections::HashMap;

/// HTTP response structure used by provider integrations.
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
    pub headers: HashMap<String, String>,
}

/// HTTP transport required by provider integrations.
#[async_trait]
pub trait HttpClient: Send + Sync {
    async fn post(
        &self,
        url: &str,
        headers: HashMap<String, String>,
        body: String,
    ) -> anyhow::Result<HttpResponse>;

    async fn get(
        &self,
        url: &str,
        headers: HashMap<String, String>,
    ) -> anyhow::Result<HttpResponse>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    struct RecordingHttpClient {
        requests: Arc<Mutex<Vec<String>>>,
    }

    #[async_trait]
    impl HttpClient for RecordingHttpClient {
        async fn post(
            &self,
            url: &str,
            _headers: HashMap<String, String>,
            body: String,
        ) -> anyhow::Result<HttpResponse> {
            self.requests
                .lock()
                .unwrap()
                .push(format!("POST {url} {body}"));
            Ok(HttpResponse {
                status: 200,
                body: "posted".to_string(),
                headers: HashMap::new(),
            })
        }

        async fn get(
            &self,
            url: &str,
            _headers: HashMap<String, String>,
        ) -> anyhow::Result<HttpResponse> {
            self.requests.lock().unwrap().push(format!("GET {url}"));
            Ok(HttpResponse {
                status: 200,
                body: "listed".to_string(),
                headers: HashMap::new(),
            })
        }
    }

    #[tokio::test]
    async fn fake_http_client_can_drive_provider_transport_port() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let client = RecordingHttpClient {
            requests: requests.clone(),
        };

        let post = client
            .post(
                "https://example.test/items",
                HashMap::new(),
                "body".to_string(),
            )
            .await
            .unwrap();
        let get = client
            .get("https://example.test/items", HashMap::new())
            .await
            .unwrap();

        assert_eq!(post.body, "posted");
        assert_eq!(get.body, "listed");
        assert_eq!(
            requests.lock().unwrap().as_slice(),
            [
                "POST https://example.test/items body".to_string(),
                "GET https://example.test/items".to_string()
            ]
        );
    }
}

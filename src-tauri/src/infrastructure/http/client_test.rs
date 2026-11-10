#[cfg(test)]
mod tests {
    use super::super::client::HttpClient;
    use super::super::reqwest_impl::ReqwestHttpClient;
    use mockito::Server;
    use std::collections::HashMap;
    use std::net::{Ipv4Addr, TcpListener};

    #[tokio::test]
    async fn test_post_request() {
        if skip_when_local_tcp_listener_unavailable("test_post_request") {
            return;
        }

        let mut server = Server::new_async().await;

        let mock = server
            .mock("POST", "/test")
            .match_header("content-type", "application/json")
            .match_header("authorization", "Bearer test-token")
            .match_body(r#"{"key":"value"}"#)
            .with_status(200)
            .with_header("x-custom", "test-header")
            .with_body(r#"{"success":true}"#)
            .create_async()
            .await;

        let client = ReqwestHttpClient::new();
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());
        headers.insert("authorization".to_string(), "Bearer test-token".to_string());

        let url = format!("{}/test", server.url());
        let response = client
            .post(&url, headers, r#"{"key":"value"}"#.to_string())
            .await
            .expect("POST request failed");

        mock.assert_async().await;
        assert_eq!(response.status, 200);
        assert_eq!(response.body, r#"{"success":true}"#);
        assert_eq!(response.headers.get("x-custom").unwrap(), "test-header");
    }

    #[tokio::test]
    async fn test_get_request() {
        if skip_when_local_tcp_listener_unavailable("test_get_request") {
            return;
        }

        let mut server = Server::new_async().await;

        let mock = server
            .mock("GET", "/data")
            .match_header("accept", "application/json")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":"test"}"#)
            .create_async()
            .await;

        let client = ReqwestHttpClient::new();
        let mut headers = HashMap::new();
        headers.insert("accept".to_string(), "application/json".to_string());

        let url = format!("{}/data", server.url());
        let response = client.get(&url, headers).await.expect("GET request failed");

        mock.assert_async().await;
        assert_eq!(response.status, 200);
        assert_eq!(response.body, r#"{"data":"test"}"#);
        assert_eq!(
            response.headers.get("content-type").unwrap(),
            "application/json"
        );
    }

    fn skip_when_local_tcp_listener_unavailable(test_name: &str) -> bool {
        match TcpListener::bind((Ipv4Addr::LOCALHOST, 0)) {
            Ok(listener) => {
                drop(listener);
                false
            }
            Err(err) => {
                eprintln!(
                    "skipping {test_name}: local TCP listener unavailable for mockito ({err})"
                );
                true
            }
        }
    }
}

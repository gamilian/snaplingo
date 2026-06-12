#[cfg(test)]
mod tests {
    use crate::translate::{TranslationProvider, GoogleTranslateProvider};

    #[tokio::test]
    async fn test_google_translate_success() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/translate_a/single")
            .match_query(mockito::Matcher::Any)
            .with_status(200)
            .with_body(r#"[[["Hello","你好",null,null,10]],null,"zh-CN"]"#)
            .create();

        let provider = GoogleTranslateProvider::new(server.url());
        let result = provider.translate("你好", "zh-CN", "en").await.unwrap();

        assert_eq!(result.text, "Hello");
        assert_eq!(result.provider_id, "google-translate");
        mock.assert();
    }
}

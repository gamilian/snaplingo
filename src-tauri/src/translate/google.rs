use super::provider::{TranslationProvider, TranslationResult};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;

pub struct GoogleTranslateProvider {
    client: Client,
    base_url: String,
}

impl GoogleTranslateProvider {
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
        }
    }

    pub fn default() -> Self {
        Self::new("https://translate.googleapis.com".to_string())
    }
}

#[async_trait]
impl TranslationProvider for GoogleTranslateProvider {
    fn id(&self) -> &str {
        "google-translate"
    }

    fn name(&self) -> &str {
        "Google Translate"
    }

    async fn translate(&self, text: &str, from: &str, to: &str) -> Result<TranslationResult> {
        let url = format!(
            "{}/translate_a/single?client=gtx&sl={}&tl={}&dt=t&q={}",
            self.base_url,
            from,
            to,
            urlencoding::encode(text)
        );

        let response = self.client.get(&url).send().await?;
        let json: Value = response.json().await?;

        let translated = json[0][0][0]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid response format"))?
            .to_string();

        Ok(TranslationResult {
            provider_id: self.id().to_string(),
            text: translated,
            detected_language: json[2].as_str().map(String::from),
        })
    }

    fn requires_api_key(&self) -> bool {
        false
    }
}

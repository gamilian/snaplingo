use std::sync::Arc;

use crate::http::HttpClient;

use snaplingo_core::providers::{
    LLMClient, LLMProtocol, LlmClientConfig, LlmModelLister, LlmRuntime,
};

use super::{AnthropicLLMClient, GeminiLLMClient, OpenAILLMClient};

pub struct ProviderLlmRuntime {
    http_client: Arc<dyn HttpClient>,
}

impl ProviderLlmRuntime {
    pub fn new(http_client: Arc<dyn HttpClient>) -> Self {
        Self { http_client }
    }
}

impl LlmRuntime for ProviderLlmRuntime {
    fn translation_client(&self, config: LlmClientConfig) -> Arc<dyn LLMClient> {
        match config.protocol {
            LLMProtocol::OpenAI => Arc::new(OpenAILLMClient::new_chat_completions(
                self.http_client.clone(),
                config.endpoint,
                config.model,
                config.api_key,
            )),
            LLMProtocol::OpenAIResponses => Arc::new(OpenAILLMClient::new_responses(
                self.http_client.clone(),
                config.endpoint,
                config.model,
                config.api_key,
            )),
            LLMProtocol::Anthropic => Arc::new(AnthropicLLMClient::new(
                self.http_client.clone(),
                config.endpoint,
                config.model,
                config.api_key,
            )),
            LLMProtocol::Gemini => Arc::new(GeminiLLMClient::new(
                self.http_client.clone(),
                config.endpoint,
                config.model,
                config.api_key,
            )),
        }
    }

    fn model_lister(&self, config: LlmClientConfig) -> Arc<dyn LlmModelLister> {
        match config.protocol {
            LLMProtocol::OpenAI | LLMProtocol::OpenAIResponses => {
                Arc::new(OpenAILLMClient::new_chat_completions(
                    self.http_client.clone(),
                    config.endpoint,
                    String::new(),
                    config.api_key,
                ))
            }
            LLMProtocol::Anthropic => Arc::new(AnthropicLLMClient::new(
                self.http_client.clone(),
                config.endpoint,
                String::new(),
                config.api_key,
            )),
            LLMProtocol::Gemini => Arc::new(GeminiLLMClient::new(
                self.http_client.clone(),
                config.endpoint,
                String::new(),
                config.api_key,
            )),
        }
    }
}

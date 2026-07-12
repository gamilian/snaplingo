use std::sync::Arc;

use crate::application::providers::{
    HttpClient, LLMClient, LLMProtocol, LlmClientConfig, LlmModelLister, LlmRuntime,
};

use super::{AnthropicLLMClient, GeminiLLMClient, OpenAILLMClient};

pub(crate) struct InfrastructureLlmRuntime {
    http_client: Arc<dyn HttpClient>,
}

impl InfrastructureLlmRuntime {
    pub(crate) fn new(http_client: Arc<dyn HttpClient>) -> Self {
        Self { http_client }
    }
}

impl LlmRuntime for InfrastructureLlmRuntime {
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

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// LLM 客户端统一接口.
#[async_trait]
pub trait LLMClient: Send + Sync {
    async fn generate(&self, request: &LLMRequest) -> Result<LLMResponse>;
}

/// Capability interface for enumerating models an LLM endpoint exposes.
#[async_trait]
pub trait LlmModelLister: Send + Sync {
    async fn list_models(&self) -> Result<Vec<ModelInfo>>;
}

/// Factory/runtime boundary for provider-specific LLM clients.
pub trait LlmRuntime: Send + Sync {
    fn translation_client(&self, config: LlmClientConfig) -> Arc<dyn LLMClient>;

    fn model_lister(&self, config: LlmClientConfig) -> Arc<dyn LlmModelLister>;
}

#[derive(Clone, Debug)]
pub struct LlmClientConfig {
    pub protocol: LLMProtocol,
    pub endpoint: String,
    pub model: String,
    pub api_key: String,
}

/// A single model exposed by an LLM endpoint, for provider introspection.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
}

/// 响应（极简：只返回文本）.
#[derive(Debug, Clone)]
pub struct LLMResponse {
    pub text: String,
}

/// 请求参数.
#[derive(Debug, Clone)]
pub struct LLMRequest {
    pub system_prompt: Option<String>,
    pub user_prompt: String,
    pub options: LLMOptions,
}

/// 通用选项.
#[derive(Debug, Clone)]
pub struct LLMOptions {
    pub reasoning: Option<ReasoningLevel>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

impl Default for LLMOptions {
    fn default() -> Self {
        Self {
            reasoning: None,
            temperature: Some(0.0),
            max_tokens: Some(8192),
        }
    }
}

/// 推理强度.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningLevel {
    Minimal,
    Low,
    Medium,
    High,
    XHigh,
}

/// LLM 协议类型.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LLMProtocol {
    #[serde(rename = "openai")]
    OpenAI,
    #[serde(rename = "openai-responses")]
    OpenAIResponses,
    #[serde(rename = "anthropic")]
    Anthropic,
    #[serde(rename = "gemini")]
    Gemini,
}

impl LLMProtocol {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::OpenAI => "openai",
            Self::OpenAIResponses => "openai-responses",
            Self::Anthropic => "anthropic",
            Self::Gemini => "gemini",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeLlmClient;

    #[async_trait]
    impl LLMClient for FakeLlmClient {
        async fn generate(&self, request: &LLMRequest) -> Result<LLMResponse> {
            Ok(LLMResponse {
                text: format!("translated: {}", request.user_prompt),
            })
        }
    }

    struct FakeModelLister;

    #[async_trait]
    impl LlmModelLister for FakeModelLister {
        async fn list_models(&self) -> Result<Vec<ModelInfo>> {
            Ok(vec![ModelInfo {
                id: "test-model".to_string(),
            }])
        }
    }

    struct FakeLlmRuntime;

    impl LlmRuntime for FakeLlmRuntime {
        fn translation_client(&self, _config: LlmClientConfig) -> Arc<dyn LLMClient> {
            Arc::new(FakeLlmClient)
        }

        fn model_lister(&self, _config: LlmClientConfig) -> Arc<dyn LlmModelLister> {
            Arc::new(FakeModelLister)
        }
    }

    fn config() -> LlmClientConfig {
        LlmClientConfig {
            protocol: LLMProtocol::OpenAI,
            endpoint: "https://example.test/v1".to_string(),
            model: "test-model".to_string(),
            api_key: "test-key".to_string(),
        }
    }

    #[tokio::test]
    async fn fake_llm_runtime_can_drive_generation_and_listing_ports() {
        let runtime = FakeLlmRuntime;
        let client = runtime.translation_client(config());
        let response = client
            .generate(&LLMRequest {
                system_prompt: None,
                user_prompt: "hello".to_string(),
                options: LLMOptions::default(),
            })
            .await
            .unwrap();

        let models = runtime.model_lister(config()).list_models().await.unwrap();

        assert_eq!(response.text, "translated: hello");
        assert_eq!(models[0].id, "test-model");
    }
}

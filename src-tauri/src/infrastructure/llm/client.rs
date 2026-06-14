use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// LLM 客户端统一接口
#[async_trait]
pub trait LLMClient: Send + Sync {
    /// 生成文本
    async fn generate(&self, request: &LLMRequest) -> Result<LLMResponse>;
}

/// 响应（极简：只返回文本）
#[derive(Debug, Clone)]
pub struct LLMResponse {
    pub text: String,
}

/// 请求参数
#[derive(Debug, Clone)]
pub struct LLMRequest {
    pub system_prompt: Option<String>,
    pub user_prompt: String,
    pub options: LLMOptions,
}

/// 通用选项
#[derive(Debug, Clone)]
pub struct LLMOptions {
    /// Reasoning 强度
    pub reasoning: Option<ReasoningLevel>,

    /// 温度（0.0 = 确定性，1.0 = 创造性）
    pub temperature: Option<f32>,

    /// 最大生成 token 数
    pub max_tokens: Option<u32>,
}

impl Default for LLMOptions {
    fn default() -> Self {
        Self {
            reasoning: None,           // 默认不思考（快速模式）
            temperature: Some(0.0),    // 确定性翻译
            max_tokens: Some(8192),    // 足够长文档翻译
        }
    }
}

/// 推理强度（参考 Pi 的 ThinkingLevel）
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningLevel {
    Minimal,  // 快速，低成本
    Low,
    Medium,
    High,
    XHigh,    // 极高（o3-mini high 等）
}

/// LLM 协议类型
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LLMProtocol {
    #[serde(rename = "openai")]
    OpenAI,
    #[serde(rename = "anthropic")]
    Anthropic,
    #[serde(rename = "gemini")]
    Gemini,
}

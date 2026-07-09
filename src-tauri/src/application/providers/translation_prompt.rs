use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::domain::translation::TranslationRequest;
use crate::infrastructure::storage::ConfigFile;

pub const SMART_PROMPT_STRATEGY_ID: &str = "smart";
pub const DEFAULT_PROMPT_STRATEGY_ID: &str = "general";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TranslationPromptStrategy {
    pub id: String,
    pub name: String,
    pub description: String,
    pub system_prompt: String,
    pub is_builtin: bool,
    pub is_deletable: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TranslationPromptStrategyConfig {
    pub strategies: Vec<TranslationPromptStrategy>,
}

pub struct TranslationPromptConfiguration {
    config_file: Arc<ConfigFile>,
}

impl TranslationPromptConfiguration {
    pub fn new(config_file: Arc<ConfigFile>) -> Self {
        Self { config_file }
    }

    pub fn list(&self) -> TranslationPromptStrategyConfig {
        let stored = self
            .config_file
            .load::<TranslationPromptStrategyConfig>("translation_prompt_strategies")
            .ok();
        merge_prompt_strategy_config(stored)
    }

    pub fn save(
        &self,
        config: TranslationPromptStrategyConfig,
    ) -> crate::Result<TranslationPromptStrategyConfig> {
        validate_prompt_strategy_config(&config)?;
        let config = sanitize_prompt_strategy_config(config);
        self.config_file
            .save("translation_prompt_strategies", &config)
            .map_err(|e| format!("Failed to save prompt strategies: {}", e))?;
        Ok(config)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderPromptStrategy {
    pub strategy_id: String,
    pub fallback_strategy_id: String,
}

impl Default for ProviderPromptStrategy {
    fn default() -> Self {
        Self {
            strategy_id: SMART_PROMPT_STRATEGY_ID.to_string(),
            fallback_strategy_id: DEFAULT_PROMPT_STRATEGY_ID.to_string(),
        }
    }
}

pub fn default_prompt_strategy_config() -> TranslationPromptStrategyConfig {
    TranslationPromptStrategyConfig {
        strategies: vec![
            builtin_strategy(
                DEFAULT_PROMPT_STRATEGY_ID,
                "通用翻译",
                "适合大多数普通文本。",
                "You are a professional translation engine. Translate the user's text from {source_lang} to {target_lang}. Preserve meaning, tone, formatting, names, numbers, and URLs. Return only the translation.",
            ),
            builtin_strategy(
                "technical",
                "技术文档",
                "适合代码、API、软件工程、产品文档。",
                "You are a technical translator. Translate the user's text from {source_lang} to {target_lang}. Preserve code, commands, API names, file paths, placeholders, Markdown, and technical terms. Return only the translation.",
            ),
            builtin_strategy(
                "academic",
                "论文/学术",
                "适合论文、摘要、研究报告和正式论述。",
                "You are an academic translator. Translate the user's text from {source_lang} to {target_lang} in a precise, formal academic style. Preserve citations, terminology, formulas, and paragraph structure. Return only the translation.",
            ),
            builtin_strategy(
                "casual",
                "社交/口语",
                "适合聊天、评论、邮件和自然口语。",
                "You are a natural conversational translator. Translate the user's text from {source_lang} to {target_lang} in fluent everyday language. Keep intent, emotion, and politeness natural. Return only the translation.",
            ),
            builtin_strategy(
                "mixed-zh-en",
                "中英夹杂",
                "适合保留部分英文术语的中英混排表达。",
                "You are a bilingual Chinese-English translation expert. Translate the user's text from {source_lang} to {target_lang}, keeping widely used English technical terms where they improve readability. Produce a natural mixed Chinese-English style when appropriate. Return only the translation.",
            ),
        ],
    }
}

pub fn merge_prompt_strategy_config(
    config: Option<TranslationPromptStrategyConfig>,
) -> TranslationPromptStrategyConfig {
    let mut merged = default_prompt_strategy_config();
    let Some(config) = config else {
        return merged;
    };

    for strategy in config.strategies {
        if let Some(existing) = merged
            .strategies
            .iter_mut()
            .find(|item| item.id == strategy.id)
        {
            existing.name = strategy.name;
            existing.description = strategy.description;
            existing.system_prompt = strategy.system_prompt;
        } else {
            merged.strategies.push(strategy);
        }
    }

    merged
}

pub fn sanitize_prompt_strategy_config(
    config: TranslationPromptStrategyConfig,
) -> TranslationPromptStrategyConfig {
    let defaults = default_prompt_strategy_config();
    let mut sanitized = Vec::new();

    for default_strategy in defaults.strategies {
        let override_strategy = config
            .strategies
            .iter()
            .find(|strategy| strategy.id == default_strategy.id);
        sanitized.push(TranslationPromptStrategy {
            id: default_strategy.id,
            name: override_strategy
                .map(|strategy| strategy.name.clone())
                .unwrap_or(default_strategy.name),
            description: override_strategy
                .map(|strategy| strategy.description.clone())
                .unwrap_or(default_strategy.description),
            system_prompt: override_strategy
                .map(|strategy| strategy.system_prompt.clone())
                .unwrap_or(default_strategy.system_prompt),
            is_builtin: true,
            is_deletable: false,
        });
    }

    let builtin_ids: Vec<_> = sanitized.iter().map(|item| item.id.clone()).collect();
    for strategy in config
        .strategies
        .into_iter()
        .filter(|strategy| !builtin_ids.contains(&strategy.id))
    {
        sanitized.push(TranslationPromptStrategy {
            is_builtin: false,
            is_deletable: true,
            ..strategy
        });
    }

    TranslationPromptStrategyConfig {
        strategies: sanitized,
    }
}

pub fn render_translation_system_prompt(
    strategies: &[TranslationPromptStrategy],
    provider_strategy: &ProviderPromptStrategy,
    request: &TranslationRequest,
) -> String {
    let fallback_strategy_id = DEFAULT_PROMPT_STRATEGY_ID;
    let selected_id = if provider_strategy.strategy_id == SMART_PROMPT_STRATEGY_ID {
        select_prompt_strategy_id(strategies, request, fallback_strategy_id)
    } else {
        provider_strategy.strategy_id.as_str()
    };

    let strategy = find_strategy(strategies, selected_id)
        .or_else(|| find_strategy(strategies, fallback_strategy_id))
        .or_else(|| find_strategy(strategies, DEFAULT_PROMPT_STRATEGY_ID));

    strategy
        .map(|item| render_prompt_template(&item.system_prompt, request))
        .unwrap_or_else(|| {
            render_prompt_template(
                "You are a professional translation engine. Translate the user's text from {source_lang} to {target_lang}. Return only the translation.",
                request,
            )
        })
}

pub fn select_prompt_strategy_id<'a>(
    strategies: &'a [TranslationPromptStrategy],
    request: &TranslationRequest,
    fallback_strategy_id: &'a str,
) -> &'a str {
    let text = request.text.to_lowercase();

    if text.contains("```")
        || text.contains("api")
        || text.contains("http")
        || text.contains("json")
        || text.contains("typescript")
        || text.contains("rust")
        || text.contains("function")
    {
        return strategy_or_fallback(strategies, "technical", fallback_strategy_id);
    }

    if text.contains("abstract")
        || text.contains("doi:")
        || text.contains("et al")
        || text.contains("methodology")
        || text.contains("references")
    {
        return strategy_or_fallback(strategies, "academic", fallback_strategy_id);
    }

    if text.len() < 160
        && (text.contains("lol")
            || text.contains("hey")
            || text.contains("thanks")
            || text.contains("哈哈")
            || text.contains("谢谢"))
    {
        return strategy_or_fallback(strategies, "casual", fallback_strategy_id);
    }

    if contains_chinese(&request.text) && contains_ascii_word(&request.text) {
        return strategy_or_fallback(strategies, "mixed-zh-en", fallback_strategy_id);
    }

    strategy_or_fallback(strategies, fallback_strategy_id, DEFAULT_PROMPT_STRATEGY_ID)
}

pub fn validate_prompt_strategy_config(
    config: &TranslationPromptStrategyConfig,
) -> crate::Result<()> {
    if config
        .strategies
        .iter()
        .all(|strategy| strategy.id != DEFAULT_PROMPT_STRATEGY_ID)
    {
        return Err(crate::AppError::Other(
            "Default prompt strategy cannot be removed".into(),
        ));
    }

    for strategy in &config.strategies {
        if strategy.id.trim().is_empty() {
            return Err(crate::AppError::Other("Strategy id cannot be empty".into()));
        }
        if strategy.name.trim().is_empty() {
            return Err(crate::AppError::Other(
                "Strategy name cannot be empty".into(),
            ));
        }
        if strategy.system_prompt.trim().is_empty() {
            return Err(crate::AppError::Other(
                "Strategy system prompt cannot be empty".into(),
            ));
        }
    }

    Ok(())
}

fn builtin_strategy(
    id: &str,
    name: &str,
    description: &str,
    system_prompt: &str,
) -> TranslationPromptStrategy {
    TranslationPromptStrategy {
        id: id.to_string(),
        name: name.to_string(),
        description: description.to_string(),
        system_prompt: system_prompt.to_string(),
        is_builtin: true,
        is_deletable: false,
    }
}

fn render_prompt_template(template: &str, request: &TranslationRequest) -> String {
    template
        .replace("{source_lang}", &request.source_lang)
        .replace("{target_lang}", &request.target_lang)
}

fn find_strategy<'a>(
    strategies: &'a [TranslationPromptStrategy],
    id: &str,
) -> Option<&'a TranslationPromptStrategy> {
    strategies.iter().find(|item| item.id == id)
}

fn strategy_or_fallback<'a>(
    strategies: &'a [TranslationPromptStrategy],
    preferred: &'a str,
    fallback: &'a str,
) -> &'a str {
    if strategies.iter().any(|item| item.id == preferred) {
        preferred
    } else {
        fallback
    }
}

fn contains_chinese(text: &str) -> bool {
    text.chars()
        .any(|ch| ('\u{4e00}'..='\u{9fff}').contains(&ch))
}

fn contains_ascii_word(text: &str) -> bool {
    text.split(|ch: char| !ch.is_ascii_alphanumeric())
        .any(|word| word.len() >= 2 && word.chars().any(|ch| ch.is_ascii_alphabetic()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(text: &str) -> TranslationRequest {
        TranslationRequest {
            text: text.to_string(),
            source_lang: "auto".to_string(),
            target_lang: "zh-CN".to_string(),
        }
    }

    #[test]
    fn default_config_includes_editable_general_strategy() {
        let config = default_prompt_strategy_config();
        let general = config
            .strategies
            .iter()
            .find(|item| item.id == DEFAULT_PROMPT_STRATEGY_ID)
            .unwrap();

        assert_eq!(general.name, "通用翻译");
        assert!(!general.is_deletable);
        assert!(general.system_prompt.contains("{target_lang}"));
    }

    #[test]
    fn smart_strategy_selects_technical_prompt_for_code_like_text() {
        let config = default_prompt_strategy_config();

        let selected = select_prompt_strategy_id(
            &config.strategies,
            &request("Call the REST API with this JSON payload."),
            DEFAULT_PROMPT_STRATEGY_ID,
        );

        assert_eq!(selected, "technical");
    }

    #[test]
    fn render_prompt_uses_edited_general_strategy_for_fallback() {
        let config = TranslationPromptStrategyConfig {
            strategies: vec![TranslationPromptStrategy {
                id: DEFAULT_PROMPT_STRATEGY_ID.to_string(),
                name: "通用翻译".to_string(),
                description: "".to_string(),
                system_prompt: "Translate into {target_lang} with my custom rule.".to_string(),
                is_builtin: true,
                is_deletable: false,
            }],
        };

        let prompt = render_translation_system_prompt(
            &config.strategies,
            &ProviderPromptStrategy::default(),
            &request("Hello world"),
        );

        assert_eq!(prompt, "Translate into zh-CN with my custom rule.");
    }

    #[test]
    fn smart_strategy_falls_back_to_general_even_with_legacy_custom_fallback() {
        let config = default_prompt_strategy_config();

        let prompt = render_translation_system_prompt(
            &config.strategies,
            &ProviderPromptStrategy {
                strategy_id: SMART_PROMPT_STRATEGY_ID.to_string(),
                fallback_strategy_id: "casual".to_string(),
            },
            &request("Plain text without a strong category signal."),
        );

        let general_prompt = config
            .strategies
            .iter()
            .find(|strategy| strategy.id == DEFAULT_PROMPT_STRATEGY_ID)
            .unwrap();
        assert_eq!(
            prompt,
            render_prompt_template(
                &general_prompt.system_prompt,
                &request("Plain text without a strong category signal.")
            )
        );
    }

    #[test]
    fn validates_that_general_strategy_cannot_be_removed() {
        let result = validate_prompt_strategy_config(&TranslationPromptStrategyConfig {
            strategies: vec![],
        });

        assert_eq!(
            result.unwrap_err().to_string(),
            "Default prompt strategy cannot be removed"
        );
    }
}
